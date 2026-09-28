//! Immutable effect definitions, separated from per-match animation slots.
use super::*;
use std::sync::Arc;

/// One character row of efAsync_DatEntries (efasync.c:1209-1260) the port
/// loads: its particle bank number (the row index), file, table symbol and
/// the model ids its table rows take (`bank * 1000 + row`).
#[derive(Clone, Copy, Debug)]
pub struct CharacterEffectFile {
    pub bank: u8,
    pub file: &'static str,
    pub table: &'static str,
    pub models: u32,
}
impl CharacterEffectFile {
    pub const fn first_model(&self) -> u32 {
        self.bank as u32 * 1000
    }
}

/// Character effect files loaded with every scene, in bank order.
pub const CHARACTER_EFFECT_FILES: [CharacterEffectFile; 3] = [
    CharacterEffectFile {
        bank: 3,
        file: "EfFxData.dat",
        table: "effFoxDataTable",
        models: 6,
    },
    CharacterEffectFile {
        bank: 4,
        file: "EfCaData.dat",
        table: "effCaptainDataTable",
        models: 6,
    },
    CharacterEffectFile {
        bank: 16,
        file: "EfMsData.dat",
        table: "effMarsDataTable",
        models: 2,
    },
];

/// Particle banks of `CHARACTER_EFFECT_FILES`, in the same order.
pub(super) type CharacterBanks = [Option<ParticleBank>; CHARACTER_EFFECT_FILES.len()];

/// The particle banks an effect's DPtcl events and spawns select from.
#[derive(Clone, Copy)]
pub(super) struct Banks<'a> {
    pub common: &'a ParticleBank,
    pub characters: &'a CharacterBanks,
}
impl<'a> Banks<'a> {
    /// The bank a particle id's high part names (efLib_SpawnParticleEffect).
    pub fn get(&self, bank: i32) -> Result<&'a ParticleBank> {
        if bank == 0 {
            return Ok(self.common);
        }
        character_bank(self.characters, bank)
    }
}
pub(super) fn character_bank(banks: &CharacterBanks, bank: i32) -> Result<&ParticleBank> {
    CHARACTER_EFFECT_FILES
        .iter()
        .position(|file| i32::from(file.bank) == bank)
        .and_then(|index| banks[index].as_ref())
        .with_context(|| format!("effect particle bank {bank} not loaded"))
}
pub(super) fn is_character_bank(bank: u8) -> bool {
    CHARACTER_EFFECT_FILES.iter().any(|file| file.bank == bank)
}

#[derive(Clone)]
pub struct Resources {
    pub(super) models: Vec<Arc<Effect>>,
    pub(super) character_banks: CharacterBanks,
}
impl Resources {
    /// `characters` holds the archives of `CHARACTER_EFFECT_FILES`, in order.
    pub fn load(
        common: &Archive,
        characters: &[Archive; CHARACTER_EFFECT_FILES.len()],
    ) -> Result<Self> {
        let mut models = pool::load_common(common)?;
        let mut character_banks = CharacterBanks::default();
        for ((file, archive), slot) in CHARACTER_EFFECT_FILES
            .iter()
            .zip(characters)
            .zip(&mut character_banks)
        {
            let (bank, bank_models) = character(archive, file)?;
            models.extend(bank_models);
            *slot = Some(bank);
        }
        Ok(Self {
            models,
            character_banks,
        })
    }
}
pub(super) fn character(
    archive: &Archive,
    file: &CharacterEffectFile,
) -> Result<(ParticleBank, Vec<Arc<Effect>>)> {
    let symbol = file.table;
    let table = archive
        .public(symbol)
        .with_context(|| format!("{symbol} effect table"))?;
    let commands = archive.link(table)?.context("particle commands")? as usize;
    let textures = archive.link(table + 4)?.context("particle textures")? as usize;
    let particles = ParticleBank::from_bytes(
        &archive.data()[commands..textures],
        &archive.data()[textures..],
    )?;
    let models = (0..file.models)
        .map(|index| {
            Effect::load_table(
                archive,
                symbol,
                index,
                file.first_model() + index,
                file.bank,
            )
            .map(Arc::new)
        })
        .collect::<Result<_>>()?;
    Ok((particles, models))
}
