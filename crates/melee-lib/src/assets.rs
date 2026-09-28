//! Per-slot character archives and resources selected by the stage descriptor.
use anyhow::{Context, Result};
use hsd_anim::{
    jobj::{JObjId, JObjTree},
    load::load_joint_tree,
};
use hsd_archive::{desc::read_public_jobj, Archive};
use hsd_particle::bank::ParticleBank;
use melee_ft::fighter::assets::{CharacterDescriptor, FighterAssets};
use std::{fs, mem::ManuallyDrop, path::Path};

pub struct Assets {
    /// FNV-1a over named source bytes in load order; diagnostic identity, not security.
    pub(crate) fingerprint: u64,
    pub(crate) interface: Archive,
    pub(crate) effect_resources: melee_ef::Resources,
    pub(crate) items: crate::scene_items::Resources,
    pub arena: melee_ft::fighter::life::Arena,
    pub stage_camera: melee_cm::StageCamera,
    /// Ground_801C2D24 at the markers the Sudden Death rain reads.
    pub drop_markers: melee_gr::bomb_rain::DropMarkers,
    pub(crate) fighters: ManuallyDrop<[FighterAssets; 2]>,
    pub stage: Archive,
    /// EfCoData.dat, then melee_ef::CHARACTER_EFFECT_FILES in order.
    pub(crate) visual_effect_archives: [Archive; 1 + melee_ef::CHARACTER_EFFECT_FILES.len()],
    pub stage_descriptor: &'static crate::scene_stage::StageDescriptor,
    pub stage_desc: melee_gr::desc::StageDesc,
    pub particle_bank: ParticleBank,
    pub common_particle_bank: ParticleBank,
    pub characters: [CharacterArchive; 2],
    /// Pokemon Stadium's form archives (grpstadium.c `datfiles`), read
    /// mid-match by the transformation; indexed by `Form::archive`.
    pub stage_forms: Vec<FormArchive>,
}
/// One archive of maps loaded after the stage's own (grDatFiles_801C6478).
pub struct FormArchive {
    pub archive: Archive,
    pub models: Vec<melee_gr::desc::ModelDesc>,
}
impl Assets {
    pub fn fighters(&self) -> &[FighterAssets; 2] {
        &self.fighters
    }

    pub fn load(
        files: &Path,
        descriptors: [&'static CharacterDescriptor; 2],
        stage_descriptor: &'static crate::scene_stage::StageDescriptor,
    ) -> Result<Self> {
        use std::hash::Hasher;
        let fingerprint = std::cell::RefCell::new(std::hash::DefaultHasher::new());
        let read = |name: &str| -> Result<Vec<u8>> {
            let bytes = fs::read(files.join(name)).with_context(|| format!("loading {name}"))?;
            let mut hash = fingerprint.borrow_mut();
            hash.write(name.as_bytes());
            hash.write(&[0]);
            hash.write(&(bytes.len() as u64).to_le_bytes());
            hash.write(&bytes);
            Ok(bytes)
        };
        let archive = |name| -> Result<Archive> { Ok(Archive::parse(&read(name)?)?) };
        let common = archive("PlCo.dat")?;
        let mut characters = Vec::new();
        let mut fighters = Vec::new();
        let mut data_archives = std::collections::BTreeMap::new();
        for descriptor in descriptors {
            let data = if let Some(data) = data_archives.get(descriptor.data_file) {
                std::sync::Arc::clone(data)
            } else {
                let data = std::sync::Arc::new(archive(descriptor.data_file)?);
                data_archives.insert(descriptor.data_file, std::sync::Arc::clone(&data));
                data
            };
            let resources = FighterAssets::load(
                descriptor,
                &data,
                &common,
                &read(descriptor.animation_file)?,
            )
            .map_err(|e| anyhow::anyhow!("{} resources: {e}", descriptor.data_symbol))?;
            let costumes = descriptor
                .costumes
                .iter()
                .map(|c| archive(c.file))
                .collect::<Result<_>>()?;
            characters.push(CharacterArchive {
                descriptor,
                data,
                costumes,
            });
            fighters.push(resources);
        }
        let stage = archive(stage_descriptor.file)?;
        let stage_desc = (stage_descriptor.read)(&stage).map_err(|e| anyhow::anyhow!("{e}"))?;
        let particle_bank = ParticleBank::from_archive(&stage, "map_ptcl", "map_texg")?;
        let stage_forms = if stage_desc.kind == melee_types::GrKind::PStadium {
            ["GrPs1.dat", "GrPs2.dat", "GrPs3.dat", "GrPs4.dat"]
                .into_iter()
                .map(|name| -> Result<FormArchive> {
                    let archive = archive(name)?;
                    let models = melee_gr::desc::read_stadium_form(&archive)
                        .map_err(|e| anyhow::anyhow!("{name}: {e}"))?;
                    Ok(FormArchive { archive, models })
                })
                .collect::<Result<_>>()?
        } else {
            Vec::new()
        };
        let effects = archive("EfCoData.dat")?;
        // efAsync_LoadSync (efasync.c:1287-1316): command/texture pointers.
        let table = effects
            .public("effCommonDataTable")
            .context("effect table")?;
        let commands = effects.link(table)?.context("effect commands")? as usize;
        let textures = effects.link(table + 4)?.context("effect textures")? as usize;
        let common_particle_bank = ParticleBank::from_bytes(
            &effects.data()[commands..textures],
            &effects.data()[textures..],
        )?;
        let marker = |index| stage_position(&stage, &stage_desc, index);
        let low = marker(0x97)?;
        let high = marker(0x98)?;
        let camera_range = camera_range(&stage_desc, |index| {
            let bound = stage_desc
                .position_bindings
                .iter()
                .any(|b| b.stage_position == index);
            bound.then(|| marker(index)).transpose()
        })?;
        let arena = melee_ft::fighter::life::Arena {
            left: low.x.min(high.x),
            right: low.x.max(high.x),
            top: low.y.max(high.y),
            bottom: low.y.min(high.y),
            // Ground_801C39C0 subtracts the camera centre before Stage adds it back.
            camera_top: camera_range.bounds.top + camera_range.offset.y,
            revival_positions: [
                marker(4)?,
                marker(5).or_else(|_| marker(4))?,
                marker(6).or_else(|_| marker(4))?,
                marker(7).or_else(|_| marker(4))?,
            ],
            // stage_info.unk8C.b4: set by grlast.c:233 and grbattle.c:147,
            // cleared by grstory.c:77 and groldpupupu.c:145.
            player_revival_markers: matches!(
                stage_desc.kind,
                melee_types::GrKind::Last | melee_types::GrKind::Battle
            ),
        };
        let stage_camera = stage_camera(&stage_desc, &camera_range, [low, high])?;
        // Ground_801C2D24 fails for a marker the stage binds no joint to.
        let bound_marker = |index: i16| -> Result<Option<hsd_types::Vec3>> {
            if stage_desc
                .position_bindings
                .iter()
                .any(|b| b.stage_position == index)
            {
                marker(index).map(Some)
            } else {
                Ok(None)
            }
        };
        let mut drop_markers = melee_gr::bomb_rain::DropMarkers::default();
        for (i, slot) in drop_markers.items.iter_mut().enumerate() {
            *slot = bound_marker(melee_gr::bomb_rain::ITEM_MARKER_FIRST + i as i16)?;
        }
        for (i, slot) in drop_markers.spawns.iter_mut().enumerate() {
            *slot = bound_marker(i as i16)?;
        }
        let [fox_effects, captain_effects, yoshi_effects, purin_effects, peach_effects, mars_effects] =
            melee_ef::CHARACTER_EFFECT_FILES.map(|file| archive(file.file));
        let character_effects = [
            fox_effects?,
            captain_effects?,
            yoshi_effects?,
            purin_effects?,
            peach_effects?,
            mars_effects?,
        ];
        let effect_resources = melee_ef::Resources::load(&effects, &character_effects)?;
        let interface = archive("IfAll.usd")?;
        // it_8027B798 reads p_ftCommonData, the same PlCo every slot loads.
        let common_damage = &fighters[0].damage;
        let launch = melee_it::hurt::ItemLaunch {
            velocity_scale: common_damage.velocity_scale,
            air_angle: common_damage.sakurai_air_angle,
            ground_angle: common_damage.sakurai_ground_angle,
            ground_threshold: common_damage.grounded_angle_threshold,
            ground_maximum: common_damage.sakurai_maximum_threshold,
        };
        let items = crate::scene_items::Resources::load(&read, &characters, &stage, launch)?;
        let fighters = fighters.try_into().ok().expect("two character resources");
        let characters = characters.try_into().ok().expect("two character archives");
        // Finish all fallible work before installing manually dropped ownership.
        Ok(Self {
            fingerprint: fingerprint.into_inner().finish(),
            visual_effect_archives: {
                let [fox, captain, yoshi, purin, peach, mars] = character_effects;
                [effects, fox, captain, yoshi, purin, peach, mars]
            },
            effect_resources,
            interface,
            items,
            arena,
            stage_camera,
            drop_markers,
            fighters: ManuallyDrop::new(fighters),
            stage,
            stage_descriptor,
            stage_desc,
            particle_bank,
            common_particle_bank,
            characters,
            stage_forms,
        })
    }
}
pub struct CharacterArchive {
    pub descriptor: &'static CharacterDescriptor,
    pub data: std::sync::Arc<Archive>,
    costumes: Vec<Archive>,
}
impl CharacterArchive {
    pub(crate) fn costume(&self, costume: u8) -> &Archive {
        &self.costumes[usize::from(costume)]
    }
    pub(crate) fn model(&self, costume: u8) -> (JObjTree, JObjId) {
        let archive = &self.costumes[usize::from(costume)];
        let symbol = self.descriptor.costumes[usize::from(costume)].joint_symbol;
        load_joint_tree(archive, &read_public_jobj(archive, symbol).unwrap()).unwrap()
    }
}

/// Ground_801C2D24: resolve an archive stage-position binding under map scale.
/// Ground_801C0800, Ground_801C39C0 and Ground_801C3BB4: the stage's camera
/// description from grGroundParam and the camera (0x94..0x96) and blast
/// zone (0x97, 0x98) markers, all relative to the camera centre.
/// `Ground_801C39C0` (0x801C39C0): the camera bounds relative to the camera
/// centre (marker 0x94) and that centre, or the default "dummy CamRange"
/// when a marker is missing (Pokemon Stadium binds no 0x94).
struct CameraRange {
    bounds: melee_cm::Rect,
    offset: hsd_types::Vec3,
}
fn camera_range(
    stage: &melee_gr::desc::StageDesc,
    marker: impl Fn(i16) -> Result<Option<hsd_types::Vec3>>,
) -> Result<CameraRange> {
    use melee_types::GrKind;
    let (Some(a), Some(b), Some(centre)) = (marker(0x95)?, marker(0x96)?, marker(0x94)?) else {
        anyhow::ensure!(
            !matches!(
                stage.kind,
                GrKind::Castle
                    | GrKind::Corneria
                    | GrKind::Unk26
                    | GrKind::Inishie2
                    | GrKind::RCruise
                    | GrKind::Yorster
                    | GrKind::MuteCity
            ),
            "ground.c:2183-2238: stage-specific dummy CamRange"
        );
        // ground.c:2175-2181.
        return Ok(CameraRange {
            bounds: melee_cm::Rect {
                left: -170.0,
                right: 170.0,
                top: 120.0,
                bottom: -60.0,
            },
            offset: hsd_types::Vec3::ZERO,
        });
    };
    let (left, right) = if a.x < b.x {
        (a.x - centre.x, b.x - centre.x)
    } else {
        (b.x - centre.x, a.x - centre.x)
    };
    let (bottom, top) = if a.y < b.y {
        (a.y - centre.y, b.y - centre.y)
    } else {
        (b.y - centre.y, a.y - centre.y)
    };
    Ok(CameraRange {
        bounds: melee_cm::Rect {
            left,
            right,
            top,
            bottom,
        },
        offset: centre,
    })
}

fn stage_camera(
    stage: &melee_gr::desc::StageDesc,
    range: &CameraRange,
    blast: [hsd_types::Vec3; 2],
) -> Result<melee_cm::StageCamera> {
    use melee_types::GrKind;
    anyhow::ensure!(
        !matches!(
            stage.kind,
            GrKind::Castle
                | GrKind::Corneria
                | GrKind::Zebes
                | GrKind::Garden
                | GrKind::KinokoRoute
                | GrKind::Homerun
        ),
        "Camera_8002AF68: the stage's lowest eye height is not ported"
    );
    let centre = range.offset;
    let [a, b] = blast;
    let (blast_left, blast_right) = if a.x < b.x {
        (a.x - centre.x, b.x - centre.x)
    } else {
        (b.x - centre.x, a.x - centre.x)
    };
    let (blast_bottom, blast_top) = if a.y < b.y {
        (a.y - centre.y, b.y - centre.y)
    } else {
        (b.y - centre.y, a.y - centre.y)
    };
    let p = &stage.parameters.camera;
    Ok(melee_cm::StageCamera {
        bounds: range.bounds,
        offset_x: centre.x,
        offset_y: centre.y,
        fov: p.fov,
        pan_degrees: p.pan_degrees,
        yaw_scale: p.yaw_scale,
        pitch_scale: p.pitch_scale,
        track_ratio: p.track_ratio,
        fixed_zoom: p.fixed_zoom,
        track_smooth: p.track_smooth,
        min_depth: p.min_depth,
        max_depth: p.max_depth,
        blast_zone: melee_cm::Rect {
            left: blast_left,
            right: blast_right,
            top: blast_top,
            bottom: blast_bottom,
        },
        // Ground_801BFFB0's stage_info.x724.
        floor: -10000.0,
        min_eye_height: -f32::MAX,
    })
}
pub(crate) fn stage_position(
    archive: &Archive,
    stage: &melee_gr::desc::StageDesc,
    index: i16,
) -> Result<hsd_types::Vec3> {
    let binding = stage
        .position_bindings
        .iter()
        .find(|b| b.stage_position == index)
        .context("stage marker")?;
    let (mut tree, root) = load_joint_tree(archive, &stage.models[binding.model_id].joint)?;
    // grAnime_801C8138 evaluates frame zero before Ground_801C39C0 reads markers.
    if let Some(animation) = stage.models[binding.model_id].animations.first() {
        hsd_anim::load::attach_anim_joint(&mut tree, root, animation, archive)?;
        tree.req_anim_all(root, 0.0);
        tree.anim_all::<melee_ft::fighter::RetailTrig>(root);
        ensure_no_marker_events(&tree)?;
    }
    let wrapper = tree.alloc();
    let scale = stage.parameters.map_scale;
    tree.set_scale(wrapper, &hsd_types::Vec3::new(scale, scale, scale));
    tree.add_child(wrapper, root);
    let joint = tree
        .bone(root, binding.joint_index as usize)
        .context("stage marker joint")?;
    Ok(melee_ft::collision::ecb::world_position(&mut tree, joint))
}

fn ensure_no_marker_events(tree: &JObjTree) -> Result<()> {
    anyhow::ensure!(
        tree.events.is_empty(),
        "stage marker model has side-effect animation events"
    );
    Ok(())
}

impl Drop for Assets {
    #[inline(never)]
    fn drop(&mut self) {
        // SAFETY: load installs a complete array only after fallible setup. The
        // field is never taken out; this is its only destruction path. Keeping
        // it here prevents resource drop glue being generated by every app.
        unsafe {
            ManuallyDrop::drop(&mut self.fighters);
        }
    }
}

#[cfg(test)]
mod visual_tests {
    use super::*;
    #[test]
    fn final_destination_visual_geometry_and_textures_decode() {
        let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../harness/roms/files/GrNLa.dat");
        if !melee_test_support::require_files([&file]) {
            return;
        }
        let archive = Archive::parse(&std::fs::read(file).unwrap()).unwrap();
        let stage = melee_gr::desc::read_final_destination(&archive).unwrap();
        fn count(
            archive: &Archive,
            decoder: &mut hsd_archive::visual::TextureDecoder<'_>,
            joint: &hsd_archive::desc::JObjDesc,
        ) -> (usize, usize) {
            let mut triangles = 0;
            let mut images = 0;
            let mut display = joint.u.dobj();
            while let Some(dobj) = display {
                for mesh in hsd_archive::visual::read_polygons(archive, dobj.pobjdesc).unwrap() {
                    triangles += mesh.indices.len() / 3;
                }
                if let Some(material) = &dobj.mobj {
                    images += decoder.read_chain(material.texdesc).unwrap().len();
                }
                display = dobj.next.as_deref();
            }
            for subtree in [joint.child.as_deref(), joint.next.as_deref()]
                .into_iter()
                .flatten()
            {
                let (t, i) = count(archive, decoder, subtree);
                triangles += t;
                images += i;
            }
            (triangles, images)
        }
        let mut triangles = 0;
        let mut images = 0;
        let mut decoder = hsd_archive::visual::TextureDecoder::new(&archive);
        for model in &stage.models {
            let (t, i) = count(&archive, &mut decoder, &model.joint);
            triangles += t;
            images += i;
        }
        assert!(triangles > 1000);
        assert!(images > 0);
        eprintln!("Final Destination: {triangles} triangles, {images} texture layers");
    }
}
