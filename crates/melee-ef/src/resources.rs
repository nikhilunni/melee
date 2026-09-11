//! Immutable effect definitions, separated from per-match animation slots.
use super::*;
use std::sync::Arc;

#[derive(Clone)]
pub struct Resources {
    pub(super) models: Vec<Arc<Effect>>,
    pub(super) fox_bank: ParticleBank,
    pub(super) mars_bank: ParticleBank,
}
impl Resources {
    pub fn load(common: &Archive, fox: &Archive, mars: &Archive) -> Result<Self> {
        let mut models = pool::load_common(common)?;
        let (fox_bank, fox_models) = character(fox, "effFoxDataTable", 3, 0xBB8, 6)?;
        let (mars_bank, mars_models) = character(mars, "effMarsDataTable", 16, 0x3E80, 2)?;
        models.extend(fox_models);
        models.extend(mars_models);
        Ok(Self {
            models,
            fox_bank,
            mars_bank,
        })
    }
}
pub(super) fn character(
    archive: &Archive,
    symbol: &str,
    bank: u8,
    first_id: u32,
    count: u32,
) -> Result<(ParticleBank, Vec<Arc<Effect>>)> {
    let table = archive
        .public(symbol)
        .with_context(|| format!("{symbol} effect table"))?;
    let commands = archive.link(table)?.context("particle commands")? as usize;
    let textures = archive.link(table + 4)?.context("particle textures")? as usize;
    let particles = ParticleBank::from_bytes(
        &archive.data()[commands..textures],
        &archive.data()[textures..],
    )?;
    let models = (0..count)
        .map(|index| {
            Effect::load_table(archive, symbol, index, first_id + index, bank).map(Arc::new)
        })
        .collect::<Result<_>>()?;
    Ok((particles, models))
}
