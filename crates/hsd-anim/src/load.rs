//! Archive-to-runtime glue for `HSD_JObjLoadJoint` and `HSD_JObjAddAnimAll`
//! (`src/sysdolphin/baselib/jobj.c`). Binary layouts stay in `hsd-archive`.
//!
//! This is the headless, default-class loader. Polygon and texture rendering
//! remain deferred as in `dobj`/`mobj`. Constraints, instance references, cubic spline
//! and particle joints, custom classes, and external AObj references are rejected
//! because the runtime cannot represent their behavior. Pixel-engine descriptors
//! are rendering-only state and are ignored here.

use std::collections::HashSet;
use std::fmt;

use hsd_archive::desc::DescError;
use hsd_archive::{desc, Archive};
use hsd_types::{Mtx, Vec3};

use crate::aobj::AObjDesc;
use crate::dobj::DObj;
use crate::fobj::FObjDesc;
use crate::jobj::{
    AnimJoint, JObjId, JObjTree, JointSpec, JOBJ_INSTANCE, JOBJ_JOINT, JOBJ_PTCL, JOBJ_SPLINE,
};
use crate::mobj::{GxColor, MObj, Material, RENDER_BLENDING, RENDER_NO_ZUPDATE};

/// Failures in `HSD_JObjLoadJoint` / `HSD_JObjAddAnimAll` (`jobj.c`).
#[derive(Debug)]
pub enum LoadError {
    Archive(DescError),
    MismatchedTreeShape {
        joint: Option<JObjId>,
        animation_offset: Option<u32>,
    },
    InvalidJoint(JObjId),
    RepeatedJoint(JObjId),
    UnsupportedFlag {
        descriptor_offset: u32,
        flags: u32,
    },
    UnsupportedFeature {
        descriptor_offset: u32,
        feature: &'static str,
    },
    MissingMaterial {
        descriptor_offset: u32,
    },
}

impl fmt::Display for LoadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Archive(error) => write!(f, "archive descriptor read failed: {error}"),
            Self::MismatchedTreeShape {
                joint,
                animation_offset,
            } => write!(
                f,
                "joint/animation tree shape mismatch: joint {joint:?}, animation {animation_offset:?}"
            ),
            Self::InvalidJoint(id) => write!(f, "joint {id:?} is outside the runtime tree"),
            Self::RepeatedJoint(id) => {
                write!(f, "joint {id:?} occurs more than once in the runtime tree")
            }
            Self::UnsupportedFlag {
                descriptor_offset,
                flags,
            } => write!(
                f,
                "descriptor {descriptor_offset:#x} has unsupported flags {flags:#x}"
            ),
            Self::UnsupportedFeature {
                descriptor_offset,
                feature,
            } => write!(
                f,
                "descriptor {descriptor_offset:#x} requires unsupported {feature}"
            ),
            Self::MissingMaterial { descriptor_offset } => write!(
                f,
                "material descriptor {descriptor_offset:#x} has no HSD_Material"
            ),
        }
    }
}

impl std::error::Error for LoadError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Archive(error) => Some(error),
            _ => None,
        }
    }
}

impl From<DescError> for LoadError {
    fn from(error: DescError) -> Self {
        Self::Archive(error)
    }
}

/// `HSD_JObjLoadJoint` / `JObjLoad` (`jobj.c:610-672`, retail `0x80370E44`
/// / `0x80370BEC`): load the root, its children, and its next siblings.
/// Allocation is depth-first, children before siblings, including root siblings.
/// The descriptor already owns all supported data; `archive` is reserved for
/// future typed readers of the remaining reference payloads.
pub fn load_joint_tree(
    archive: &Archive,
    root: &desc::JObjDesc,
) -> Result<(JObjTree, JObjId), LoadError> {
    let mut tree = JObjTree::new();
    let mut first = None;
    let mut previous = None;
    for descriptor in root.siblings() {
        let spec = joint_spec(archive, descriptor)?;
        let id = tree.load_joint(&spec);
        if let Some(previous) = previous {
            tree.get_mut(previous).next = Some(id);
        }
        first.get_or_insert(id);
        previous = Some(id);
    }
    for descriptor in root.siblings().flat_map(|j| j.descendants()) {
        if let desc::JObjUnion::Spline(Some(offset)) = descriptor.u {
            let spline = desc::spline::LinearSpline::read(archive, offset)?;
            let id = (0..tree.len())
                .map(JObjId)
                .find(|&id| tree.get(id).id == descriptor.offset)
                .unwrap();
            tree.get_mut(id).spline = Some(spline);
        }
    }
    Ok((tree, first.expect("root.siblings() includes root")))
}

/// `JObjLoad` (`jobj.c:629-665`): build the owned inputs to the runtime loader.
fn joint_spec(archive: &Archive, joint: &desc::JObjDesc) -> Result<JointSpec, LoadError> {
    check_class(joint.class_name.as_deref(), "hsd_jobj", joint.offset)?;
    check_joint_flags(
        joint.flags
            & if matches!(joint.u, desc::JObjUnion::Spline(Some(_))) {
                !JOBJ_SPLINE
            } else {
                u32::MAX
            },
        joint.offset,
    )?;
    if joint.robjdesc.is_some() {
        return Err(unsupported(joint.offset, "RObj constraints"));
    }
    let mut spec = joint_transforms(joint);
    spec.children = joint
        .children()
        .map(|child| joint_spec(archive, child))
        .collect::<Result<_, _>>()?;
    spec.dobj = load_dobj_chain(archive, joint.u.dobj())?;
    Ok(spec)
}

/// `JObjLoad` (`jobj.c:650-663`): copy SRT, envelope matrix, flags, and id.
/// There is no Euler-to-quaternion conversion, even under USE_QUATERNION;
/// `JObjInit` leaves rotate.w zero and `JObjLoad` writes only x/y/z.
/// Confirmed in retail at `0x80370DA0-0x80370DE4`: loads/stores only.
fn joint_transforms(joint: &desc::JObjDesc) -> JointSpec {
    JointSpec {
        flags: joint.flags,
        rotation: vector(joint.rotation),
        scale: vector(joint.scale),
        position: vector(joint.position),
        mtx: joint.mtx.map(Mtx),
        id: joint.offset,
        ..JointSpec::new()
    }
}

/// `JObjLoad` (`jobj.c:651-655`): copy vector components without arithmetic.
fn vector(value: desc::Vec3) -> Vec3 {
    Vec3::new(value.x, value.y, value.z)
}

/// `HSD_DObjLoadDesc` / `DObjLoad` (`dobj.c:178-230`): preserve chain order.
fn load_dobj_chain(
    archive: &Archive,
    head: Option<&desc::DObjDesc>,
) -> Result<Vec<DObj>, LoadError> {
    let Some(head) = head else {
        return Ok(Vec::new());
    };
    head.siblings()
        .map(|descriptor| {
            check_class(
                descriptor.class_name.as_deref(),
                "hsd_dobj",
                descriptor.offset,
            )?;
            let material = descriptor
                .mobj
                .as_deref()
                .map(|material| load_material(archive, material))
                .transpose()?;
            Ok(DObj::load(material))
        })
        .collect()
}

/// `HSD_MObjLoadDesc` / `MObjLoad` (`mobj.c:152-193`): material copy and TOON bit.
fn load_material(archive: &Archive, descriptor: &desc::MObjDesc) -> Result<MObj, LoadError> {
    check_class(
        descriptor.class_name.as_deref(),
        "hsd_mobj",
        descriptor.offset,
    )?;
    if descriptor.rendermode & RENDER_BLENDING == RENDER_NO_ZUPDATE {
        return Err(LoadError::UnsupportedFlag {
            descriptor_offset: descriptor.offset,
            flags: RENDER_NO_ZUPDATE,
        });
    }
    // `desc->pedesc` (mobj.c:159) is copied into a pixel-engine block that only
    // rendering reads. Headless simulation ignores it (Milestone 8).
    let material = descriptor.mat.ok_or(LoadError::MissingMaterial {
        descriptor_offset: descriptor.offset,
    })?;
    let mut result = MObj::load(
        descriptor.rendermode,
        Material {
            ambient: color(material.ambient),
            diffuse: color(material.diffuse),
            specular: color(material.specular),
            alpha: material.alpha,
            shininess: material.shininess,
        },
        None,
    );
    result.textures =
        hsd_archive::visual::TextureDescriptor::read_chain(archive, descriptor.texdesc)
            .map_err(|_| unsupported(descriptor.offset, "texture descriptor"))?
            .into_iter()
            .map(crate::tobj::TObj::load)
            .collect();
    Ok(result)
}

/// `MObjLoad` (`mobj.c:157`): copy GXColor channels verbatim.
fn color(value: desc::GxColor) -> GxColor {
    GxColor::new(value.r, value.g, value.b, value.a)
}

/// `HSD_JObjAddAnimAll` (`jobj.c:325-349`, retail `0x8036FB5C`) with null
/// material/shape arguments. AnimJoint has no material-animation payload:
/// those arrive through a separate MatAnimJoint in C. Existing material AObjs
/// remain attached, matching `HSD_MObjAddAnim(mobj, NULL)` (`mobj.c:55-68`).
///
/// Only this root's subtree is visited; root.next and anim.next are ignored,
/// as in C. Unlike C's permissive short-chain handling, this archive boundary
/// requires matching child/sibling shapes. Validation and conversion finish
/// before mutation, so errors leave the tree unchanged. Call `req_anim_all`
/// separately to request a frame (`HSD_JObjReqAnimAll`, `jobj.c:269-272`).
pub fn attach_anim_joint(
    tree: &mut JObjTree,
    root: JObjId,
    anim: &desc::AnimJoint,
    _archive: &Archive,
) -> Result<(), LoadError> {
    let attachments = prepare_animation(tree, root, anim)?;
    let attachments = attachments
        .into_iter()
        .map(|(id, animation)| {
            let reference = animation.aobjdesc.as_ref().map_or(0, |a| a.obj_id);
            let target = if reference != 0 {
                Some(
                    (0..tree.len())
                        .map(JObjId)
                        .find(|&target| {
                            tree.get(target).id == reference && tree.get(target).spline.is_some()
                        })
                        .ok_or_else(|| {
                            unsupported(reference, "external/non-spline AObj reference")
                        })?,
                )
            } else {
                None
            };
            Ok((id, animation, target))
        })
        .collect::<Result<Vec<_>, LoadError>>()?;
    for (id, animation, target) in attachments {
        tree.add_anim(id, Some(&animation), None);
        tree.get_mut(id).path_reference = target;
    }
    Ok(())
}

/// `HSD_JObjAddAnimAll` (`jobj.c:335-346`): pair child/next links before attaching.
fn prepare_animation(
    tree: &JObjTree,
    root: JObjId,
    anim: &desc::AnimJoint,
) -> Result<Vec<(JObjId, AnimJoint)>, LoadError> {
    let mut pending = vec![(Some(root), Some(anim), false)];
    let mut visited = HashSet::new();
    let mut attachments = Vec::new();
    while let Some((id, animation, follow_next)) = pending.pop() {
        let (id, animation) = match (id, animation) {
            (None, None) => continue,
            (Some(id), Some(animation)) => (id, animation),
            _ => {
                return Err(LoadError::MismatchedTreeShape {
                    joint: id,
                    animation_offset: animation.map(|a| a.offset),
                });
            }
        };
        if id.0 >= tree.len() {
            return Err(LoadError::InvalidJoint(id));
        }
        if !visited.insert(id) {
            return Err(LoadError::RepeatedJoint(id));
        }
        let joint = tree.get(id);
        check_joint_flags(
            joint.flags
                & if joint.spline.is_some() {
                    !JOBJ_SPLINE
                } else {
                    u32::MAX
                },
            joint.id,
        )?;
        attachments.push((id, animation_node(animation)?));
        if follow_next {
            pending.push((joint.next, animation.next.as_deref(), true));
        }
        pending.push((joint.child, animation.child.as_deref(), true));
    }
    Ok(attachments)
}

/// `HSD_JObjAddAnim` (`jobj.c:303-313`): prepare AObj replacement and scale flags.
fn animation_node(anim: &desc::AnimJoint) -> Result<AnimJoint, LoadError> {
    if anim.robj_anim.is_some() {
        return Err(unsupported(anim.offset, "RObj animation"));
    }
    Ok(AnimJoint {
        aobjdesc: anim.aobjdesc.as_deref().map(animation_object).transpose()?,
        flags: anim.flags,
        children: Vec::new(),
    })
}

/// `HSD_AObjLoadDesc` (`aobj.c:179-218`): pass flags and tracks to AObj's loader.
pub fn animation_object(anim: &desc::AObjDesc) -> Result<AObjDesc, LoadError> {
    if (anim.obj_id != 0 && !anim.obj_id_is_link) || (anim.obj_id == 0 && anim.obj_id_is_link) {
        return Err(unsupported(anim.offset, "AObj object reference"));
    }
    Ok(AObjDesc {
        flags: anim.flags,
        end_frame: anim.end_frame,
        fobjdesc: anim.tracks().map(animation_track).collect(),
        obj_id: anim.obj_id,
    })
}

/// `HSD_FObjLoadDesc` (`fobj.c`), called by `HSD_AObjLoadDesc` (`aobj.c:196`).
/// Preserve encoded bytes; the existing FObj loader performs startframe conversion.
fn animation_track(track: &desc::FObjDesc) -> FObjDesc {
    FObjDesc {
        length: track.length,
        startframe: track.startframe,
        obj_type: track.type_,
        frac_value: track.frac_value,
        frac_slope: track.frac_slope,
        ad: track.ad.clone(),
    }
}

/// `JObjLoad` / `HSD_JObjSetupMatrixSub` (`jobj.c`): reject deferred joint kinds.
fn check_joint_flags(flags: u32, offset: u32) -> Result<(), LoadError> {
    let unsupported = flags & (JOBJ_INSTANCE | JOBJ_SPLINE | JOBJ_PTCL | JOBJ_JOINT);
    if unsupported != 0 {
        return Err(LoadError::UnsupportedFlag {
            descriptor_offset: offset,
            flags: unsupported,
        });
    }
    Ok(())
}

/// `JObjLoadJointSub` / `HSD_DObjLoadDesc` / `HSD_MObjLoadDesc` (jobj/dobj/mobj.c):
/// only the default HSD classes have runtime implementations here.
fn check_class(name: Option<&str>, default: &str, offset: u32) -> Result<(), LoadError> {
    if name.is_some_and(|name| name != default) {
        return Err(unsupported(offset, "custom class dispatch"));
    }
    Ok(())
}

/// Deferred payload in `JObjLoad` / `HSD_JObjAddAnimAll` (`jobj.c`).
fn unsupported(descriptor_offset: u32, feature: &'static str) -> LoadError {
    LoadError::UnsupportedFeature {
        descriptor_offset,
        feature,
    }
}

/// Convert HSD_MatAnimJoint payloads once, retaining immutable texture tables.
/// The returned tree can be attached alongside joint animation through the
/// existing JObj/DObj traversal; callers prepare replacements outside the tick.
pub fn material_animation(
    archive: &Archive,
    joint: &desc::MatAnimJoint,
) -> Result<crate::jobj::MatAnimJoint, LoadError> {
    use crate::{mobj::MatAnim, tobj::TexAnim};
    let records = desc::material_animation::MaterialAnimation::read_chain(archive, joint.matanim)?;
    let matanim = records
        .into_iter()
        .map(|record| {
            if record.render_animation.is_some() {
                return Err(unsupported(joint.offset, "render animation"));
            }
            let textures = record
                .textures
                .into_iter()
                .map(|texture| {
                    Ok(TexAnim {
                        id: texture.id,
                        animation: texture
                            .animation
                            .as_ref()
                            .map(animation_object)
                            .transpose()?,
                        images: texture.images.into(),
                        palettes: texture.palettes.into(),
                    })
                })
                .collect::<Result<_, LoadError>>()?;
            Ok(MatAnim {
                aobjdesc: record
                    .animation
                    .as_ref()
                    .map(animation_object)
                    .transpose()?,
                textures,
            })
        })
        .collect::<Result<_, LoadError>>()?;
    Ok(crate::jobj::MatAnimJoint {
        matanim,
        children: joint
            .children()
            .map(|child| material_animation(archive, child))
            .collect::<Result<_, _>>()?,
    })
}
