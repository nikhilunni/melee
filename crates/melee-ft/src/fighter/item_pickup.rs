//! Items in reach and in hand: ftpickupitem.c (80094150..80094E2C) and the
//! LightGet state.
//!
//! The scene offers the grabbable items to the fighter procs that can pick
//! one up (input and animation); the search is retail's box test over them.
//! A pickup asks the scene to attach the item (Item_8026AB54) right after the
//! proc. While a fighter holds an item only audited motion states may run:
//! every other retail path that branches on `item_gobj` is unported and
//! fails closed at the motion change.
use super::{
    assets::{FighterAssets, Result},
    Fighter, FighterCore, MotionData,
};
use gekko_math::fma::fmadds;
use hsd_types::Vec2;
use melee_types::{fixed::FixedVec, CommonMotionState as S, GroundOrAir};

/// One pickup box of ftData x40 (itPickup): a centre offset (x forward) and
/// half extents.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PickupBox {
    pub offset: Vec2,
    pub half_extent: Vec2,
}

/// ftData x40 (itPickup, copied to fp+294): where a fighter can reach items.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PickupBoxes {
    pub ground_light: PickupBox,
    pub ground_heavy: PickupBox,
    pub air_light: PickupBox,
}
impl PickupBoxes {
    pub fn read(archive: &hsd_archive::Archive, offset: u32) -> hsd_archive::desc::Result<Self> {
        let r = archive.reader();
        let read_box = |at: u32| -> hsd_archive::desc::Result<PickupBox> {
            Ok(PickupBox {
                offset: Vec2::new(r.f32(at)?, r.f32(at + 4)?),
                half_extent: Vec2::new(r.f32(at + 8)?, r.f32(at + 12)?),
            })
        };
        Ok(Self {
            ground_light: read_box(offset)?,
            ground_heavy: read_box(offset + 0x10)?,
            air_light: read_box(offset + 0x20)?,
        })
    }
}

/// A grabbable item (Item_IsGrabbable) as the pickup search sees it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PickupCandidate {
    pub item: u32,
    /// it_8026B344: the item's pickup point.
    pub position: Vec2,
    /// itGetGrabRangeX / Y.
    pub range: Vec2,
    /// itIsHeavy.
    pub heavy: bool,
    /// it_8026B4F0: food and coins, which a fighter can take with a full hand.
    pub consumable: bool,
    /// ItemAttr x0 bits 0x78 and 0x07 (it_8026B30C, itGetHoldKind).
    pub use_kind: u8,
    pub hand_hold_kind: u8,
    /// ItemAttr x1C (itGetDamageMultiplier).
    pub damage_multiplier: f32,
}

/// The items offered to the current proc, in the item list's order.
#[derive(Clone, Debug, Default)]
pub struct PickupCandidates {
    offered: bool,
    items: FixedVec<PickupCandidate, { melee_it::ITEM_CAPACITY }>,
}
impl PickupCandidates {
    pub fn offer(&mut self, items: impl Iterator<Item = PickupCandidate>) {
        self.items.clear();
        for item in items {
            self.items.push(item);
        }
        self.offered = true;
    }
    pub fn withdraw(&mut self) {
        self.offered = false;
        self.items.clear();
    }
}

/// ftpickupitem_800942A0's flag bits: which weights the search accepts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PickupWeights {
    pub light: bool,
    pub heavy: bool,
}
impl PickupWeights {
    pub const LIGHT: Self = Self {
        light: true,
        heavy: false,
    };
    pub const HEAVY: Self = Self {
        light: false,
        heavy: true,
    };
    pub const ANY: Self = Self {
        light: true,
        heavy: true,
    };
}

/// fp->item_gobj (+1974): the item in the fighter's hand.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HeldItem {
    pub item: u32,
    pub heavy: bool,
    pub hand_hold_kind: u8,
    /// it_8026B30C: 0 throws, 2 swings, 3 shoots.
    pub use_kind: u8,
    /// ItemAttr x1C (itGetDamageMultiplier).
    pub damage_multiplier: f32,
}

/// Motion states audited for a held item. Retail branches on `item_gobj` in
/// about forty files; a held item entering any other state is unported.
const HELD_ITEM_STATES: [S; 79] = [
    S::LightGet,
    S::Wait,
    // ftCo_AppealS: the taunt never reads the item; its IASA throws it.
    S::AppealSR,
    S::AppealSL,
    // ftCo_Walk: a light item changes nothing but the IASA's item throw.
    S::WalkSlow,
    S::WalkMiddle,
    S::WalkFast,
    // Guard (ftCo_Guard): the shield keeps the item; A throws it.
    S::GuardOn,
    S::Guard,
    S::GuardOff,
    S::GuardSetOff,
    S::GuardReflect,
    // Dashing and running keep a light item; A there is a dash throw.
    S::Dash,
    S::Run,
    S::RunBrake,
    // ftCo_TurnRun_IASA only offers the running jump (fn_800CAF78).
    S::TurnRun,
    S::LightThrowDash,
    // Turning, crouching and rolls keep it too.
    S::Turn,
    S::Squat,
    S::SquatWait,
    S::SquatRv,
    S::EscapeF,
    S::EscapeB,
    S::EscapeN,
    // ftCo_80095A30's ground throws: the item leaves at the release flag.
    S::LightThrowF,
    S::LightThrowB,
    S::LightThrowHi,
    S::LightThrowLw,
    S::LightThrowF4,
    S::LightThrowB4,
    S::LightThrowHi4,
    S::LightThrowLw4,
    // A jump while holding, and ftCo_80095328's air throws.
    S::KneeBend,
    S::JumpF,
    S::JumpB,
    // ftCo_JumpAerial_IASA runs the same item throw and catch checks.
    S::JumpAerialF,
    S::JumpAerialB,
    S::Fall,
    S::FallF,
    S::FallB,
    // ftCo_Damage / DamageFly / DamageFall: no item branch; a launch rolls
    // Fighter_8006CDA4's drop first, and after hitstun the IASA throws.
    S::DamageHi1,
    S::DamageHi2,
    S::DamageHi3,
    S::DamageN1,
    S::DamageN2,
    S::DamageN3,
    S::DamageLw1,
    S::DamageLw2,
    S::DamageLw3,
    S::DamageAir1,
    S::DamageAir2,
    S::DamageAir3,
    S::DamageFlyHi,
    S::DamageFlyN,
    S::DamageFlyLw,
    S::DamageFlyTop,
    S::DamageFlyRoll,
    S::DamageFall,
    // ftCliffCommon_80081370 and the ftCo_Cliff* states: no item branch;
    // a fighter hangs, climbs, attacks, rolls and jumps holding it.
    S::CliffCatch,
    S::CliffWait,
    S::CliffClimbSlow,
    S::CliffClimbQuick,
    S::CliffAttackSlow,
    S::CliffAttackQuick,
    S::CliffEscapeSlow,
    S::CliffEscapeQuick,
    S::CliffJumpSlow1,
    S::CliffJumpSlow2,
    S::CliffJumpQuick1,
    S::CliffJumpQuick2,
    // ftCo_Landing: no item branch; its IASA sees the item like Wait's.
    S::Landing,
    S::LightThrowAirF,
    S::LightThrowAirB,
    S::LightThrowAirHi,
    S::LightThrowAirLw,
    S::LightThrowAirF4,
    S::LightThrowAirB4,
    S::LightThrowAirHi4,
    S::LightThrowAirLw4,
];

/// ftCo_SM_Wait1_1, the idle animation while holding an item: ft_8008A348
/// names it by the equal enum value ftCo_MS_DeadUpFall (6).
const WAIT_HOLDING_ITEM_ANIMATION: i32 = 6;

/// efAsync_Spawn 0x422, the sparkle of an aerial item catch.
const ITEM_PICKUP_SPARKLE: u16 = 0x422;

/// ftpickupitem_800942A0's starting best squared distance: an item farther
/// from the box centre is never chosen.
const PICKUP_SEARCH_LIMIT: f32 = 30000.0;

impl FighterCore {
    /// ft_8008A348 after its Wait entry: with an item held in any hand pose
    /// but 2, the kind's item idle (ftCo_8008A698 -> ftCo_8008A6D8).
    pub(super) fn play_wait_holding_idle(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.held_item.is_some_and(|held| held.hand_hold_kind != 2)
            && assets.motions.contains_key(&WAIT_HOLDING_ITEM_ANIMATION)
        {
            self.play_idle_animation(assets, WAIT_HOLDING_ITEM_ANIMATION)?;
        }
        Ok(())
    }
    /// The ground IASA context: facing, specials, shield and the held item.
    pub fn wait_context(&self) -> crate::input::WaitContext {
        crate::input::WaitContext {
            facing: self.physics.facing,
            specials_available: self.capabilities.specials,
            shield_health: self.status.shield_health,
            held_item: self.held_item.map(|held| held.use_kind == 0),
            ..Default::default()
        }
    }
    /// ftpickupitem_800942A0 (800942A0): the nearest offered item whose
    /// pickup range overlaps the fighter's pickup box.
    pub fn find_pickup(
        &self,
        boxes: &PickupBoxes,
        weights: PickupWeights,
    ) -> Option<PickupCandidate> {
        assert!(
            self.pickup_candidates.offered,
            "ftpickupitem_800942A0: the scene offered no items to this proc"
        );
        // ftCo_800A2040 && cpu.xC == 28 only concerns CPU fighters. x2222_b4
        // (a heal restriction) is never set for the supported kinds.
        let light = if self.physics.ground_or_air == GroundOrAir::Ground {
            boxes.ground_light
        } else {
            boxes.air_light
        };
        let position = self.physics.position;
        let facing = self.physics.facing;
        let mut nearest = PICKUP_SEARCH_LIMIT;
        let mut result = None;
        for candidate in self.pickup_candidates.items.iter() {
            if self.held_item.is_some() && !candidate.consumable {
                continue;
            }
            let wanted = if candidate.heavy {
                weights.heavy
            } else {
                weights.light
            };
            if !wanted {
                continue;
            }
            let reach = if candidate.heavy {
                boxes.ground_heavy
            } else {
                light
            };
            let (x_range, y_range) = (candidate.range.x, candidate.range.y);
            let item = candidate.position;
            // retail 800943D4: fmadds.
            let x0 = fmadds(facing, reach.offset.x, position.x);
            let y0 = position.y + reach.offset.y;
            let (x1, y1) = (reach.half_extent.x, reach.half_extent.y);
            if x0 - x1 - x_range < item.x
                && x_range + (x0 + x1) > item.x
                && y0 - y1 - y_range < item.y
                && y_range + (y0 + y1) > item.y
            {
                let y_diff = item.y - y0;
                let x_diff = item.x - x0;
                // retail 80094438..3C: fmuls, then fmadds.
                let distance = fmadds(x_diff, x_diff, y_diff * y_diff);
                if distance < nearest {
                    result = Some(*candidate);
                    nearest = distance;
                }
            }
        }
        result
    }

    /// fp->item_gobj is released: Item_8026A848 -> ftCommon_8007E6DC.
    pub fn release_held_item(&mut self, item: u32, assets: &FighterAssets) {
        let held = self.held_item.take().expect("released item was held");
        assert_eq!(held.item, item, "ftLib_800867A0: released another item");
        // ftCo_800C5240 is the hammer; OnItemDropExt: Fighter_OnItemDrop
        // (ft/inlines.h:188); ftLib_80086724 passes drop flag 1, so the shown
        // slot's live hand animation is removed too (ftAnim_80070CC4).
        let hand = assets
            .item_hand
            .expect("ftData_OnItemDropExt for this kind");
        // ftAnim_80070FB4(pose, -1); the pose stays applied until the next
        // motion change reinstalls the (now empty) selection.
        self.animation.part_animations[hand.pose].previous = -1;
        if self.animation.part_animations[hand.shown].current != -1 {
            unimplemented!("ftAnim_80070CC4: removing a live hand animation");
        }
    }

    /// Keep a held item inside the audited states (see HELD_ITEM_STATES).
    /// Character special rows pass `special` with the table's
    /// `specials_keep_held_item` audit instead.
    pub(super) fn require_held_item_state(&self, state: S, special: Option<bool>) {
        let audited = match special {
            Some(keeps_item) => keeps_item,
            None => HELD_ITEM_STATES.contains(&state),
        };
        if self.held_item.is_some() && !audited {
            unimplemented!("holding an item: motion state {state:?} is not audited");
        }
    }
}

impl FighterCore {
    /// The holder lent to its held item's callbacks: the part's joint
    /// (ftLib_80086630), the ECB centre and the current attack.
    pub fn item_holder(&mut self, part: u8) -> melee_it::ItemHolder<'_> {
        let joint = self.animation.parts[usize::from(part)].joint;
        // ftLib_80086990: vector_add(v, &cur_pos, 0, 0.5 * (top + bottom), 0),
        // three fadds (retail 800869A4..C8).
        let ecb = &self.collision.data.ecb;
        let position = self.physics.position;
        let center = hsd_types::Vec3::new(
            position.x + 0.0,
            position.y + 0.5 * (ecb.top.y + ecb.bottom.y),
            position.z + 0.0,
        );
        melee_it::ItemHolder {
            skeleton: &mut self.skeleton,
            part: joint,
            center,
            attack: self.combat.stale.attack(),
        }
    }
}

impl Fighter {
    /// ftpickupitem_80094790 (80094790), before a jab or tilt: pick up an
    /// item in reach instead. True when LightGet or HeavyGet began.
    pub(super) fn try_item_pickup(&mut self, assets: &FighterAssets) -> Result<bool> {
        // x1978, the second (consumable) hand, is never filled here.
        let Some(item) = self.core.find_pickup(&assets.pickup, PickupWeights::ANY) else {
            return Ok(false);
        };
        if item.heavy {
            unimplemented!("ftpickupitem_80094694: HeavyGet");
        }
        self.enter_item_get(S::LightGet, assets)?;
        Ok(true)
    }

    /// ftpickupitem_80094694 (80094694) without its loop mode: clear the
    /// throw flags, enter the state and play its first frame.
    fn enter_item_get(&mut self, state: S, assets: &FighterAssets) -> Result<()> {
        self.core.commands.take_throw_flag_b3();
        self.change_motion_state(state.into(), assets)?;
        self.step_animation(assets);
        // mv.co.itemget.x0; take_dmg_cb = ftpickupitem_80094DF8 only
        // concerns consumables.
        self.core.state_data = MotionData::ItemGet {
            heavy: state == S::HeavyGet,
        };
        Ok(())
    }

    /// ftpickupitem_Anim (80094A14): the script's throw flag closes the
    /// hand on the nearest item; the animation ends in Wait.
    pub(super) fn item_get_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::ItemGet { heavy } = self.core.state_data else {
            panic!("item get scratch")
        };
        if self.core.commands.take_throw_flag_b3() {
            let weights = if heavy {
                PickupWeights::HEAVY
            } else {
                PickupWeights::LIGHT
            };
            if let Some(item) = self.core.find_pickup(&assets.pickup, weights) {
                self.take_item(item, assets);
            }
        }
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            if let Some(held) = self.core.held_item {
                assert!(!held.heavy, "ftCo_80096D9C: HeavyWait");
            }
            // ftpickupitem_8009447C: kinds with use kind 5 are consumed, and
            // the Warp Star and hammer take over; none is a supported kind.
            self.enter_wait_holding(assets)?;
        }
        Ok(())
    }

    /// ft_8008A2BC -> ft_8008A348 (8008A348) after a pickup: Wait, and with
    /// an item held in any hand pose but 2 the kind's item idle
    /// (ftCo_8008A698 -> ftCo_8008A6D8) when it has one. DownSpot
    /// (x2224_b2) and the hammer (ftCo_800C5240) cannot follow LightGet.
    fn enter_wait_holding(&mut self, assets: &FighterAssets) -> Result<()> {
        self.change_motion_state(S::Wait.into(), assets)
    }

    /// ftCo_800D7100 (800D7100): LR held and A pressed in the air, with an
    /// empty hand, no catch yet this airtime (x2224_b1) and the previous A
    /// press at least PlCo +1C frames old, catches the nearest light item in
    /// reach (fn_800D6F58). True when it caught one; the IASA then returns.
    pub(super) fn try_aerial_item_catch(&mut self, assets: &FighterAssets) -> bool {
        use crate::input::Buttons;
        let core = &self.core;
        if core.held_item.is_some()
            || !core.input.current.held.intersects(Buttons::SHIELD)
            || !core.input.pressed.intersects(Buttons::A)
            || core.item_catch_locked
            || i32::from(core.input.buttons.previous_attack) < assets.damage.tech_lockout
        {
            return false;
        }
        let Some(item) = core.find_pickup(&assets.pickup, PickupWeights::LIGHT) else {
            return false;
        };
        self.catch_item(item, assets);
        true
    }

    /// fn_800D6F58 (800D6F58): the hand closes on `item` with no motion
    /// change (ftpickupitem_800948A8's light path), then ftpickupitem_8009447C
    /// (nothing for a thrown kind), the pickup sparkle (efAsync_Spawn 0x422
    /// at the light item part) and x2224_b1.
    fn catch_item(&mut self, item: PickupCandidate, assets: &FighterAssets) {
        self.take_item(item, assets);
        let part = usize::from(self.core.bones.model.animation_translation);
        let joint = self.core.animation.parts[part].joint.0;
        self.core
            .effects
            .push(melee_ef::request::EffectRequest::Attached {
                id: ITEM_PICKUP_SPARKLE,
                bone: joint,
            });
        self.core.item_catch_locked = true;
    }

    /// ftpickupitem_800948A8 (800948A8): the hand closes on `item`.
    fn take_item(&mut self, item: PickupCandidate, assets: &FighterAssets) {
        assert!(self.core.held_item.is_none(), "x1978: a second held item");
        assert_ne!(item.use_kind, 5, "ftpickupitem_8009447C: consumable pickup");
        self.core.held_item = Some(HeldItem {
            item: item.item,
            heavy: item.heavy,
            hand_hold_kind: item.hand_hold_kind,
            use_kind: item.use_kind,
            damage_multiplier: item.damage_multiplier,
        });
        // ftpickupitem_80094818(gobj, true) -> ftData_OnItemPickupExt:
        // Fighter_OnItemPickup (ft/inlines.h:143).
        let hand = assets
            .item_hand
            .expect("ftData_OnItemPickupExt for this kind");
        let pose = match item.hand_hold_kind {
            1 => Some(1),
            2 => Some(0),
            3 => Some(2),
            4 => Some(3),
            _ => None,
        };
        if let Some(pose) = pose {
            // ftAnim_80070FB4.
            self.core.animation.part_animations[hand.pose].previous = pose;
        }
        // ftAnim_80070C48: apply the shown slot's selection.
        let shown = self.core.animation.part_animations[hand.shown].previous;
        if shown != -1 {
            self.core
                .apply_part_animation(assets, hand.shown, shown as usize, 0.0);
        }
        // Item_8026AB54 at the light item part (ftData x8 +0x10).
        self.core.item_requests.push(melee_it::ItemRequest::PickUp {
            item: item.item,
            part: self.core.bones.model.animation_translation,
        });
    }

    /// ftpickupitem_Coll (80094B44): ft_800841B8, whose ground loss drops
    /// the item and falls (ftpickupitem_80094D90).
    pub(super) fn item_get_collision(&mut self, map: &mut melee_mp::CollMap) {
        use crate::collision::ground::{map_escape, WaitGroundResult};
        if map_escape(
            &mut self.core.physics,
            &mut self.core.collision,
            map,
            &mut self.core.skeleton,
            self.core.animation.root,
            self.core.input.current.stick.x,
        ) == WaitGroundResult::EnterFall
        {
            unimplemented!("ftpickupitem_80094D90: LightGet leaving the ground");
        }
    }
}
