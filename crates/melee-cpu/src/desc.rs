//! The CPU's data in PlCo.dat: `ftLoadCommonData` pData[22], which
//! Fighter_800679B0 stores in `Fighter_804D64FC` (fighter.h:15).
use hsd_archive::Archive;

/// Fighter kinds the per-kind tables cover (FTKIND_MAX).
pub const KINDS: usize = 33;
/// `cmdscripts` entries: the pointer array runs up to the `x4` table.
const SCRIPTS: usize = 62;

/// `ftCo_AttackEntry` (ftcpuattack.c:29), 0x24 bytes: an attack the CPU
/// may use and the box, relative to itself, in which it hits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttackEntry {
    /// +00: the command script to run (an index into `scripts`).
    pub script: i32,
    /// +04: frames until the attack hits.
    pub frames: i32,
    /// +08..+14: the hit box, front/back x and low/high y.
    pub front: f32,
    pub back: f32,
    pub low: f32,
    pub high: f32,
    /// +18.
    pub weight: f32,
    /// +1C: the attack is considered only when x80 % period == 0.
    pub period: i32,
    /// +20: the lowest CPU level that uses it.
    pub level: i32,
}

impl AttackEntry {
    /// A placeholder for fixed arrays of entries.
    pub const NONE: Self = Self {
        script: 0,
        frames: 0,
        front: 0.0,
        back: 0.0,
        low: 0.0,
        high: 0.0,
        weight: 0.0,
        period: 0,
        level: 0,
    };
}

/// `struct Fighter_804D64FC_t`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CpuData {
    /// +00: command scripts, each ending in CpuCmd_Done.
    pub scripts: Vec<Vec<u8>>,
    /// +04: ground attacks per kind.
    pub ground: Vec<Option<Vec<AttackEntry>>>,
    /// +08: air attacks per kind.
    pub air: Vec<Option<Vec<AttackEntry>>>,
    /// +10: attacks against a target in the ledge states.
    pub smash: Vec<Option<Vec<AttackEntry>>>,
    /// +14: attacks against stage enemies.
    pub special: Vec<Option<Vec<AttackEntry>>>,
    /// +18: attacks with a held weapon.
    pub weapon: Vec<Option<Vec<AttackEntry>>>,
    /// +1C: attacks against a target over no floor.
    pub edge_guard: Vec<Option<Vec<AttackEntry>>>,
    /// +20: a distance per kind.
    // TODO(meaning): read by ftCo_800A20A0 (twice it is the "close" range).
    pub reach: Vec<f32>,
    /// +24: the weapon reach bonus per weapon kind.
    pub weapon_reach: [f32; 6],
}

type Result<T> = std::result::Result<T, String>;

fn io<T>(r: std::result::Result<T, hsd_archive::Error>) -> Result<T> {
    r.map_err(|e| e.to_string())
}

/// A relocated pointer that must be present.
fn pointer(archive: &Archive, at: u32, what: &str) -> Result<u32> {
    io(archive.link(at))?.ok_or_else(|| format!("null {what}"))
}

impl CpuData {
    /// Read pData[22] of `ftLoadCommonData`.
    pub fn read(archive: &Archive) -> Result<Self> {
        Self::read_inner(archive).map_err(|e| format!("PlCo CPU data: {e}"))
    }

    fn read_inner(archive: &Archive) -> Result<Self> {
        let root = archive
            .public("ftLoadCommonData")
            .ok_or("missing ftLoadCommonData")?;
        let data = pointer(archive, root + 22 * 4, "pData[22]")?;
        let scripts_table = pointer(archive, data, "cmdscripts")?;
        let scripts = (0..SCRIPTS as u32)
            .map(|i| -> Result<Vec<u8>> {
                match io(archive.link(scripts_table + 4 * i))? {
                    Some(script) => read_script(archive, script),
                    None => Ok(Vec::new()),
                }
            })
            .collect::<Result<_>>()?;
        let table = |field: u32| -> Result<Vec<Option<Vec<AttackEntry>>>> {
            let per_kind = pointer(archive, data + field, "attack table")?;
            (0..KINDS as u32)
                .map(|kind| {
                    io(archive.link(per_kind + 4 * kind))?
                        .map(|list| read_entries(archive, list))
                        .transpose()
                })
                .collect()
        };
        let r = archive.reader();
        let reach_table = pointer(archive, data + 0x20, "reach table")?;
        let reach = (0..KINDS as u32)
            .map(|kind| io(r.f32(reach_table + 4 * kind)))
            .collect::<Result<_>>()?;
        let weapons = pointer(archive, data + 0x24, "weapon reach table")?;
        let mut weapon_reach = [0.0; 6];
        for (i, reach) in weapon_reach.iter_mut().enumerate() {
            *reach = io(r.f32(weapons + 4 * i as u32))?;
        }
        Ok(Self {
            scripts,
            ground: table(0x04)?,
            air: table(0x08)?,
            smash: table(0x10)?,
            special: table(0x14)?,
            weapon: table(0x18)?,
            edge_guard: table(0x1C)?,
            reach,
            weapon_reach,
        })
    }
}

/// The bytes ftCo_800B4880 (0x800B4880) copies from a script, through the
/// closing CpuCmd_Done. Its argument test reads through the moving cursor:
/// after a two-argument command's first argument it tests that argument,
/// not the command, before copying a second byte.
fn read_script(archive: &Archive, mut at: u32) -> Result<Vec<u8>> {
    let r = archive.reader();
    let mut bytes = Vec::new();
    loop {
        let command = io(r.u8(at))?;
        if command == 0x7F {
            bytes.push(command);
            return Ok(bytes);
        }
        bytes.push(command);
        if io(r.u8(at))? > 0xBF {
            at += 1;
            bytes.push(io(r.u8(at))?);
        }
        if io(r.u8(at))? > 0x7F {
            at += 1;
            bytes.push(io(r.u8(at))?);
        }
        at += 1;
    }
}

/// Entries up to the one whose command is zero.
fn read_entries(archive: &Archive, mut at: u32) -> Result<Vec<AttackEntry>> {
    let r = archive.reader();
    let mut entries = Vec::new();
    loop {
        let script = io(r.s32(at))?;
        if script == 0 {
            return Ok(entries);
        }
        entries.push(AttackEntry {
            script,
            frames: io(r.s32(at + 4))?,
            front: io(r.f32(at + 8))?,
            back: io(r.f32(at + 0xC))?,
            low: io(r.f32(at + 0x10))?,
            high: io(r.f32(at + 0x14))?,
            weight: io(r.f32(at + 0x18))?,
            period: io(r.s32(at + 0x1C))?,
            level: io(r.s32(at + 0x20))?,
        });
        at += 0x24;
    }
}
