//! Items in reach and in hand: ftpickupitem.c (80094150..80094E2C) and the
//! LightGet and HeavyGet states.
//!
//! The scene offers the grabbable items to the fighter procs that can pick
//! one up (input and animation); the search is retail's box test over them.
//! A pickup asks the scene to attach the item (Item_8026AB54) right after the
//! proc. While a fighter holds an item only audited motion states may run:
//! every other retail path that branches on `item_gobj` is unported and
//! fails closed at the motion change.
use super::{
    assets::{FighterAssets, Result},
    ActionId, Fighter, FighterCore, MotionData,
};
use gekko_math::fma::fmadds;
use hsd_types::{Vec2, Vec3};
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
    /// itGetKind.
    pub kind: melee_types::ItemKind,
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
    /// xD4C: the shots left (it_8026B594).
    pub ammo: i32,
    /// it_80292394 / it_80295748: where a shot leaves the item, in the hand
    /// joint's frame (the Star Rod's and Lip's Stick's attributes x4).
    pub shot_offset: Vec3,
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
    /// itGetKind(fp->item_gobj).
    pub kind: melee_types::ItemKind,
    pub heavy: bool,
    pub hand_hold_kind: u8,
    /// it_8026B30C: 0 throws, 2 swings, 3 shoots.
    pub use_kind: u8,
    /// ItemAttr x1C (itGetDamageMultiplier).
    pub damage_multiplier: f32,
    /// xD4C as the hand holds it: the holder's shots count it down with
    /// the item's (it_802923BC); it_8026B594 tests it.
    pub ammo: i32,
    /// The candidate's `shot_offset`.
    pub shot_offset: Vec3,
}

/// fp->item_gobj holding one of the fighter's own articles (Peach's
/// parasol) rather than a picked-up item: only the launch drop
/// (Fighter_8006CDA4) and the parasol status read it here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArticleInHand {
    pub kind: melee_types::ItemKind,
    /// fp->parts index the article hangs from (Item_8026AB54's part).
    pub part: usize,
    /// it_8026B30C: 0 throws, 2 swings, 3 shoots.
    pub use_kind: u8,
}

/// Motion states audited for a held item. Retail branches on `item_gobj` in
/// about forty files; a held item entering any other state is unported.
const HELD_ITEM_STATES: [S; 184] = [
    S::LightGet,
    // ftpickupitem.c and ftCo_Lift.c: a heavy item is lifted, carried,
    // turned and thrown (ftCo_ItemThrow.c); every one drops it when hit.
    S::HeavyGet,
    S::LiftWait,
    S::LiftWalk1,
    S::LiftWalk2,
    S::LiftTurn,
    S::HeavyThrowF,
    S::HeavyThrowB,
    S::HeavyThrowHi,
    S::HeavyThrowLw,
    S::HeavyThrowF4,
    S::HeavyThrowB4,
    S::HeavyThrowHi4,
    S::HeavyThrowLw4,
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
    // ftCo_StopWall.c (ftCo_8009EE30): a dash stopped by a wall; no item
    // branch, and its IASA is empty.
    S::StopWall,
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
    // ftCo_Attack1_CheckInput's drop with a shoulder held (Item_8026AC74
    // at the release flag).
    S::LightThrowDrop,
    // ftswing.c: a battering item swings in the hand (ft_0CD3.c).
    S::SwordSwing1,
    S::SwordSwing3,
    S::SwordSwing4,
    S::SwordSwingDash,
    S::BatSwing1,
    S::BatSwing3,
    S::BatSwing4,
    S::BatSwingDash,
    S::HarisenSwing1,
    S::HarisenSwing3,
    S::HarisenSwing4,
    S::HarisenSwingDash,
    S::StarRodSwing1,
    S::StarRodSwing3,
    S::StarRodSwing4,
    S::StarRodSwingDash,
    // ftCo_Rebound.c: a clanked swing rebounds with the item in hand (its
    // take_dmg_cb gave the hand back in ftCommon_8007DB58); no item branch.
    S::ReboundStop,
    S::Rebound,
    // ftCo_FallSpecial: no item branch for a held item.
    S::FallSpecial,
    S::FallSpecialF,
    S::FallSpecialB,
    // A jump while holding, and ftCo_80095328's air throws.
    S::KneeBend,
    S::JumpF,
    S::JumpB,
    // ftCo_JumpAerial_IASA runs the same item throw and catch checks.
    S::JumpAerialF,
    S::JumpAerialB,
    S::Fall,
    S::FallF,
    // ftCo_Ottotto_IASA is Wait's check list (the grab slot throws).
    S::Ottotto,
    S::OttottoWait,
    // ftCo_FallAerial* run ftCo_Fall's callbacks after an aerial jump.
    S::FallAerial,
    S::FallAerialF,
    S::FallAerialB,
    S::FallB,
    // ftCo_Pass (ftCo_Pass.c): the drop through a platform has Fall's IASA
    // item checks (ftCo_80095328, ftCo_800D7100) and no item branch.
    S::Pass,
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
    // ftCo_Down*.c: bound, wait, damage, getups and spot never read the item.
    S::DownBoundU,
    S::DownWaitU,
    S::DownDamageU,
    S::DownStandU,
    S::DownAttackU,
    S::DownFowardU,
    S::DownBackU,
    S::DownSpotU,
    S::DownBoundD,
    S::DownWaitD,
    S::DownDamageD,
    S::DownStandD,
    S::DownAttackD,
    S::DownFowardD,
    S::DownBackD,
    S::DownSpotD,
    // ftCo_EscapeAir_IASA: ftCo_80095328's air throws and drop for the
    // dodge's first PlCo +334 frames; otherwise an air dodge keeps it.
    S::EscapeAir,
    // ftCo_Capture*: no item branch; a captured fighter keeps it.
    S::CapturePulledHi,
    S::CaptureWaitHi,
    S::CaptureDamageHi,
    S::CapturePulledLw,
    S::CaptureWaitLw,
    S::CaptureDamageLw,
    S::CaptureCut,
    S::CaptureJump,
    // ftCo_Thrown* and ftCo_800DDDE4's release: no item branch either.
    S::ThrownF,
    S::ThrownB,
    S::ThrownHi,
    S::ThrownLw,
    // ftCo_Shouldered_IASA and the cargo throws' victims: no item branch.
    S::ShoulderedWait,
    S::ShoulderedWalkSlow,
    S::ShoulderedWalkMiddle,
    S::ShoulderedWalkFast,
    S::ShoulderedTurn,
    S::ThrownFF,
    S::ThrownFB,
    S::ThrownFHi,
    S::ThrownFLw,
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
    // ftCo_Landing (shared by LandingFallSpecial): no item branch; its
    // IASA sees the item like Wait's.
    S::Landing,
    S::LandingFallSpecial,
    S::LightThrowAirF,
    S::LightThrowAirB,
    S::LightThrowAirHi,
    S::LightThrowAirLw,
    S::LightThrowAirF4,
    S::LightThrowAirB4,
    S::LightThrowAirHi4,
    S::LightThrowAirLw4,
    // ftCo_Passive*.c: the techs never read the item.
    S::Passive,
    S::PassiveStandF,
    S::PassiveStandB,
    S::PassiveWall,
    S::PassiveWallJump,
    S::PassiveCeil,
    // ftCo_MissFoot.c: the slip off an edge never reads the item.
    S::MissFoot,
    // ftCo_StopCeil.c: the ceiling bump (ftCo_8009EFA4, from a jump's
    // ftCo_80096CC8) never reads the item and has no IASA.
    S::StopCeil,
    // ftCo_ShieldBreak*.c and ftCo_Furafura.c: a shield broken with the item
    // in hand (ftCo_80098B20) keeps it through the pop, the fall, the
    // get-up and the daze; none reads the item or offers an interrupt.
    S::ShieldBreakFly,
    S::ShieldBreakFall,
    S::ShieldBreakDownU,
    S::ShieldBreakDownD,
    S::ShieldBreakStandU,
    S::ShieldBreakStandD,
    S::Furafura,
    // ftCo_CaptureYoshi.c and ftCo_YoshiEgg.c: swallowed and laid as an egg
    // (Yoshi's Egg Lay), the victim keeps its item; neither reads it.
    S::CaptureYoshi,
    S::YoshiEgg,
    // ftkirbycaptureyoshi.c and ftkirbyyoshiegg.c: the same, from Kirby's
    // copied Egg Lay.
    S::CaptureKirbyYoshi,
    S::KirbyYoshiEgg,
    // ftCo_CaptureCaptain.c (ftCo_8009CA0C): ftCo_8009750C drops only a heavy
    // item; a light one stays in hand and no callback reads it.
    S::CaptureCaptain,
    // ftCo_CaptureMewtwo.c / ftCo_ThrownMewtwo.c: Confusion's catch drops
    // only a heavy item (ftCo_8009750C); neither state reads the item.
    S::CaptureMewtwo,
    S::CaptureMewtwoAir,
    S::ThrownMewtwo,
    S::ThrownMewtwoAir,
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
            tether_active: self.tether_article,
            floor: self.collision.data.floor,
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

    /// Fighter_OnItemPickup's ftAnim_80070FB4: the hand's pose for the
    /// item's hold kind (itGetHoldKind).
    pub(super) fn pose_hand_for_item(&mut self, hand_hold_kind: u8, assets: &FighterAssets) {
        let hand = assets
            .item_hand
            .expect("ftData_OnItemPickupExt for this kind");
        let pose = match hand_hold_kind {
            1 => Some(1),
            2 => Some(0),
            3 => Some(2),
            4 => Some(3),
            _ => None,
        };
        if let Some(pose) = pose {
            self.animation.part_animations[hand.pose].previous = pose;
        }
    }

    /// ftPe_8011D518's restore: fp->item_gobj is the stowed item again,
    /// shown and unfrozen (it_8026BB20, it_8026B73C); the hand's pickup pose
    /// (ftpickupitem_80094818(gobj, true)) follows once assets are at hand
    /// ([`Fighter::apply_pending_pickup_pose`]).
    pub fn restore_stowed_item(&mut self) {
        let held = self.stowed_item.take().expect("a stowed item");
        assert!(self.held_item.is_none(), "ftPe_8011D518: the hand is not empty");
        self.held_item = Some(held);
        self.item_requests.push(melee_it::ItemRequest::Stow {
            item: held.item,
            stowed: false,
        });
        self.pickup_pose_pending = true;
    }

    /// fp->item_gobj is released: Item_8026A848 -> ftCommon_8007E6DC.
    pub fn release_held_item(&mut self, item: u32, assets: &FighterAssets) {
        let held = self.held_item.take().expect("released item was held");
        assert_eq!(held.item, item, "ftLib_800867A0: released another item");
        self.release_hand_pose(assets);
    }

    /// A callback without the fighter's assets destroys the held item
    /// (Item_8026A8EC -> ftCommon_8007E6DC): fp->item_gobj is empty at once;
    /// the hand's drop pose follows before the proc's item requests
    /// ([`Fighter::apply_pending_pickup_pose`]).
    pub fn release_held_item_now(&mut self, item: u32) {
        let held = self.held_item.take().expect("released item was held");
        assert_eq!(held.item, item, "ftLib_800867A0: released another item");
        self.pickup_pose_pending = false;
        self.drop_pose_pending = true;
    }

    /// Fighter_OnItemDrop(gobj, true) for the item just let go.
    pub(super) fn release_hand_pose(&mut self, assets: &FighterAssets) {
        // ftCo_800C5240 is the hammer; OnItemDropExt: Fighter_OnItemDrop
        // (ft/inlines.h:188); ftLib_80086724 passes drop flag 1, so the shown
        // slot's live hand animation is removed too (ftAnim_80070CC4).
        let hand = assets
            .item_hand
            .expect("ftData_OnItemDropExt for this kind");
        // ftAnim_80070FB4(pose, -1); the pose stays applied until the next
        // motion change reinstalls the (now empty) selection.
        self.animation.part_animations[hand.pose].previous = -1;
        super::commands::remove_part_animation(
            &mut self.animation,
            &mut self.skeleton,
            assets,
            hand.shown,
        );
    }

    /// ftCommon_8007F5CC's hook target for the item now in hand.
    pub(super) fn held_item_hand(&self, assets: &FighterAssets) -> super::commands::HeldItemHand {
        use super::commands::HeldItemHand;
        match &self.held_item {
            None => HeldItemHand::Empty,
            Some(item) if item.heavy => HeldItemHand::Heavy,
            Some(_) => HeldItemHand::Light(
                assets
                    .item_hand
                    .expect("ftData_OnItemInvisible for this kind")
                    .shown,
            ),
        }
    }

    /// Keep a held item inside the audited states (see HELD_ITEM_STATES).
    /// Character special rows pass `special` with the table's
    /// `specials_keep_held_item` audit instead.
    pub(super) fn require_held_item_state(
        &self,
        action: ActionId,
        state: S,
        special: Option<bool>,
        common_down_air_animation: bool,
    ) {
        if self.held_item.is_some_and(|item| {
            !held_item_state_is_audited(
                item.kind,
                action,
                state,
                special,
                common_down_air_animation,
            )
        }) {
            unimplemented!("holding an item: motion state {state:?} is not audited");
        }
    }
}

/// A private character row can share a common identity but run different
/// callbacks (Game & Watch's Manhole is row 345, identity AttackLw3), so
/// only the actual common rows are admitted. With a battering item in hand
/// the vertical tilts and smashes, the aerials and their landings run their
/// common callbacks unchanged: none reads the item beyond the item-throw
/// checks (ftCo_AttackHi3/Lw3/Hi4/Lw4.c, ftCo_AttackAir.c, ftCo_LandingAir.c).
fn held_item_state_is_audited(
    kind: melee_types::ItemKind,
    action: ActionId,
    state: S,
    special: Option<bool>,
    common_down_air_animation: bool,
) -> bool {
    match special {
        Some(keeps_item) => keeps_item,
        None => {
            HELD_ITEM_STATES.contains(&state)
                || (super::item_swing::is_swing_item(kind)
                    && action == ActionId(state as u16)
                    && (SWING_ITEM_COMMON_STATES.contains(&state)
                        || (state == S::AttackAirLw && common_down_air_animation)))
        }
    }
}

/// The common rows a fighter enters with a battering item in hand (beside
/// the swings, which HELD_ITEM_STATES lists).
const SWING_ITEM_COMMON_STATES: [S; 13] = [
    S::AttackHi3,
    S::AttackLw3,
    S::AttackHi4,
    S::AttackLw4,
    S::AttackAirN,
    S::AttackAirF,
    S::AttackAirB,
    S::AttackAirHi,
    S::LandingAirN,
    S::LandingAirF,
    S::LandingAirB,
    S::LandingAirHi,
    S::LandingAirLw,
];

impl FighterCore {
    /// ftCo_800D7100 / ftCo_800D705C's shared test: LR held and A pressed
    /// with an empty hand, no catch yet this airtime (x2224_b1) and the
    /// previous A press at least PlCo +1C frames old (x683).
    fn catch_input(&self, assets: &FighterAssets) -> bool {
        use crate::input::Buttons;
        self.held_item.is_none()
            && self.input.current.held.intersects(Buttons::SHIELD)
            && self.input.pressed.intersects(Buttons::A)
            && !self.item_catch_locked
            && i32::from(self.input.buttons.previous_attack) >= assets.damage.tech_lockout
    }

    /// ftCo_800D705C (800D705C): the same input with nothing caught yet
    /// opens a catch window (ftCo_800D71D8 runs it). Returns whether it did.
    pub(super) fn try_open_catch_window(&mut self, assets: &FighterAssets) -> bool {
        if !self.catch_input(assets) {
            return false;
        }
        self.catch_window = assets.damage.catch_window_frames as u16;
        true
    }
}

impl FighterCore {
    /// The holder lent to its held item's callbacks: the part's joint
    /// (ftLib_80086630), the ECB centre and the current attack with its
    /// stale multiplier.
    #[inline(never)] // one definition; melee-lib lends it to item callbacks
    pub fn item_holder(&mut self, part: u8, assets: &FighterAssets) -> melee_it::ItemHolder<'_> {
        let joint = self.animation.parts[usize::from(part)].joint;
        let attack = self.combat.stale.attack();
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
            attack,
            attack_stale: self
                .combat
                .stale
                .multiplier_for(attack.map(|attack| attack.move_id), &assets.stale_weights),
        }
    }
}

impl Fighter {
    /// ftpickupitem_80094790 (80094790), before a jab or tilt: pick up an
    /// item in reach instead. True when LightGet or HeavyGet began.
    pub fn try_item_pickup(&mut self, assets: &FighterAssets) -> Result<bool> {
        // x1978, the second (consumable) hand, is never filled here.
        let Some(item) = self.core.find_pickup(&assets.pickup, PickupWeights::ANY) else {
            return Ok(false);
        };
        let state = if item.heavy { S::HeavyGet } else { S::LightGet };
        self.enter_item_get(state, assets)?;
        Ok(true)
    }

    /// fn_800D8BFC (800D8BFC), Catch's x2194 once ftColl_8007BC90 found a
    /// stage enemy in the grab: the hand closes on it
    /// (ftpickupitem_800948A8), then HeavyGet in loop mode
    /// (ftpickupitem_80094694(gobj, 0x5D, 1)): the animation steps until
    /// its throw flag (ftCheckThrowB3) has closed the hand.
    pub fn catch_stage_enemy(&mut self, item: PickupCandidate, assets: &FighterAssets) -> Result<()> {
        if !matches!(self.core.motion_state.id, S::Catch | S::CatchDash) {
            unimplemented!("x2194 of a special grab catching a stage enemy (ftKb_SpecialN fn_800F6178)");
        }
        self.core.take_item(item, assets);
        self.enter_item_get_with(S::HeavyGet, assets, true)
    }

    /// ftpickupitem_80094694 (80094694) without its loop mode: clear the
    /// throw flags, enter the state and play its first frame.
    fn enter_item_get(&mut self, state: S, assets: &FighterAssets) -> Result<()> {
        self.enter_item_get_with(state, assets, false)
    }

    /// ftpickupitem_80094694 (80094694): clear the throw flags, enter the
    /// state, and step its animation once or, in loop mode, until the
    /// throw flag B3 (taken as it is read).
    fn enter_item_get_with(&mut self, state: S, assets: &FighterAssets, looping: bool) -> Result<()> {
        if state == S::HeavyGet && self.core.player.scale != 1.0 {
            unimplemented!("ftpickupitem.c:213: scaled HeavyGet animation rate");
        }
        // 800946CC calls CalcYScaledKnockback(1, scale.y, common.x8).
        // Supported setups have unit scale: its 800CF6B8 fmadds returns 1.
        self.core.commands.clear_throw_flags();
        let retained_word = self.inherited_scratch_word();
        self.change_motion_state(state.into(), assets)?;
        if looping {
            // ftpickupitem.c:223: the flag is tested before each step.
            while !self.core.commands.take_throw_flag_b3() {
                assert!(
                    self.core.animation.frames_remaining(&self.core.skeleton),
                    "ftpickupitem_80094694: HeavyGet ran out without its throw flag"
                );
                self.step_animation(assets);
            }
        } else {
            self.step_animation(assets);
        }
        // 80094754..60 publishes mv.co.itemget.x0, then take_dmg_cb.
        // Publish the explicit common callback owner after this scratch write;
        // interrupt_actions uses it instead of the character hook. Other
        // mv words survive.
        self.core.state_data = MotionData::ItemGet {
            heavy: state == S::HeavyGet,
            retained_word,
        };
        self.core.heavy_item_damage_armed = state == S::HeavyGet;
        Ok(())
    }

    /// ftpickupitem_Anim (80094A14): the script's throw flag closes the
    /// hand on the nearest item; the animation ends in Wait.
    pub(super) fn item_get_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::ItemGet { heavy, .. } = self.core.state_data else {
            panic!("item get scratch")
        };
        if self.core.commands.take_throw_flag_b3() {
            let weights = if heavy {
                PickupWeights::HEAVY
            } else {
                PickupWeights::LIGHT
            };
            if let Some(item) = self.core.find_pickup(&assets.pickup, weights) {
                self.core.take_item(item, assets);
            }
        }
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            if self.core.held_item.is_some() && self.core.motion_state.id == S::HeavyGet {
                // ftpickupitem.c:319-323: x2222_b0's carry, else LiftWait.
                if self.carries_cargo() {
                    unimplemented!("ftDk_MS_341_800DF980 (ftpickupitem.c:320): x2222_b0 lift");
                }
                return self.enter_lift_wait(assets);
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
        if !self.core.catch_input(assets) {
            return false;
        }
        let Some(item) = self.core.find_pickup(&assets.pickup, PickupWeights::LIGHT) else {
            return false;
        };
        self.core.catch_item(item, assets);
        true
    }


    /// ftPe_SpecialHi_8011D424's stow: the held item is hidden and frozen
    /// in place (it_8026BB44, it_8026B724), then the hand lets it go as
    /// fp->item_gobj (ftCommon_8007E6DC(gobj, item, true)) for the article
    /// about to take the hand.
    pub fn stow_held_item(&mut self, assets: &FighterAssets) {
        let held = self.core.held_item.expect("a stowed item is held");
        self.core.item_requests.push(melee_it::ItemRequest::Stow {
            item: held.item,
            stowed: true,
        });
        self.core.release_held_item(held.item, assets);
        self.core.stowed_item = Some(held);
    }

    /// Apply a pending [`FighterCore::restore_stowed_item`] hand pose.
    pub fn apply_pending_pickup_pose(&mut self, assets: &FighterAssets) {
        if std::mem::take(&mut self.core.drop_pose_pending) {
            self.core.release_hand_pose(assets);
        }
        if !std::mem::take(&mut self.core.pickup_pose_pending) {
            return;
        }
        let Some(held) = self.core.held_item else {
            return;
        };
        // ftpickupitem_80094818(gobj, true) -> ftData_OnItemPickupExt:
        // Fighter_OnItemPickup (ft/inlines.h:143), light items only.
        if held.heavy {
            return;
        }
        self.core.pose_hand_for_item(held.hand_hold_kind, assets);
        let hand = assets
            .item_hand
            .expect("ftData_OnItemPickupExt for this kind");
        super::commands::show_part_selection(
            &mut self.core.animation,
            &mut self.core.skeleton,
            assets,
            hand.shown,
        );
    }

    /// ftpickupitem_80094818(gobj, false) (800943BC) with fp->item_gobj
    /// already held: Fighter_OnItemPickup's hand pose for a light item,
    /// without the shown selection.
    pub fn pose_hand_for_held_item(&mut self, assets: &FighterAssets) {
        let Some(held) = self.core.held_item else {
            return;
        };
        if !held.heavy {
            self.core.pose_hand_for_item(held.hand_hold_kind, assets);
        }
    }

    /// A character article created straight into the hand (Peach's
    /// setupVeg, Link's bomb: fp->item_gobj = the new item, then
    /// ftpickupitem_80094818(gobj, catch_item), which poses the hand and,
    /// with the catch flag, applies ftAnim_80070C48's shown selection).
    /// Item_8026AB54 has already run.
    pub fn hold_spawned_item(&mut self, item: HeldItem, catch_item: bool, assets: &FighterAssets) {
        assert!(self.core.held_item.is_none(), "x1978: a second held item");
        self.core.held_item = Some(item);
        if item.heavy {
            return;
        }
        self.core.pose_hand_for_item(item.hand_hold_kind, assets);
        if catch_item {
            let hand = assets
                .item_hand
                .expect("ftData_OnItemPickupExt for this kind");
            super::commands::show_part_selection(
                &mut self.core.animation,
                &mut self.core.skeleton,
                assets,
                hand.shown,
            );
        }
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

impl FighterCore {
    /// ftCo_800D71D8 (800D71D8), each unfrozen animation proc after
    /// ftAnim_8006EBA4 (the state's first [`Self::step_animation`], or after
    /// its callback when it does not step): while the window is open an
    /// empty hand catches a light item in reach; the window's last frame
    /// sets the catch lock.
    pub(crate) fn run_catch_window(&mut self, assets: &FighterAssets) {
        if self.catch_window == 0 {
            return;
        }
        let item = self.find_pickup(&assets.pickup, PickupWeights::LIGHT);
        self.catch_window -= 1;
        if self.catch_window == 0 {
            self.item_catch_locked = true;
        }
        if let Some(item) = item {
            self.catch_item(item, assets);
            self.catch_window = 0;
        }
    }

    /// fn_800D6F58 (800D6F58): the hand closes on `item` with no motion
    /// change (ftpickupitem_800948A8's light path), then ftpickupitem_8009447C
    /// (nothing for a thrown kind), the pickup sparkle (efAsync_Spawn 0x422
    /// at the light item part) and x2224_b1.
    fn catch_item(&mut self, item: PickupCandidate, assets: &FighterAssets) {
        self.take_item(item, assets);
        let part = usize::from(self.bones.model.animation_translation);
        let joint = self.animation.parts[part].joint.0;
        self.effects
            .push(melee_ef::request::EffectRequest::Attached {
                id: ITEM_PICKUP_SPARKLE,
                bone: joint,
            });
        self.item_catch_locked = true;
    }

    /// ftpickupitem_800948A8 (800948A8): the hand closes on `item`.
    fn take_item(&mut self, item: PickupCandidate, assets: &FighterAssets) {
        assert!(self.held_item.is_none(), "x1978: a second held item");
        assert_ne!(item.use_kind, 5, "ftpickupitem_8009447C: consumable pickup");
        self.held_item = Some(HeldItem {
            item: item.item,
            kind: item.kind,
            heavy: item.heavy,
            hand_hold_kind: item.hand_hold_kind,
            use_kind: item.use_kind,
            damage_multiplier: item.damage_multiplier,
            ammo: item.ammo,
            shot_offset: item.shot_offset,
        });
        // ftpickupitem_80094818(gobj, true) -> ftData_OnItemPickupExt:
        // Fighter_OnItemPickup (ft/inlines.h:143) and the Kirby and Purin
        // variants, light items only.
        if !item.heavy {
            self.pose_hand_for_item(item.hand_hold_kind, assets);
            let hand = assets
                .item_hand
                .expect("ftData_OnItemPickupExt for this kind");
            // ftAnim_80070C48: apply the shown slot's selection.
            super::commands::show_part_selection(
                &mut self.animation,
                &mut self.skeleton,
                assets,
                hand.shown,
            );
        }
        // Item_8026AB54 at the item part (ftData x8 +0x10, +0x11 if heavy).
        let held = self.held_item.expect("just taken");
        self.item_requests.push(melee_it::ItemRequest::PickUp {
            item: item.item,
            part: self.held_item_part(&held),
        });
        if item.heavy {
            // ft_800881D8(fp, x4C_sfx->x2C, 127, 64).
            self.push_voice(assets.lift.lift_sound);
        }
    }
}

#[cfg(test)]
mod held_state_tests {
    use super::*;

    #[test]
    fn battering_audit_admits_common_attack_rows_but_not_private_ones() {
        use melee_types::ItemKind;
        // The common vertical tilts and smashes, the five aerials and their
        // landings are witnessed with a Sword and a Bat in hand; private
        // rows keep their guards, even with the same common identity.
        for kind in [ItemKind::Sword, ItemKind::Bat] {
            for (action, state, expected) in [
                (57, S::AttackLw3, true),
                (63, S::AttackHi4, true),
                (345, S::AttackLw3, false), // Game & Watch's Manhole callbacks.
                (56, S::AttackHi3, true),
                (64, S::AttackLw4, true),
                (67, S::AttackAirB, true),
                (348, S::AttackAirB, false), // Game & Watch's Turtle row.
                (65, S::AttackAirN, true),
                (66, S::AttackAirF, true),
                (68, S::AttackAirHi, true),
                (69, S::AttackAirLw, true),
                (349, S::AttackAirHi, false), // Game & Watch's private up aerial.
                (349, S::AttackAirLw, false), // Private action is not common69.
                (345, S::AttackAirF, false), // Private action/common identity mismatch.
                (70, S::LandingAirN, true),
                (71, S::LandingAirF, true),
                (72, S::LandingAirB, true),
                (73, S::LandingAirHi, true),
                (74, S::LandingAirLw, true),
            ] {
                assert_eq!(
                    held_item_state_is_audited(kind, ActionId(action), state, None, true),
                    expected,
                    "{kind:?}: action {action}, identity {state:?}"
                );
            }
        }
        for (action, state) in [(66, S::AttackAirF), (67, S::AttackAirB), (68, S::AttackAirHi)] {
            assert!(held_item_state_is_audited(ItemKind::Sword, ActionId(action), state, None, false));
        }
        assert!(!held_item_state_is_audited(
            ItemKind::Sword,
            ActionId(69),
            S::AttackAirLw,
            None,
            false,
        ));
        assert!(!held_item_state_is_audited(
            ItemKind::Dosei,
            ActionId(69),
            S::AttackAirLw,
            None,
            true,
        ));
        assert!(!held_item_state_is_audited(
            ItemKind::Sword,
            ActionId(57),
            S::AttackLw3,
            Some(false),
            true,
        ));
        assert!(!held_item_state_is_audited(
            ItemKind::Dosei,
            ActionId(67),
            S::AttackAirB,
            None,
            true,
        ));
        for (action, state) in [(66, S::AttackAirF), (68, S::AttackAirHi)] {
            assert!(!held_item_state_is_audited(
                ItemKind::Dosei,
                ActionId(action),
                state,
                None,
                true
            ));
        }
        assert!(!held_item_state_is_audited(
            ItemKind::Dosei,
            ActionId(57),
            S::AttackLw3,
            None,
            true,
        ));
    }
}
