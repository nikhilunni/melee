//! Fighter model display state (ftparts.c, ftanim.c and ftdrawcommon.c):
//! which of a fighter's DObjs a normal draw shows, from the ftData part
//! groups, the simulation's group selections and the fighter's draw gates;
//! and the costume's material animation with its texture requests.
use super::*;
use hsd_archive::desc::JObjDesc;
use melee_types::FighterKind;

/// The ftData `x8->x0` part sets the normal draw reads. Set 2 indexes the
/// metal model's DObj list (`fp->x203C`), which versus never draws.
const HIGH_POLY: usize = 0;
const LOW_POLY: usize = 1;
const METAL_EXTRA: usize = 3;
const SET_COUNT: usize = 4;
/// `ftParts_80074194`'s DObj list capacity (ftparts.c:369).
const DOBJ_CAPACITY: usize = 128;
/// `CostumeTObjList.costume_tobjs` capacity (ftanim.c:1007).
const COSTUME_TOBJ_CAPACITY: u32 = 5;
/// `FtPartsVis.model_num` limit (ftparts.c:516).
const GROUP_CAPACITY: usize = 11;

/// `FtPartsVisLookup[model_num]` for one set: per group, per variant, the
/// `dobj_list` indices it covers.
type Lookup = Vec<Vec<Vec<u8>>>;

/// ftData `x8`: the part sets and the costume texture list.
struct PartData {
    group_count: usize,
    sets: [Option<Lookup>; SET_COUNT],
    costume_textures: Vec<u16>,
}

/// One fighter model's part groups, DObj numbering and costume materials.
pub(super) struct FighterParts {
    kind: FighterKind,
    costume: u8,
    /// ftData `x8->x0.model_num`: how many groups `x5F4_arr` selects.
    group_count: usize,
    /// `vis->xC[set]` after ftParts_8007487C's costume fallback.
    sets: [Option<Lookup>; SET_COUNT],
    /// `dobj_list` index of each mesh source, keyed by the owning joint's
    /// description offset and the DObj's position in its joint's chain.
    slots: BTreeMap<(u32, usize), u8>,
    /// The current frame's `HSD_DObj` flag bit 0 for each `dobj_list` slot.
    hidden: [bool; DOBJ_CAPACITY],
    /// Whether the fighter's model draws at all this frame.
    drawn: bool,
    materials: CostumeMaterials,
    outline: Option<Outline>,
}

/// ftGw_Init_OnLoad (ftgamewatch.c:522-541): Mr. Game & Watch's body colour
/// per costume (ftMaterial_800BFB4C) and his outline, part set 4
/// (`x5AC.xC[4] = items[10]`). ftDrawCommon_80080E18 draws the outline's
/// DObjs first in the outline colour without depth writes
/// (ftDrawCommon_80080E18_inline0, ftMaterial_800BF6BC), then the model.
struct Outline {
    lookup: Lookup,
    /// `x4_GAMEWATCH_COLOR[costume]`.
    body: [u8; 4],
    /// `x14_GAMEWATCH_OUTLINE` (`x610_color_rgba[1]`): alpha is its weight.
    color: [u8; 4],
    /// This frame's set-4 selection per `dobj_list` slot.
    shown: [bool; DOBJ_CAPACITY],
}
impl Outline {
    fn covers(&self, slot: usize) -> bool {
        self.lookup
            .iter()
            .flatten()
            .any(|dobjs| dobjs.contains(&(slot as u8)))
    }
    /// The outline lookup is ftData `x48_items[10]`; the colours are
    /// ftGameWatchAttributes +0x04 (per costume) and +0x14.
    fn read(
        character: &crate::assets::CharacterArchive,
        costume: u8,
        group_count: usize,
    ) -> Result<Option<Self>, PresentationError> {
        const OUTLINE_ITEM: u32 = 10;
        if character.descriptor.kind != FighterKind::GameWatch {
            return Ok(None);
        }
        let data = &*character.data;
        let r = data.reader();
        let link = |offset: u32| {
            data.link(offset)
                .map_err(error)?
                .ok_or_else(|| error("Game & Watch outline data missing"))
        };
        let root = data
            .public(character.descriptor.data_symbol)
            .ok_or_else(|| error("fighter data symbol missing"))?;
        let attributes = link(root + 4)?;
        let items = link(root + 0x48)?;
        let lookup = read_lookup(data, link(items + 4 * OUTLINE_ITEM)?, group_count)?;
        let color = |offset: u32| -> Result<[u8; 4], PresentationError> {
            let bytes = r.slice(attributes + offset, 4).map_err(error)?;
            Ok([bytes[0], bytes[1], bytes[2], bytes[3]])
        };
        Ok(Some(Self {
            lookup,
            body: color(4 + 4 * u32::from(costume.min(3)))?,
            color: color(0x14)?,
            shown: [false; DOBJ_CAPACITY],
        }))
    }
}

/// ftAnim_80070308: the costume's MatAnimJoint on the costume's own joints
/// (before OnLoad's graft), requested at frame 0; ftAnim_80070200's costume
/// TObjs follow the simulation's ftAnim_800704F0 frame requests.
struct CostumeMaterials {
    tree: JObjTree,
    root: JObjId,
    /// Joint description offset to the material tree's joint.
    ids: BTreeMap<u32, JObjId>,
    /// Costume TObj `i`: its joint offset, DObj and texture positions.
    textures: Vec<(u32, usize, usize)>,
}

impl FighterParts {
    /// ftParts_SetupParts (DObj numbering), ftParts_8007487C (part sets)
    /// and ftAnim_80070308 (costume materials). `skeleton` is the
    /// simulation's model of `model`.
    pub fn new(
        character: &crate::assets::CharacterArchive,
        costume: u8,
        model: &JObjDesc,
        skeleton: &JObjTree,
    ) -> Result<Self, PresentationError> {
        let data = read_part_data(character, costume)?;
        let mut slots = BTreeMap::new();
        let mut grafted = Vec::new();
        Walk {
            graft: character.graft_part().map(usize::from),
            joint_index: 0,
            next: 0,
            slots: &mut slots,
            grafted: &mut grafted,
        }
        .number(model)?;
        for (key, slot) in grafted {
            slots.retain(|_, existing| *existing != slot);
            slots.insert(key, slot);
        }
        let outline = Outline::read(character, costume, data.group_count)?;
        let materials = CostumeMaterials::new(
            character,
            costume,
            skeleton,
            &slots,
            &data.costume_textures,
            outline.as_ref().map(|o| o.body),
        )?;
        Ok(Self {
            kind: character.descriptor.kind,
            costume,
            group_count: data.group_count,
            sets: data.sets,
            slots,
            hidden: [false; DOBJ_CAPACITY],
            drawn: true,
            materials,
            outline,
        })
    }

    /// The draw state for this frame: ftDrawCommon_800805C8's normal path
    /// (ftdrawcommon.c:233-241) on a non-metal fighter. Sets 1 (low poly)
    /// and 3 (metal-only parts) stay hidden as ftParts_8007487C left them;
    /// set 0 shows each group's selected variant and hides its others
    /// (ftParts_80074B6C). Then the costume materials at this frame.
    pub fn capture(&mut self, fighter: &melee_ft::fighter::Fighter) {
        self.hidden = [false; DOBJ_CAPACITY];
        for set in [LOW_POLY, METAL_EXTRA] {
            for slot in self.sets[set].iter().flatten().flatten().flatten() {
                self.hidden[usize::from(*slot)] = true;
            }
        }
        let selected: [i32; GROUP_CAPACITY] =
            std::array::from_fn(|group| self.selection(fighter, group));
        if let Some(groups) = &self.sets[HIGH_POLY] {
            apply_selection(groups, &selected, |slot, shown| self.hidden[slot] = !shown);
        }
        if let Some(outline) = &mut self.outline {
            outline.shown = [false; DOBJ_CAPACITY];
            apply_selection(&outline.lookup, &selected, |slot, shown| {
                outline.shown[slot] = shown
            });
        }
        // ftDrawCommon_80080E18 skips a disabled fighter (x221F_b3, the
        // inactive transformation partner); ftDrawCommon_800805C8 one that
        // is invisible or hidden by its script (x221E_b5).
        self.drawn = !(fighter.status.disabled
            || fighter.effect_state.invisible
            || fighter.commands.fighter_hidden);
        self.materials.capture(fighter);
    }

    /// Whether the mesh from `joint`'s `display`th DObj is hidden this frame.
    pub fn hides(&self, joint: u32, display: usize) -> bool {
        let Some(slot) = self.slots.get(&(joint, display)).map(|s| usize::from(*s)) else {
            return !self.drawn;
        };
        if let Some(outline) = self.outline.as_ref().filter(|o| o.covers(slot)) {
            return !self.drawn || !outline.shown[slot];
        }
        !self.drawn || self.hidden[slot]
    }

    /// Whether the mesh from `joint`'s `display`th DObj is outline geometry,
    /// drawn before the model without depth writes.
    pub fn is_outline(&self, joint: u32, display: usize) -> bool {
        self.slots.get(&(joint, display)).is_some_and(|slot| {
            self.outline
                .as_ref()
                .is_some_and(|o| o.covers(usize::from(*slot)))
        })
    }

    /// The post-texture colour overlay for the mesh, if any: the outline
    /// colour for outline geometry (ftMaterial_800BF6BC, `x2223_b2`).
    pub fn overlay(&self, joint: u32, display: usize) -> Option<[f32; 4]> {
        self.is_outline(joint, display).then(|| {
            self.outline
                .as_ref()
                .map_or([0.0; 4], |o| o.color.map(|v| f32::from(v) / 255.0))
        })
    }

    /// The costume material state of `joint`'s `display`th DObj, if the
    /// costume's own model has that joint (OnLoad's graft has none).
    pub fn material(&self, joint: u32, display: usize) -> Option<&hsd_anim::mobj::MObj> {
        let id = self.materials.ids.get(&joint)?;
        self.materials.tree.dobj(*id)?.get(display)?.mobj.as_ref()
    }

    /// `x5F4_arr[group].idx`: a script or move's ftParts_80074B0C selection,
    /// otherwise the `prev` value Fighter_ChangeMotionState restores.
    fn selection(&self, fighter: &melee_ft::fighter::Fighter, group: usize) -> i32 {
        if group >= self.group_count {
            return HIDDEN;
        }
        if let Some(selected) = fighter.commands.model_selections.get(&(group as i32)) {
            return *selected;
        }
        resting_selection(self.kind, self.costume, group, fighter)
    }
}

impl CostumeMaterials {
    fn new(
        character: &crate::assets::CharacterArchive,
        costume: u8,
        skeleton: &JObjTree,
        slots: &BTreeMap<(u32, usize), u8>,
        costume_textures: &[u16],
        body_color: Option<[u8; 4]>,
    ) -> Result<Self, PresentationError> {
        let archive = character.costume(costume);
        let (mut tree, root) =
            hsd_anim::load::load_joint_tree(archive, &character.costume_desc(costume))
                .map_err(error)?;
        if let Some(anim) = character
            .costume_material_animation(costume)
            .map_err(error)?
        {
            let anim = hsd_anim::load::material_animation(archive, &anim).map_err(error)?;
            tree.add_anim_all(root, None, Some(&anim));
        }
        let ids = tree.ids().map(|id| (tree.get(id).id, id)).collect();
        // ftParts_80075240: TObj `n` counts through dobj_list in slot order.
        let skeleton_ids: BTreeMap<u32, JObjId> =
            skeleton.ids().map(|id| (skeleton.get(id).id, id)).collect();
        let mut by_slot: Vec<_> = slots.iter().map(|(key, slot)| (*slot, *key)).collect();
        by_slot.sort_unstable();
        let mut all = Vec::new();
        for (_, (joint, display)) in by_slot {
            let count = skeleton_ids
                .get(&joint)
                .and_then(|id| skeleton.dobj(*id))
                .and_then(|dobjs| dobjs.get(display))
                .and_then(|dobj| dobj.mobj.as_ref())
                .map_or(0, |mobj| mobj.textures.len());
            all.extend((0..count).map(|texture| (joint, display, texture)));
        }
        let textures = costume_textures
            .iter()
            .map(|index| {
                all.get(usize::from(*index))
                    .copied()
                    .ok_or_else(|| error("can't find fighter texture anim"))
            })
            .collect::<Result<_, _>>()?;
        let mut materials = Self {
            tree,
            root,
            ids,
            textures,
        };
        materials.evaluate([].iter());
        if let Some([r, g, b, _]) = body_color {
            // ftMaterial_800BFB4C: every MObj of the model takes the colour.
            for id in materials.tree.ids().collect::<Vec<_>>() {
                for dobj in materials.tree.dobj_mut(id).into_iter().flatten() {
                    if let Some(mobj) = &mut dobj.mobj {
                        mobj.set_diffuse_color(r, g, b);
                    }
                }
            }
        }
        Ok(materials)
    }

    /// Evaluate the costume materials at this frame's texture requests.
    fn capture(&mut self, fighter: &melee_ft::fighter::Fighter) {
        self.evaluate(fighter.commands.texture_frames.iter());
    }

    /// ftAnim_80070308 requests every track at frame 0; ftAnim_800704F0
    /// requests costume TObj `index` at `frame` (its AObj rate is 0).
    fn evaluate<'a>(&mut self, requests: impl Iterator<Item = &'a (usize, f32)>) {
        self.tree.req_anim_all(self.root, 0.0);
        for (index, frame) in requests {
            let Some(&(joint, display, texture)) = self.textures.get(*index) else {
                continue;
            };
            let Some(id) = self.ids.get(&joint) else {
                continue;
            };
            if let Some(tobj) = self
                .tree
                .dobj_mut(*id)
                .and_then(|dobjs| dobjs.get_mut(display))
                .and_then(|dobj| dobj.mobj.as_mut())
                .and_then(|mobj| mobj.textures.get_mut(texture))
            {
                tobj.req_anim(*frame);
            }
        }
        self.tree
            .anim_all::<melee_ft::fighter::RetailTrig>(self.root);
    }
}

/// ftParts_80074B6C: in each group, the selected variant's DObjs show and
/// every other variant's hide.
fn apply_selection(
    groups: &Lookup,
    selected: &[i32; GROUP_CAPACITY],
    mut set: impl FnMut(usize, bool),
) {
    for (group, variants) in groups.iter().enumerate() {
        for (variant, dobjs) in variants.iter().enumerate() {
            for slot in dobjs {
                set(usize::from(*slot), variant as i32 == selected[group]);
            }
        }
    }
}

/// ftParts_80074A4C's and ftParts_80074B0C's "no variant" selection.
const HIDDEN: i32 = -1;

/// `x5F4_arr[group].prev` as each kind's OnDeath (run when the fighter
/// enters play) sets it; groups it never sets keep ftParts_800749CC's -1.
fn resting_selection(
    kind: FighterKind,
    costume: u8,
    group: usize,
    fighter: &melee_ft::fighter::Fighter,
) -> i32 {
    use FighterKind as K;
    let shown = |groups: &[usize]| if groups.contains(&group) { 0 } else { HIDDEN };
    match kind {
        // ftLk_Init_OnDeath (800EAD84), ftCl_Init_OnDeath (80148C64): groups
        // 0..=2. ftLk_Init_OnItemPickupExt / OnItemDropExt: an item in hand
        // moves the shield hand (group 2) to variant 1.
        K::Link | K::CLink => match group {
            2 if fighter.held_item.is_some() => 1,
            _ => shown(&[0, 1, 2]),
        },
        // ftPp_Init_OnDeath, ftNn_Init_OnDeath, ftPk_Init_OnDeath
        // (80124620), ftMs_Init_OnDeath, ftZd_Init_OnDeath (801392E8).
        K::Popo | K::Nana | K::Pikachu | K::Mars | K::Zelda => shown(&[0, 1]),
        // ftFe_Init_OnDeath (8014EEF8): (0, 0), (1, 0), (2, -1).
        K::Emblem => shown(&[0, 1]),
        // ftSk_Init_OnDeath (80110044), ftGn_Init_OnDeath: (0, 0), (1, -1).
        K::Seak | K::Ganon => shown(&[0]),
        // ftPe_Init_OnDeath (ftpeach.c:407-434): 0, 2 and 4 shown, 3
        // hidden; costume 1 shows group 5 instead of 1 and 6.
        K::Peach => match (costume, group) {
            (1, 5) | (1, 0 | 2 | 4) => 0,
            (1, _) => HIDDEN,
            (_, 0 | 1 | 2 | 4 | 6) => 0,
            _ => HIDDEN,
        },
        // ftPc_Init_OnDeath (80149EAC): costume 1..=3 shows that group.
        K::Pichu => match group {
            0 => 0,
            _ if group == usize::from(costume) => 0,
            _ => HIDDEN,
        },
        // ftGw_Init_OnDeath (ftgamewatch.c:493-503): 0, 2 and 3 shown.
        K::GameWatch => shown(&[0, 2, 3]),
        // Every other kind's OnDeath: ftParts_80074A4C(gobj, 0, 0).
        _ => shown(&[0]),
    }
}

/// ftData `x8`: `{ model_num, vis_table, tobj count, tobj lists }`.
/// ftParts_8007487C takes `vis_table[costume][set]`, or costume 0's entry
/// when null; ftAnim_80070200 takes `lists[costume]` the same way.
fn read_part_data(
    character: &crate::assets::CharacterArchive,
    costume: u8,
) -> Result<PartData, PresentationError> {
    let data = &*character.data;
    let r = data.reader();
    let link = |offset: u32| data.link(offset).map_err(error);
    let for_costume = |table: u32, stride: u32, field: u32| -> Result<_, PresentationError> {
        match link(table + u32::from(costume) * stride + field)? {
            Some(entry) => Ok(Some(entry)),
            None => link(table + field),
        }
    };
    let root = data
        .public(character.descriptor.data_symbol)
        .ok_or_else(|| error("fighter data symbol missing"))?;
    let mut result = PartData {
        group_count: 0,
        sets: Default::default(),
        costume_textures: Vec::new(),
    };
    let Some(parts) = link(root + 8)? else {
        return Ok(result);
    };
    result.group_count = r.u32(parts).map_err(error)? as usize;
    if result.group_count > GROUP_CAPACITY {
        return Err(error("fighter parts model num over"));
    }
    let texture_count = r.u32(parts + 8).map_err(error)?;
    if texture_count > COSTUME_TOBJ_CAPACITY {
        return Err(error("fighter tobj num over"));
    }
    if let Some(lists) = link(parts + 12)? {
        if let Some(list) = for_costume(lists, 4, 0)? {
            for i in 0..texture_count {
                result
                    .costume_textures
                    .push(r.u16(list + 2 * i).map_err(error)?);
            }
        }
    }
    let Some(table) = link(parts + 4)? else {
        return Ok(result);
    };
    for set in 0..SET_COUNT {
        if let Some(groups) = for_costume(table, 16, set as u32 * 4)? {
            result.sets[set] = Some(read_lookup(data, groups, result.group_count)?);
        }
    }
    Ok(result)
}

/// One `FtPartsVisLookup[group_count]` array: per group `{ variant count,
/// TempS* }`, per variant `{ DObj count, u8* dobj_list indices }`.
fn read_lookup(
    data: &Archive,
    groups: u32,
    group_count: usize,
) -> Result<Lookup, PresentationError> {
    let r = data.reader();
    let link = |offset: u32| data.link(offset).map_err(error);
    let mut lookup = Vec::with_capacity(group_count);
    for group in 0..group_count as u32 {
        let at = groups + group * 8;
        let mut variants = Vec::new();
        if let Some(list) = link(at + 4)? {
            for variant in 0..r.u32(at).map_err(error)? {
                let at = list + variant * 8;
                let mut dobjs = Vec::new();
                if let Some(indices) = link(at + 4)? {
                    for k in 0..r.u32(at).map_err(error)? {
                        let slot = r.u8(indices + k).map_err(error)?;
                        if usize::from(slot) >= DOBJ_CAPACITY {
                            return Err(error("fighter part DObj index over"));
                        }
                        dobjs.push(slot);
                    }
                }
                variants.push(dobjs);
            }
        }
        lookup.push(variants);
    }
    Ok(lookup)
}

/// ftParts_SetupParts / ftParts_80074194: DObjs numbered in the joint
/// walk's pre-order (an instanced joint's children are not visited). The
/// OnLoad graft (ftParts_800753D4) runs later with its own counter from 0,
/// so its DObjs overwrite the list's first entries.
struct Walk<'a> {
    /// The grafted joint's pre-order index (its conditional part).
    graft: Option<usize>,
    joint_index: usize,
    next: usize,
    slots: &'a mut BTreeMap<(u32, usize), u8>,
    grafted: &'a mut Vec<((u32, usize), u8)>,
}
impl Walk<'_> {
    fn number(&mut self, joint: &JObjDesc) -> Result<(), PresentationError> {
        let grafted = self.graft == Some(self.joint_index);
        self.joint_index += 1;
        let mut display = 0;
        let mut dobj = joint.u.dobj();
        while let Some(current) = dobj {
            if grafted {
                self.grafted.push(((joint.offset, display), display as u8));
            } else {
                if self.next >= DOBJ_CAPACITY {
                    return Err(error("fighter dobj num over"));
                }
                self.slots.insert((joint.offset, display), self.next as u8);
                self.next += 1;
            }
            dobj = current.next.as_deref();
            display += 1;
        }
        if !joint.is_instance() {
            if let Some(child) = joint.child.as_deref() {
                self.number(child)?;
            }
        }
        if let Some(sibling) = joint.next.as_deref() {
            self.number(sibling)?;
        }
        Ok(())
    }
}
