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
    pub fighters: [FighterAssets; 2],
    pub stage: Archive,
    pub stage_descriptor: &'static crate::scene_stage::StageDescriptor,
    pub stage_desc: melee_gr::desc::StageDesc,
    pub particle_bank: ParticleBank,
    pub effects: Archive,
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
        Ok(Self {
            fighters: fighters.try_into().ok().expect("two character resources"),
            stage,
            stage_descriptor,
            stage_desc,
            particle_bank,
            effects,
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
    pub(crate) fn model(&self, costume: u8) -> (JObjTree, JObjId) {
        let archive = &self.costumes[usize::from(costume)];
        let symbol = self.descriptor.costumes[usize::from(costume)].joint_symbol;
        load_joint_tree(archive, &read_public_jobj(archive, symbol).unwrap()).unwrap()
    }
}
