//! Per-slot character archives and resources selected by the stage descriptor.
use anyhow::{Context, Result};
use hsd_anim::{
    jobj::{JObjId, JObjTree},
    load::load_joint_tree,
};
use hsd_archive::{desc::read_public_jobj, Archive};
use hsd_particle::bank::ParticleBank;
use melee_ft::fighter::assets::{CharacterDescriptor, FighterAssets};
use std::{fs, path::Path};

pub struct Assets {
    pub(crate) items: crate::scene_items::Resources,
    pub arena: melee_ft::fighter::life::Arena,
    pub fighters: [FighterAssets; 2],
    pub stage: Archive,
    pub stage_descriptor: &'static crate::scene_stage::StageDescriptor,
    pub stage_desc: melee_gr::desc::StageDesc,
    pub particle_bank: ParticleBank,
    pub effects: Archive,
    pub fox_effects: Archive,
    pub common_particle_bank: ParticleBank,
    pub characters: [CharacterArchive; 2],
}
impl Assets {
    pub fn load(
        files: &Path,
        descriptors: [&'static CharacterDescriptor; 2],
        stage_descriptor: &'static crate::scene_stage::StageDescriptor,
    ) -> Result<Self> {
        let read = |name| fs::read(files.join(name)).with_context(|| format!("loading {name}"));
        let archive = |name| -> Result<Archive> { Ok(Archive::parse(&read(name)?)?) };
        let common = archive("PlCo.dat")?;
        let mut characters = Vec::new();
        let mut fighters = Vec::new();
        for descriptor in descriptors {
            let data = archive(descriptor.data_file)?;
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
        let camera = [marker(0x95)?, marker(0x96)?];
        let centre = marker(0x94)?;
        let arena = melee_ft::fighter::life::Arena {
            left: low.x.min(high.x),
            right: low.x.max(high.x),
            top: low.y.max(high.y),
            bottom: low.y.min(high.y),
            // Ground_801C39C0 subtracts the camera centre before Stage adds it back.
            camera_top: (camera[0].y.max(camera[1].y) - centre.y) + centre.y,
            revival_positions: [
                marker(4)?,
                marker(5).or_else(|_| marker(4))?,
                marker(6).or_else(|_| marker(4))?,
                marker(7).or_else(|_| marker(4))?,
            ],
            player_revival_markers: stage_desc.kind == melee_types::GrKind::Last,
        };
        Ok(Self {
            items: crate::scene_items::Resources::load(files)?,
            arena,
            fighters: fighters.try_into().ok().expect("two character resources"),
            stage,
            stage_descriptor,
            stage_desc,
            particle_bank,
            effects,
            fox_effects: archive("EfFxData.dat")?,
            common_particle_bank,
            characters: characters.try_into().ok().expect("two character archives"),
        })
    }
}
pub struct CharacterArchive {
    pub descriptor: &'static CharacterDescriptor,
    pub data: Archive,
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
