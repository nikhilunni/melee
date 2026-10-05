//! Menu models as retail lays them out: a joint tree with its parallel
//! material-animation tree, joints numbered the way `lb_80011E24` counts
//! them, and the texture each animated TObj shows at a given frame.
use hsd_anim::aobj::{AObj, AObjEndCallback};
use hsd_archive::{
    desc::{material_animation::MaterialAnimation, JObjDesc, MatAnimJoint},
    visual::TextureDescriptor,
    Archive,
};

/// A model descriptor's joint tree and material animation (the `joint` and
/// `matanim_joint` of a `StaticModelDesc`, or one of a `DynamicModelDesc`'s
/// material animations).
pub(crate) struct Model {
    joint: JObjDesc,
    matanim: Option<MatAnimJoint>,
}

/// One texture a joint's display objects draw: GX image and palette offsets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Texture {
    pub image: u32,
    pub palette: Option<u32>,
    /// Whether a texture animation chose this image (rather than the TObj's
    /// own default).
    pub animated: bool,
}

fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}

impl Model {
    /// `StaticModelDesc` at `at`: joint +0x0, animjoint +0x4, matanim +0x8,
    /// shapeanim +0xC (sc/types.h).
    pub fn static_at(archive: &Archive, at: u32) -> Result<Self, String> {
        let joint = archive
            .link(at)
            .map_err(error)?
            .ok_or_else(|| format!("model at {at:#x} has no joint"))?;
        let matanim = archive.link(at + 8).map_err(error)?;
        Self::new(archive, joint, matanim)
    }

    /// `DynamicModelDesc` at `at` with its `index`th material animation:
    /// joint +0x0, anims +0x4, matanims +0x8, shapeanims +0xC (sc/types.h).
    pub fn dynamic_at(archive: &Archive, at: u32, index: u32) -> Result<Self, String> {
        let joint = archive
            .link(at)
            .map_err(error)?
            .ok_or_else(|| format!("model at {at:#x} has no joint"))?;
        let matanim = match archive.link(at + 8).map_err(error)? {
            Some(list) => archive.link(list + 4 * index).map_err(error)?,
            None => None,
        };
        Self::new(archive, joint, matanim)
    }

    fn new(archive: &Archive, joint: u32, matanim: Option<u32>) -> Result<Self, String> {
        Ok(Self {
            joint: JObjDesc::read(archive, joint).map_err(error)?,
            matanim: matanim
                .map(|at| MatAnimJoint::read(archive, at))
                .transpose()
                .map_err(error)?,
        })
    }

    /// Joints in `lb_80011E24` order (lbspdisplay.c): pre-order, children
    /// before siblings, an instance's children skipped; each with the
    /// material-animation node `HSD_JObjAddAnimAll` pairs with it.
    fn joints(&self) -> Vec<(&JObjDesc, Option<&MatAnimJoint>)> {
        let mut out = Vec::new();
        let mut stack = vec![(&self.joint, self.matanim.as_ref())];
        while let Some((joint, anim)) = stack.pop() {
            out.push((joint, anim));
            // Siblings come after this joint's whole sub-tree.
            if let Some(next) = joint.next.as_deref() {
                stack.push((next, anim.and_then(|a| a.next.as_deref())));
            }
            if let Some(child) = joint.child.as_deref() {
                stack.push((child, anim.and_then(|a| a.child.as_deref())));
            }
        }
        out
    }

    /// Every TObj of joint `index`, in display-object then texture order,
    /// with the image its texture animation shows at `frame`
    /// (`HSD_TObjReqAnim` then one `TObjUpdateFunc` pass, tobj.c).
    pub fn textures(
        &self,
        archive: &Archive,
        index: usize,
        frame: f32,
    ) -> Result<Vec<Texture>, String> {
        let joints = self.joints();
        let (joint, anim) = joints
            .get(index)
            .ok_or_else(|| format!("the model has no joint {index}"))?;
        let records = match anim.and_then(|a| a.matanim) {
            Some(head) => MaterialAnimation::read_chain(archive, Some(head)).map_err(error)?,
            None => Vec::new(),
        };
        let mut out = Vec::new();
        let Some(dobj) = joint.u.dobj() else {
            return Ok(out);
        };
        for (i, dobj) in dobj.siblings().enumerate() {
            let Some(mobj) = dobj.mobj.as_deref() else {
                continue;
            };
            let descriptors = TextureDescriptor::read_chain(archive, mobj.texdesc).map_err(error)?;
            for descriptor in descriptors {
                let texanim = records
                    .get(i)
                    .and_then(|r| r.textures.iter().find(|t| t.id == descriptor.id));
                let mut texture = Texture {
                    image: descriptor.image,
                    palette: descriptor.palette,
                    animated: false,
                };
                if let Some(texanim) = texanim {
                    if let Some(animation) = &texanim.animation {
                        let animation = hsd_anim::load::animation_object(animation)
                            .map_err(|e| format!("{e:?}"))?;
                        let mut aobj = AObj::load_desc(&animation);
                        aobj.req_anim(frame);
                        let mut end = AObjEndCallback::default();
                        aobj.interpret_anim(
                            &mut |track, value| {
                                // TObjUpdateFunc (tobj.c): TIMG and TCLT
                                // select table entries by fctiwz(value).
                                let pick = |table: &[Option<u32>]| {
                                    usize::try_from(gekko_math::msl::fctiwz(value))
                                        .ok()
                                        .and_then(|i| table.get(i).copied().flatten())
                                };
                                match track {
                                    TRACK_IMAGE => {
                                        if let Some(image) = pick(&texanim.images) {
                                            texture.image = image;
                                            texture.animated = true;
                                        }
                                    }
                                    TRACK_PALETTE => {
                                        if let Some(palette) = pick(&texanim.palettes) {
                                            texture.palette = Some(palette);
                                        }
                                    }
                                    _ => {}
                                }
                            },
                            &mut end,
                        );
                    }
                }
                out.push(texture);
            }
        }
        Ok(out)
    }
}

/// `HSD_A_T_TIMG`: the texture animation's image-table index.
const TRACK_IMAGE: u8 = 1;
/// `HSD_A_T_TCLT`: the texture animation's palette-table index.
const TRACK_PALETTE: u8 = 10;
