//! Fighter accessories drawn beside the model: Jigglypuff's costume hats.
use super::*;
use hsd_archive::desc::JObjDesc;
use melee_types::FighterKind;
use std::collections::BTreeSet;

/// `ftPr_Init_803D05B4`: the hat joint each Jigglypuff costume loads.
const PURIN_HATS: [Option<&str>; 5] = [
    None,
    Some("PlyPurinReHat_TopN_joint"),
    Some("PlyPurinBuHat_TopN_joint"),
    Some("PlyPurinGrHat_TopN_joint"),
    Some("PlyPurinYeHat_TopN_joint"),
];
/// ftData `x48_items[1]`: the hat's part data, an FtPartsDesc at +4.
const PURIN_HAT_ITEM: u32 = 1;

/// Jigglypuff's costume hat (ftPr_Init_8013C360): a joint tree from the
/// costume archive, drawn after the model at a fighter bone's matrix
/// (ftPr_Init_UnkMtxFunc0). Its own part sets are shown whole for set 0
/// and hidden otherwise: ftPr_Init_UnkIntBoolFunc0 calls ftParts_80074CA0
/// or 80074D7C as the model's sets change.
pub(super) struct Accessory {
    /// The wearing fighter's slot.
    pub slot: usize,
    pub tree: JObjTree,
    root: JObjId,
    /// The bone whose matrix the hat copies: ftPr_Init_UnkMtxFunc0 reads
    /// `fp->parts[6]` (retail 0x8013C528: `lwz r29, 0x60(r3)`), the
    /// sixth joint in the model's walk, not a part-table lookup.
    pub bone: usize,
    /// DObjs a normal draw hides: listed in sets 1 to 4, not in set 0.
    hidden: BTreeSet<(u32, usize)>,
}
impl Accessory {
    /// The accessory `character`'s costume loads, with its joint description.
    pub fn load(
        character: &crate::assets::CharacterArchive,
        costume: u8,
        slot: usize,
    ) -> Result<Option<(Self, JObjDesc)>, PresentationError> {
        if character.descriptor.kind != FighterKind::Purin {
            return Ok(None);
        }
        let Some(Some(symbol)) = PURIN_HATS.get(usize::from(costume)) else {
            return Ok(None);
        };
        let archive = character.costume(costume);
        let desc = hsd_archive::desc::read_public_jobj(archive, symbol).map_err(error)?;
        let (tree, root) = hsd_anim::load::load_joint_tree(archive, &desc).map_err(error)?;
        // ftParts_80075650 numbers the hat's DObjs in the joint walk's order.
        let slots = fighters::number_dobjs(&desc, None)?;
        let (shown, listed) = hat_sets(character, costume)?;
        let hidden = slots
            .iter()
            .filter(|(_, slot)| listed.contains(*slot) && !shown.contains(*slot))
            .map(|(key, _)| *key)
            .collect();
        Ok(Some((
            Self {
                slot,
                tree,
                root,
                bone: 6,
                hidden,
            },
            desc,
        )))
    }
    /// HSD_JObjCopyMtx with `JOBJ_USER_DEF_MTX`: the hat takes the bone's
    /// world matrix.
    pub fn attach(&mut self, matrix: &hsd_types::Mtx) {
        self.tree.get_mut(self.root).flags |= hsd_anim::jobj::JOBJ_USER_DEF_MTX;
        self.tree.copy_mtx(self.root, matrix);
    }
    pub fn hides(&self, joint: u32, display: usize) -> bool {
        self.hidden.contains(&(joint, display))
    }
}

/// ftParts_8007487C on the hat's FtPartsDesc: the DObj slots set 0 lists,
/// and those the other sets list.
fn hat_sets(
    character: &crate::assets::CharacterArchive,
    costume: u8,
) -> Result<(BTreeSet<u8>, BTreeSet<u8>), PresentationError> {
    let data = &*character.data;
    let link = |offset: u32| data.link(offset).map_err(error);
    let root = data
        .public(character.descriptor.data_symbol)
        .ok_or_else(|| error("fighter data symbol missing"))?;
    let mut shown = BTreeSet::new();
    let mut listed = BTreeSet::new();
    let Some(items) = link(root + 0x48)? else {
        return Ok((shown, listed));
    };
    let Some(hat) = link(items + 4 * PURIN_HAT_ITEM)? else {
        return Ok((shown, listed));
    };
    let parts = hat + 4;
    let group_count = data.reader().u32(parts).map_err(error)? as usize;
    let Some(table) = link(parts + 4)? else {
        return Ok((shown, listed));
    };
    for set in 0..fighters::SET_COUNT as u32 {
        let groups = match link(table + u32::from(costume) * 16 + set * 4)? {
            Some(groups) => Some(groups),
            None => link(table + set * 4)?,
        };
        let Some(groups) = groups else { continue };
        let slots = fighters::read_lookup(data, groups, group_count)?
            .into_iter()
            .flatten()
            .flatten();
        if set == 0 {
            shown.extend(slots);
        } else {
            listed.extend(slots);
        }
    }
    Ok((shown, listed))
}
