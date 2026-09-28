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
pub const CHARACTER_EFFECT_FILES: [CharacterEffectFile; 14] = [
    // Mario (efAsync_DatEntries[1]): model 0x3E8, the fireball's hand flash
    // (efAlt 0x47A), and 0x3E9, the Tornado's (efAlt 0x47C).
    CharacterEffectFile {
        bank: 1,
        file: "EfMrData.dat",
        table: "effMarioDataTable",
        models: 2,
    },
    // Samus (efAsync_DatEntries[2]): models 0x7D0..0x7D2 (efAlt 0x482,
    // 0x486, 0x487).
    CharacterEffectFile {
        bank: 2,
        file: "EfSsData.dat",
        table: "effSamusDataTable",
        models: 3,
    },
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
    // Models 0x1B58..0x1B5C (efSync 0x4BD..0x4C5): the jolt, Skull Bash
    // and Thunder effects.
    CharacterEffectFile {
        bank: 7,
        file: "EfPkData.dat",
        table: "effPikachuDataTable",
        models: 5,
    },
    // Particles only: the Egg Throw burst's generator 0x2328 (efsync.c:225).
    CharacterEffectFile {
        bank: 9,
        file: "EfYsData.dat",
        table: "effYoshiDataTable",
        models: 0,
    },
    // Sing's notes, model 0x2AF8 (efsync.c:305-308).
    CharacterEffectFile {
        bank: 11,
        file: "EfPrData.dat",
        table: "effPurinDataTable",
        models: 1,
    },
    // Particles only: the ice block's generators 0x36B0..0x36B7
    // (efsync.c:414-425).
    CharacterEffectFile {
        bank: 14,
        file: "EfIcData.dat",
        table: "effIceclimberDataTable",
        models: 0,
    },
    // Model 0x3A98 (efSync 0x4D2): the vegetable pull.
    CharacterEffectFile {
        bank: 15,
        file: "EfPeData.dat",
        table: "effPeachDataTable",
        models: 1,
    },
    CharacterEffectFile {
        bank: 16,
        file: "EfMsData.dat",
        table: "effMarsDataTable",
        models: 2,
    },
    // Zelda's and Sheik's shared file (ftData_UnkBytePerCharacter 17):
    // its particles (e.g. the transformation's generators 0x426D / 0x4271,
    // efSync 0x4FC / 0x4FD) and models 0x4268..0x4271 (efSync 0x4F4..0x501);
    // a row without a model is skipped.
    CharacterEffectFile {
        bank: 17,
        file: "EfZdData.dat",
        table: "effZeldaDataTable",
        models: 10,
    },
    // Luigi (efAsync_DatEntries[18]): model 0x4650, the fireball's hand
    // flash (efSync 0x507), and 0x4651, the Cyclone's (efSync 0x509).
    CharacterEffectFile {
        bank: 18,
        file: "EfLgData.dat",
        table: "effLuigiDataTable",
        models: 2,
    },
    // Models 0x4A38..0x4A3D (efSync 0x50B..0x50F): Warlock Punch, Wizard's
    // Foot and Gerudo Dragon.
    CharacterEffectFile {
        bank: 19,
        file: "EfGnData.dat",
        table: "effGanonDataTable",
        models: 6,
    },
    // Models 0xBF68..0xBF69 (efSync 0x510..0x512, efsync.c:642-652): Roy's
    // Counter flash and Flare Blade release.
    CharacterEffectFile {
        bank: 49,
        file: "EfFeData.dat",
        table: "effEmblemDataTable",
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

/// ftData_UnkBytePerCharacter (ftdata.c:1514), indexed by FighterKind: the
/// efAsync_DatEntries row Fighter_Create loads for each kind (-1: none).
pub const FIGHTER_EFFECT_BANKS: [i8; 33] = [
    1, 3, 4, 8, 5, 12, 6, 17, 10, 15, 14, 14, 7, 2, 9, 11, 13, 18, 16, 17, 6, 1, 3, 7, -1, 19, 49,
    -1, -1, -1, -1, 12, -1,
];

#[derive(Clone)]
pub struct Resources {
    pub(super) models: Vec<Arc<Effect>>,
    pub(super) character_banks: CharacterBanks,
    /// efAsync_DatEntries rows the match's fighters loaded, one bit each
    /// (`with_fighters`).
    pub(super) loaded_banks: u64,
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
            loaded_banks: 0,
        })
    }
    /// Fighter_Create's efAsync_LoadSync(ftData_UnkBytePerCharacter[kind])
    /// for each fighter kind (`melee_types::FighterKind` as i32): which
    /// banks retail holds (efsync.c:67 tests Mario's).
    pub fn with_fighters(mut self, kinds: impl IntoIterator<Item = i32>) -> Self {
        for kind in kinds {
            let bank = usize::try_from(kind)
                .ok()
                .and_then(|kind| FIGHTER_EFFECT_BANKS.get(kind))
                .copied()
                .unwrap_or(-1);
            if bank >= 0 {
                self.loaded_banks |= 1 << bank;
            }
        }
        self
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
    // effPeachDataTable has models only: both particle links are null.
    let particles = match (archive.link(table)?, archive.link(table + 4)?) {
        (Some(commands), Some(textures)) => ParticleBank::from_bytes(
            &archive.data()[commands as usize..textures as usize],
            &archive.data()[textures as usize..],
        )?,
        (None, None) => ParticleBank {
            version: 0,
            first_descriptor_id: 0,
            descriptors: Arc::new([]),
            textures: Arc::new([]),
        },
        _ => anyhow::bail!("{symbol}: particle commands without textures"),
    };
    // A row without a model (EF_EffectDesc.model NULL) names no model id;
    // effZeldaDataTable has one.
    let row_model = |index: u32| archive.link(table + 8 + index * 20 + 4);
    let mut rows = Vec::new();
    for index in 0..file.models {
        if row_model(index)?.is_some() {
            rows.push(index);
        }
    }
    let models = rows
        .into_iter()
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
