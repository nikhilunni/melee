//! Jigglypuff's costume hats: ftPr_Init_8013C360 (8013C360) and
//! ftPr_Init_8013C494 (8013C494), ftpurin.c.
//!
//! Costumes 1..4 (red flower, blue bow, green headband, yellow crown) load a
//! separate joint tree from the costume archive into `fp->u.pr.x223C`. Retail
//! then:
//!
//! - builds its DObj list (ftParts_80075650) and part-visibility table
//!   (ftParts_8007487C, `fp->u.pr.x2248`), which only ftPr_Init_UnkIntBoolFunc0
//!   reads when a model part is shown or hidden;
//! - draws it from ftPr_Init_UnkMtxFunc0 (ftData_UnkMtxFunc0, a display
//!   callback) with its root matrix copied from part FtPart_LLegJA's
//!   rendered matrix, while `fp->x2225_b2` is set;
//! - for the blue and green costumes, ftCo_8009DC54 (8009DC54) adds two spring
//!   chains inside the hat (ftData +2C sets 1-2 or 3-4) as dynamics sets 1
//!   and 2 with bone id FtPart_TopN.
//!
//! None of it reaches simulation state. The hat's joints are no fighter part,
//! so ftCo_8009E318 (the dynamics toggle command, which matches part joints)
//! never finds them; ftCo_8009E140 and ftCo_8009E7B4 take a Jigglypuff-only
//! arm that selects set 0 alone; the per-motion table path (x594_b4) that
//! would index fighter parts with a hat set's bone id needs ftData +2C +10,
//! which PlPr.dat leaves null (and no Jigglypuff motion sets that flag).
//! ftCo_8009DD94 solves the hat chains against the hat's render-time matrix,
//! and their joints feed only the hat's own display. The port therefore keeps
//! which hat is worn and which chains retail builds, and simulates neither
//! the hat's pose nor its springs.

/// ftPr_Init_803D05B4: the hat's public joint in each costume archive.
const HAT_JOINTS: [Option<&str>; 5] = [
    None,
    Some("PlyPurinReHat_TopN_joint"),
    Some("PlyPurinBuHat_TopN_joint"),
    Some("PlyPurinGrHat_TopN_joint"),
    Some("PlyPurinYeHat_TopN_joint"),
];

/// ftCo_8009DC54: the costumes whose hat carries spring chains, and the
/// first of the two ftData +2C sets they use (`idx * 2 + 1`).
const fn hat_chains(costume: u8) -> Option<usize> {
    match costume {
        2 => Some(1),
        3 => Some(3),
        _ => None,
    }
}

/// `fp->u.pr.x223C` while a hat is loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CostumeHat {
    /// `fp->x619_costume_id`.
    pub costume: u8,
    /// ftCo_8009DC54: the first of the two ftData +2C sets the hat's chains
    /// use; retail's `dynamics_num` is 3 while they exist.
    pub first_chain_set: Option<usize>,
}

impl CostumeHat {
    /// ftPr_Init_8013C360: the costume's hat, or none for the neutral
    /// costume. HSD_ArchiveGetPublicAddress must find the joint.
    pub fn load(
        archive: &hsd_archive::Archive,
        costume: u8,
    ) -> melee_ft::fighter::assets::Result<Option<Self>> {
        let Some(symbol) = HAT_JOINTS
            .get(usize::from(costume))
            .ok_or_else(|| format!("Jigglypuff has no costume {costume}"))?
        else {
            return Ok(None);
        };
        if archive.public(symbol).is_none() {
            return Err(format!("costume {costume} archive lacks {symbol}").into());
        }
        Ok(Some(Self {
            costume,
            first_chain_set: hat_chains(costume),
        }))
    }
}
