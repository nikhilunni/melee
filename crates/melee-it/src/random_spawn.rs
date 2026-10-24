//! Random items in a match, itspawn.c (8026C47C..8026D3CC): the weighted
//! pick tables match setup builds from the item switch and the stage's item
//! weights (it_8026D018), and the countdown the spawner GObj's proc
//! (fn_8026C88C) runs each tick. Where an item lands and the item itself
//! belong to the scene, which owns the stage markers, the collision map and
//! the item pool.
use gekko_math::{fma::fmadds, msl::fctiwz, HsdRng};
use melee_types::ItemKind;

/// The kinds a pick table can hold: every kind below It_Kind_L_Gun_Ray
/// (it_8026CA4C's loop bound, 0x23).
pub const PICKABLE_KINDS: usize = 0x23;
/// it_8026CD50: the container contents table starts at It_Kind_BombHei.
const FIRST_CONTENT_KIND: usize = 6;
/// it_8026CF04: the monster table's four kinds from It_Kind_Kuriboh.
const MONSTER_KINDS: usize = 4;
/// `+ 0.99f` (it_8026CA4C, it_8026CB9C): each weight rounds up.
const WEIGHT_ROUNDING: f32 = 0.99;

/// The match's item rules (StartMeleeRules xB and x20, gm_8016AE80 and
/// gm_8016AEA4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemRules {
    /// The Item Switch frequency: -1 off, 0 Very Low .. 4 Very High.
    pub frequency: i8,
    /// Bit `kind` set: that item kind may spawn.
    pub mask: u64,
}
impl ItemRules {
    /// Items off: what every VS rule set without the Item Switch holds
    /// (gm_1601.c:3534, every mask bit set).
    pub const OFF: Self = Self {
        frequency: -1,
        mask: u64::MAX,
    };
    pub fn enabled(&self) -> bool {
        self.frequency >= 0
    }
    /// The Item Switch frequencies the spawner reads interval ranges for.
    pub fn validate(&self) -> Result<(), String> {
        if !(-1..=4).contains(&self.frequency) {
            return Err(format!("item frequency {} is not -1..=4", self.frequency));
        }
        Ok(())
    }
}
impl Default for ItemRules {
    fn default() -> Self {
        Self::OFF
    }
}

/// What the stage gives the spawner: `stage_info.xA0` (Ground_801C28CC: the
/// ground parameters' weight times the stage row's, per kind) and the spawn
/// interval scale (Ground_801C2AE8).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StageItemRates {
    pub weights: [i32; PICKABLE_KINDS],
    pub interval_scale: f32,
}

/// ItemPickTable (it/types.h): the kinds a draw can pick and each one's
/// cumulative weight threshold, with the total as Randi's bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PickTable {
    len: usize,
    kinds: [u8; PICKABLE_KINDS],
    thresholds: [u16; PICKABLE_KINDS],
    /// x8: stored as a halfword (it_8026CA4C's `sth`).
    total: u16,
}
impl Default for PickTable {
    fn default() -> Self {
        Self {
            len: 0,
            kinds: [0; PICKABLE_KINDS],
            thresholds: [0; PICKABLE_KINDS],
            total: 0,
        }
    }
}
impl PickTable {
    pub fn total(&self) -> u16 {
        self.total
    }
    pub fn kinds(&self) -> impl Iterator<Item = ItemKind> + '_ {
        self.kinds[..self.len].iter().map(|&k| kind(k))
    }
    /// it_8026CA4C (8026CA4C): the total of the kinds from `first` whose
    /// `mask` bit is set (bit 0 is `first`), each weight scaled and rounded
    /// up; a zero weight adds nothing.
    fn sum(weights: &[i32; PICKABLE_KINDS], mask: u64, first: usize, scale: f32) -> u16 {
        let mut sum: i32 = 0;
        for (index, &weight) in weights.iter().enumerate().skip(first) {
            if mask >> (index - first) & 1 != 0 {
                sum = add_weight(sum, weight, scale);
            }
        }
        sum as u16
    }
    /// it_8026CB9C / it_8026CD50: the kinds from `first` that are switched
    /// on and weigh something, each with the running total before it.
    fn build(weights: &[i32; PICKABLE_KINDS], mask: u64, first: usize, scale: f32) -> Self {
        let mut table = Self::default();
        let mut cumulative: i32 = 0;
        for (index, &weight) in weights.iter().enumerate().skip(first) {
            if mask >> (index - first) & 1 != 0 && weight != 0 {
                table.kinds[table.len] = index as u8;
                // sthx: the threshold keeps the low halfword.
                table.thresholds[table.len] = cumulative as u16;
                table.len += 1;
                cumulative = add_weight(cumulative, weight, scale);
            }
        }
        table
    }
    /// bisectValue (8026C530): the entry whose threshold range holds `value`.
    fn bisect(&self, value: i32, lo: usize, hi: usize) -> usize {
        if lo == hi - 1 {
            return lo;
        }
        let mid = (lo + hi) / 2;
        if i32::from(self.thresholds[mid]) > value {
            self.bisect(value, lo, mid)
        } else if i32::from(self.thresholds[mid + 1]) > value {
            mid
        } else {
            self.bisect(value, mid, hi)
        }
    }
    /// it_8026C65C (8026C65C): one weighted draw.
    fn draw(&self, rng: &mut HsdRng) -> ItemKind {
        let value = rng.randi(i32::from(self.total));
        kind(self.kinds[self.bisect(value, 0, self.len)])
    }
    /// it_8026C75C (8026C75C): a kind for a new item, or `None`. Once the
    /// Poké Balls are at their limit, or Poké Balls are switched off, a
    /// table ending in one draws from the entries before it.
    pub fn pick(&self, poke_balls_barred: bool, rng: &mut HsdRng) -> Option<ItemKind> {
        if self.total == 0 {
            return None;
        }
        let mut table = *self;
        if poke_balls_barred && self.len > 0 && kind(self.kinds[self.len - 1]) == ItemKind::MBall {
            let last = self.len - 1;
            if last < 1 {
                return None;
            }
            table.total = self.thresholds[last];
            table.len = last;
        }
        Some(table.draw(rng))
    }
}

/// `sum + (scale * weight + 0.99f)`, truncated: retail 8026CADC / 8026CCF4
/// fmadds, then fadds and fctiwz.
fn add_weight(sum: i32, weight: i32, scale: f32) -> i32 {
    fctiwz(sum as f32 + fmadds(scale, weight as f32, WEIGHT_ROUNDING))
}

fn kind(value: u8) -> ItemKind {
    ItemKind::try_from(i32::from(value)).expect("a pick table holds item kinds")
}

/// it_804A0E30 (RandomItemSpawner) with it_804A0E50 and it_804A0E60, the
/// state it_8026D018 sets up for a match with items on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RandomItemSpawner {
    /// x0: ticks until the next spawn.
    pub timer: i32,
    /// x4: what spawns from thin air.
    pub items: PickTable,
    /// x18: the match's item mask.
    pub mask: u64,
    /// it_804A0E50: what a container releases (it_8026F3AC), kinds from
    /// Bob-omb on.
    pub contents: PickTable,
    /// it_804A0E60: the monster table (it_8026CF04).
    pub monsters: PickTable,
    frequency: i8,
    interval_scale: f32,
}

impl RandomItemSpawner {
    /// it_8026D018 (8026D018): the spawner for a match, or `None` (Sudden
    /// Death, items off, or nothing on the switch the stage weighs). Only a
    /// created spawner draws: its first interval.
    pub fn create(
        rules: ItemRules,
        sudden_death: bool,
        stage: &StageItemRates,
        // gm_8016AE94: lbl_8046B6A0.unk_34 (get_unk_float), by player count.
        weight_scale: f32,
        monster_weights: &[i32; MONSTER_KINDS],
        intervals: &[[i32; 2]; 5],
        rng: &mut HsdRng,
    ) -> Option<Self> {
        if sudden_death || !rules.enabled() || rules.mask == 0 {
            return None;
        }
        let total = PickTable::sum(&stage.weights, rules.mask, 0, weight_scale);
        if total == 0 {
            return None;
        }
        let mut items = PickTable::build(&stage.weights, rules.mask, 0, weight_scale);
        items.total = total;
        let content_mask = rules.mask >> FIRST_CONTENT_KIND;
        let mut contents = PickTable::default();
        let content_total =
            PickTable::sum(&stage.weights, content_mask, FIRST_CONTENT_KIND, weight_scale);
        if content_total != 0 {
            contents =
                PickTable::build(&stage.weights, content_mask, FIRST_CONTENT_KIND, weight_scale);
        }
        contents.total = content_total;
        let mut spawner = Self {
            timer: 0,
            items,
            mask: rules.mask,
            contents,
            monsters: monster_table(monster_weights),
            frequency: rules.frequency,
            interval_scale: stage.interval_scale,
        };
        spawner.timer = spawner.interval(intervals, rng);
        Some(spawner)
    }
    /// The spawner as a saved match holds it: its tables rebuilt from the
    /// rules (they are pure), its countdown read from memory.
    pub fn restore(
        rules: ItemRules,
        stage: &StageItemRates,
        weight_scale: f32,
        monster_weights: &[i32; MONSTER_KINDS],
        timer: i32,
    ) -> Option<Self> {
        // Any seed: create's one draw is replaced by the saved countdown.
        let mut rng = HsdRng::default();
        let intervals = [[1, 1]; 5];
        let mut spawner = Self::create(
            rules,
            false,
            stage,
            weight_scale,
            monster_weights,
            &intervals,
            &mut rng,
        )?;
        spawner.timer = timer;
        Some(spawner)
    }
    /// it_8026D324 (8026D324): whether `kind` is switched on.
    pub fn switched_on(&self, kind: ItemKind) -> bool {
        self.mask >> i32::from(kind) & 1 != 0
    }
    /// fn_8026C88C's countdown: true on the tick a spawn is due.
    pub fn count_down(&mut self) -> bool {
        self.timer -= 1;
        self.timer == 0
    }
    /// The next interval (fn_8026C88C's tail, it_8026D018's): a draw in the
    /// frequency's range (ItCo +FC), scaled by the stage.
    pub fn restart(&mut self, intervals: &[[i32; 2]; 5], rng: &mut HsdRng) {
        self.timer = self.interval(intervals, rng);
    }
    fn interval(&self, intervals: &[[i32; 2]; 5], rng: &mut HsdRng) -> i32 {
        let [low, high] = intervals[self.frequency as usize];
        let random = rng.randf();
        // retail 8026C9E4 / 8026D1F4: fmadds.
        let frames = fctiwz(fmadds((high - low) as f32, random, low as f32));
        fctiwz(frames as f32 * self.interval_scale)
    }
}

/// it_8026CF04 (8026CF04): the four monsters' table, unscaled.
fn monster_table(weights: &[i32; MONSTER_KINDS]) -> PickTable {
    let mut table = PickTable::default();
    let sum: i32 = weights.iter().sum();
    if sum == 0 {
        return table;
    }
    table.total = sum as u16;
    let mut cumulative: i32 = 0;
    for (index, &weight) in weights.iter().enumerate() {
        table.kinds[index] = (i32::from(ItemKind::Kuriboh) + index as i32) as u8;
        table.thresholds[index] = cumulative as u16;
        cumulative += weight;
    }
    table.len = MONSTER_KINDS;
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rates(weights: &[(ItemKind, i32)]) -> StageItemRates {
        let mut table = [0; PICKABLE_KINDS];
        for &(kind, weight) in weights {
            table[i32::from(kind) as usize] = weight;
        }
        StageItemRates {
            weights: table,
            interval_scale: 1.0,
        }
    }

    #[test]
    fn a_switched_off_or_weightless_kind_is_left_out_of_the_table() {
        let stage = rates(&[(ItemKind::Capsule, 10), (ItemKind::Sword, 5), (ItemKind::Bat, 7)]);
        let rules = ItemRules {
            frequency: 4,
            mask: 1 << i32::from(ItemKind::Sword) | 1 << i32::from(ItemKind::Tomato),
        };
        let mut rng = HsdRng::default();
        let spawner = RandomItemSpawner::create(
            rules,
            false,
            &stage,
            1.0,
            &[0; 4],
            &[[100, 200]; 5],
            &mut rng,
        )
        .unwrap();
        assert_eq!(spawner.items.kinds().collect::<Vec<_>>(), [ItemKind::Sword]);
        // 1.0 * 5 + 0.99, truncated.
        assert_eq!(spawner.items.total(), 5);
        // One draw: the first interval.
        let mut once = HsdRng::default();
        once.rand();
        assert_eq!(rng, once);
    }

    #[test]
    fn items_off_or_sudden_death_create_no_spawner_and_draw_nothing() {
        let stage = rates(&[(ItemKind::Sword, 5)]);
        let mut rng = HsdRng::default();
        for (rules, sudden_death) in [
            (ItemRules::OFF, false),
            (ItemRules { frequency: 4, mask: u64::MAX }, true),
            (ItemRules { frequency: 4, mask: 0 }, false),
        ] {
            assert!(RandomItemSpawner::create(
                rules,
                sudden_death,
                &stage,
                1.0,
                &[0; 4],
                &[[1, 2]; 5],
                &mut rng
            )
            .is_none());
        }
        assert_eq!(rng, HsdRng::default());
    }

    #[test]
    fn a_draw_lands_in_the_entry_whose_threshold_range_holds_it() {
        let stage = rates(&[(ItemKind::Capsule, 1), (ItemKind::Box, 2), (ItemKind::Sword, 3)]);
        let table = PickTable::build(&stage.weights, u64::MAX, 0, 1.0);
        // Thresholds 0, 1, 3: values 0 / 1..=2 / 3..
        let cases = [(0, 0), (1, 1), (2, 1), (3, 2), (5, 2)];
        for (value, entry) in cases {
            assert_eq!(table.bisect(value, 0, table.len), entry, "value {value}");
        }
    }
}
