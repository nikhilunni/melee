//! Archive resources for the supported two-Fox FD savestate.
use anyhow::{Context, Result};
use ft_fox::{attributes::read_fox_attributes, init::Fox};
use hsd_anim::{
    jobj::{JObjId, JObjTree},
    load::load_joint_tree,
};
use hsd_archive::{desc::read_public_jobj, Archive};
use hsd_particle::bank::ParticleBank;
use melee_ft::fighter::assets::FighterAssets;
use std::{fs, path::Path};

pub struct Assets {
    pub fighter: FighterAssets,
    pub stage: Archive,
    pub stage_desc: melee_gr::desc::StageDesc,
    pub particle_bank: ParticleBank,
    pub effects: Archive,
    pub common_particle_bank: ParticleBank,
    fox: Archive,
    costumes: [Archive; 2],
}
impl Assets {
    pub fn load(files: &Path) -> Result<Self> {
        let read = |name| fs::read(files.join(name)).with_context(|| format!("loading {name}"));
        let archive = |name| -> Result<Archive> { Ok(Archive::parse(&read(name)?)?) };
        let fox = archive("PlFx.dat")?;
        let fighter = FighterAssets::fox(&fox, &archive("PlCo.dat")?, &read("PlFxAJ.dat")?)
            .map_err(|e| anyhow::anyhow!("Fox resources: {e}"))?;
        let stage = archive("GrNLa.dat")?;
        let stage_desc =
            melee_gr::desc::read_final_destination(&stage).map_err(|e| anyhow::anyhow!("{e}"))?;
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
            fighter,
            stage,
            stage_desc,
            particle_bank,
            effects,
            common_particle_bank,
            fox,
            costumes: [archive("PlFxNr.dat")?, archive("PlFxOr.dat")?],
        })
    }
    pub(crate) fn model(&self, costume: u8) -> (JObjTree, JObjId) {
        let archive = &self.costumes[usize::from(costume)];
        let symbol = ["PlyFox5K_Share_joint", "PlyFox5KOr_Share_joint"][usize::from(costume)];
        load_joint_tree(archive, &read_public_jobj(archive, symbol).unwrap()).unwrap()
    }
    pub(crate) fn character(&self) -> Fox {
        Fox::new(read_fox_attributes(&self.fox).unwrap())
    }
}
