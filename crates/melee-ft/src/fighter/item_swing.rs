//! Swinging a held battering item: ftswing.c (ftCo_Attack_800CCF58),
//! ft_0CD1.c (the shared swing callbacks) and ft_0CD3.c (the per-item
//! rows). The sword, bat, fan and star rod are supported; the parasol and
//! lipstick fail closed at entry.
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use melee_types::{CommonMotionState as S, ItemKind};

/// The swing's input, ftCo_Attack_800CCF58's second argument (mv.co.swing.x4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwingInput {
    /// A (ftCo_Attack1_CheckInput): Swing1.
    Jab = 0,
    /// A side tilt (ftCo_AttackS3_CheckInput): Swing3.
    Tilt = 1,
    /// A side smash (ftCo_AttackS4's checkItemThrow): Swing4.
    Smash = 2,
    /// A held through the script's flag (ftCo_800CD204): Swing42, which only
    /// Captain Falcon has.
    SmashFollowUp = 3,
    /// A dash attack (ftCo_AttackDash_CheckInput): SwingDash.
    Dash = 4,
}

/// Fighter_804D654C (PlCo pData[2]): each swing item's animation rate per
/// [`SwingInput`], rows in fn_800CCEC4's order (sword, bat,
/// parasol, fan, star rod, lipstick).
pub type SwingRates = [[f32; 5]; 6];

pub fn read_swing_rates(
    common: &hsd_archive::Archive,
    root: u32,
) -> hsd_archive::desc::Result<SwingRates> {
    let table = common
        .link(root + 8)?
        .ok_or(hsd_archive::desc::DescError::NullPointer {
            field: "ftLoadCommonData[2]",
            at: root + 8,
        })?;
    let r = common.reader();
    let mut rates = [[0.0; 5]; 6];
    for (i, row) in rates.iter_mut().enumerate() {
        for (j, rate) in row.iter_mut().enumerate() {
            *rate = r.f32(table + ((i * 5 + j) * 4) as u32)?;
        }
    }
    Ok(rates)
}

/// lbl_803C6D70's sword row: the motion state per [`SwingInput`]; Swing42
/// (-1) is the kind's own row (`CharacterCallbacks::SWING42_FIRST_ROW`).
const SWORD_SWINGS: [Option<S>; 5] = [
    Some(S::SwordSwing1),
    Some(S::SwordSwing3),
    Some(S::SwordSwing4),
    None,
    Some(S::SwordSwingDash),
];
/// lbl_803C6D70's bat row.
const BAT_SWINGS: [Option<S>; 5] = [
    Some(S::BatSwing1),
    Some(S::BatSwing3),
    Some(S::BatSwing4),
    None,
    Some(S::BatSwingDash),
];
/// lbl_803C6D70's fan row (fn_800CCEC4's 3).
const HARISEN_SWINGS: [Option<S>; 5] = [
    Some(S::HarisenSwing1),
    Some(S::HarisenSwing3),
    Some(S::HarisenSwing4),
    None,
    Some(S::HarisenSwingDash),
];

/// fn_800CCEC4 (800CCEC4): a battering item's row in lbl_803C6D70 and
/// Fighter_804D654C, and its swing states, for the kinds whose swings are
/// ported. Retail's rows: sword 0, bat 1, parasol 2, fan 3, star rod 4,
/// lipstick 5, the motion states' order too.
fn swing_row(kind: ItemKind) -> Option<(usize, &'static [Option<S>; 5])> {
    match kind {
        ItemKind::Sword => Some((0, &SWORD_SWINGS)),
        ItemKind::Bat => Some((1, &BAT_SWINGS)),
        ItemKind::Harisen => Some((3, &HARISEN_SWINGS)),
        ItemKind::StarRod => Some((4, &STARROD_SWINGS)),
        _ => None,
    }
}
/// lbl_803C6D70's star rod row.
const STARROD_SWINGS: [Option<S>; 5] = [
    Some(S::StarRodSwing1),
    Some(S::StarRodSwing3),
    Some(S::StarRodSwing4),
    None,
    Some(S::StarRodSwingDash),
];

/// ftCo_800CD82C's cmd_vars[0]: no sound waiting for the accessory.
const NO_SHOT_SOUND: u32 = 540000;
/// ft_800CD914: the Star Rod's sound when it is out of shots.
const EMPTY_STAR_ROD_SOUND: u32 = 234;
/// ft_800CD914's efAsync_Spawn: the shot's flash, and the empty puff.
const STAR_SHOT_EFFECT: u16 = 1070;
const EMPTY_SHOT_EFFECT: u16 = 1032;

/// Whether `kind` is a battering item whose swings are ported
/// (it_8026B30C == 2 for the sword, bat, fan, star rod, parasol, lipstick).
pub fn is_swing_item(kind: ItemKind) -> bool {
    swing_row(kind).is_some()
}

/// mv.co.swing.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwingState {
    /// x0: A has been held since the swing began.
    pub a_held: bool,
    /// x4: the swing's input.
    pub input: SwingInput,
    /// x8: the Fan's frames left swollen after its hit (ft_800CD65C); only
    /// the Fan's entry (ftCo_800CD5A8) and callbacks touch it.
    pub swell_frames: f32,
}

impl Fighter {
    /// ftCo_Attack_800CCF58 (800CCF58): a jab or tilt input first picks up
    /// an item in reach (ftpickupitem_80094790); otherwise the held item's
    /// swing for `input` begins.
    pub(super) fn enter_item_swing(
        &mut self,
        input: SwingInput,
        assets: &FighterAssets,
    ) -> Result<()> {
        if matches!(input, SwingInput::Jab | SwingInput::Tilt) && self.try_item_pickup(assets)? {
            return Ok(());
        }
        let held = self.core.held_item.expect("ftswing.c:124: fp->item_gobj");
        // fn_800CCEC4: the swing row by item kind.
        let Some((row, states)) = swing_row(held.kind) else {
            unimplemented!("fn_800CCEC4: swinging {:?}", held.kind)
        };
        let action = match states[input as usize] {
            Some(state) => state.into(),
            // get_anim_id's Swing42 arm: the kind's own row for the item.
            None => {
                let first = self
                    .character
                    .table()
                    .swing42_first_row
                    .unwrap_or_else(|| {
                        // retail 0x800CD034..800CD080: only Captain owns Swing42.
                        // Entry clears all throw flags; among all 33 original
                        // command owners only Captain's six smash scripts set b3.
                        unreachable!("ftswing.c:97: retail Swing42 requires a static motion")
                    });
                super::ActionId(first.0 + row as u16)
            }
        };
        let rate = assets.swing_rates[row][input as usize];
        let fan = held.kind == ItemKind::Harisen;
        let shoots = held.kind == ItemKind::StarRod;
        if fan {
            // ftCo_800CD5A8 (800CD5A8): cmd_vars[0] and mv x8 cleared first.
            self.core.commands.variables[0] = 0;
        }
        if shoots {
            // ftCo_800CD82C (800CD82C): no sound waiting, a weak shot.
            self.core.commands.variables[0] = NO_SHOT_SOUND;
            self.core.commands.variables[1] = 0;
        }
        // ftCo_800CD350 / 800CD418 / 800CD5A8 -> ftCo_800CD140 (800CD140):
        // the sword's, bat's and fan's rows pass no motion flags.
        self.core.commands.clear_throw_flags();
        self.change_motion_state_with_rate(action, assets, 0.0, rate)?;
        self.step_animation(assets);
        self.core.status.interaction = super::Interaction::Attack;
        self.core.state_data = MotionData::Swing(SwingState {
            a_held: true,
            input,
            swell_frames: 0.0,
        });
        // ftCommon_8007E79C(gobj, 1) -> ftData_OnItemDrop: the swing's
        // animation takes the hand.
        self.core.release_hand_pose(assets);
        if fan {
            // deal_dmg_cb = ft_800CD65C, take_dmg_cb = ft_800CD604.
            self.core.fan_callbacks_armed = true;
        } else {
            // take_dmg_cb = ft_800CD31C.
            self.core.swing_hand_armed = true;
        }
        if shoots {
            // accessory4_cb = ft_800CD914, after the entry's step.
            self.core.arm_accessory4();
        }
        Ok(())
    }

    /// ft_800CD914 (800CD914), the Star Rod swing's accessory4: the sound
    /// the script asked for (cmd_vars[0]; the empty rod's own when it has
    /// no shots), then on throw flag b0 a star from the rod's tip
    /// (it_80292394 offset through the hand joint, lb_8000B1CC) with its
    /// flash, or the empty puff. Returns whether it owns accessory4 this
    /// tick.
    pub fn swing_accessory(&mut self) -> bool {
        use melee_it::{ItemControl, ItemRequest, SpawnItem};
        let action = self.core.motion_state.action;
        if !(S::StarRodSwing1 as u16..=S::StarRodSwingDash as u16).contains(&action.0) {
            return false;
        }
        if !self.core.accessory4_armed {
            return false;
        }
        let Some(mut held) = self.core.held_item else {
            return true;
        };
        assert_eq!(held.kind, ItemKind::StarRod, "ftswing.c:100: It_Kind_StarRod");
        // it_8026B594: the rod is empty once xD4C reaches zero.
        let empty = held.ammo <= 0;
        let commands = &mut self.core.commands;
        if commands.variables[0] != NO_SHOT_SOUND {
            let id = if empty {
                EMPTY_STAR_ROD_SOUND
            } else {
                commands.variables[0]
            };
            commands.footstep_sounds.push(super::commands::FootstepSound {
                channel: super::commands::SoundChannel::Ordinary,
                id,
                volume: 127,
                pan: 64,
            });
            commands.variables[0] = NO_SHOT_SOUND;
        }
        // ftCheckThrowB0.
        if !std::mem::take(&mut commands.throw_accessory) {
            return true;
        }
        let strong = commands.variables[1];
        let part = usize::from(self.core.bones.model.animation_translation);
        let tip = super::caches::part_position(
            &mut self.core.skeleton,
            &self.core.animation,
            part,
            held.shot_offset,
        );
        if empty {
            unimplemented!(
                "ft_800CD914 (ft_0CD3.c:257-259): the empty Star Rod's puff, efAsync {EMPTY_SHOT_EFFECT} at {tip:?}"
            );
        }
        // efAsync_Spawn(gobj, &x60C, 5, 1070, NULL, &tip, &facing_dir).
        self.core
            .effects
            .push(melee_ef::request::EffectRequest::PositionalGenerator {
                id: STAR_SHOT_EFFECT,
                position: tip,
            });
        // it_802923BC: one shot fewer, then it_802988E4's star.
        self.core.item_requests.push(ItemRequest::ControlInstance {
            item: held.item,
            control: ItemControl::Fire,
        });
        if held.ammo > 0 {
            held.ammo -= 1;
        }
        self.core.held_item = Some(held);
        let c = &self.core;
        let mut spawn = SpawnItem::ray(
            ItemKind::StarRodStar,
            c.player.id,
            hsd_types::Vec3::new(tip.x, tip.y, 0.0),
            c.physics.facing,
        );
        // it_8026BB68 -> ftLib_80086990: the ECB's midpoint (fadds, fmuls,
        // fadds).
        let midpoint = 0.5 * (c.collision.data.ecb.top.y + c.collision.data.ecb.bottom.y);
        spawn.position = hsd_types::Vec3::new(
            c.physics.position.x + 0.0,
            c.physics.position.y + midpoint,
            c.physics.position.z + 0.0,
        );
        spawn.spawn_argument = strong as i32;
        self.core.item_requests.push(ItemRequest::Spawn(spawn));
        true
    }

    /// ftCo_HarisenSwing_Anim (800CD688): the swollen fan's countdown, then
    /// the script's requests for the fan's own swing animation
    /// (cmd_vars[0] == 1) or its rest (== 2), then ftCo_800CD1BC.
    pub(super) fn fan_swing_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        use melee_it::{ItemControl, ItemRequest, SwingAnimation};
        if let Some(held) = self.core.held_item {
            let MotionData::Swing(swing) = &mut self.core.state_data else {
                panic!("swing scratch missing")
            };
            let mut requests = melee_types::fixed::FixedVec::<ItemControl, 2>::default();
            if swing.swell_frames > 0.0 {
                swing.swell_frames -= 1.0;
                if swing.swell_frames <= 0.0
                    || !self.core.animation.frames_remaining(&self.core.skeleton)
                {
                    requests.push(ItemControl::SwingShrink);
                }
            }
            let input = swing.input;
            if self.core.commands.variables[0] == 1 {
                let swing = match input {
                    SwingInput::Jab => SwingAnimation::Jab,
                    SwingInput::Tilt | SwingInput::Dash => SwingAnimation::Tilt,
                    SwingInput::Smash | SwingInput::SmashFollowUp => SwingAnimation::Smash,
                };
                requests.push(ItemControl::SwingAnimation {
                    swing,
                    rate: self.core.animation.speed,
                });
                self.core.commands.variables[0] = 0;
            }
            if self.core.commands.variables[0] == 2 {
                requests.push(ItemControl::SwingRest);
                self.core.commands.variables[0] = 0;
            }
            for control in requests {
                self.core.item_requests.push(ItemRequest::ControlInstance {
                    item: held.item,
                    control,
                });
            }
        }
        self.swing_animation(assets)
    }

    /// ft_800CD604 (800CD604), the Fan's take_dmg_cb and its leaving the
    /// floor: with the fan still held it shrinks back (it_8029282C), rests
    /// (it_80292A28) and the hand takes it again.
    pub(super) fn fan_restore(&mut self, assets: &FighterAssets) {
        use melee_it::{ItemControl, ItemRequest};
        if let Some(held) = self.core.held_item {
            for control in [ItemControl::SwingShrink, ItemControl::SwingRest] {
                self.core.item_requests.push(ItemRequest::ControlInstance {
                    item: held.item,
                    control,
                });
            }
            self.show_held_item_in_hand(assets);
        }
    }

    /// ft_800CD65C (800CD65C), the Fan's deal_dmg_cb: the held fan swells
    /// (it_802927E8) for PlCo +5A0 frames.
    pub(super) fn fan_dealt_damage(&mut self, assets: &FighterAssets) {
        use melee_it::{ItemControl, ItemRequest};
        if let Some(held) = self.core.held_item {
            self.core.item_requests.push(ItemRequest::ControlInstance {
                item: held.item,
                control: ItemControl::SwingSwell,
            });
            if let MotionData::Swing(swing) = &mut self.core.state_data {
                swing.swell_frames = assets.fan_swell_frames;
            }
        }
    }

    /// ftCo_800CD1BC (800CD1BC): Wait at the end (ft_8008A2BC), and the
    /// hand takes the item again (ftCommon_8007E7E4(gobj, 1)).
    pub(super) fn swing_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.change_motion_state(S::Wait.into(), assets)?;
            self.show_held_item_in_hand(assets);
        }
        Ok(())
    }

    /// ftCo_800CD204 (800CD204): releasing A ends the hold; the script's
    /// flag with A still held chains the second smash swing (Swing42,
    /// ftCo_Attack_800CCF58(gobj, 3)).
    pub(super) fn swing_input(&mut self, assets: &FighterAssets) -> Result<()> {
        use crate::input::Buttons;
        let held = self.core.input.current.held.intersects(Buttons::A);
        let MotionData::Swing(swing) = &mut self.core.state_data else {
            panic!("swing scratch missing")
        };
        if !held {
            swing.a_held = false;
        }
        let a_held = swing.a_held;
        if self.core.commands.take_throw_flag_b3() && a_held {
            self.enter_item_swing(SwingInput::SmashFollowUp, assets)?;
        }
        Ok(())
    }

    /// ft_800CD31C (800CD31C), the swing's take_dmg_cb and its leaving the
    /// floor: with an item still held the hand takes it again
    /// (ftCommon_8007E7E4(gobj, 1)).
    pub(super) fn swing_restore_hand(&mut self, assets: &FighterAssets) {
        if self.core.held_item.is_some() {
            self.show_held_item_in_hand(assets);
        }
    }

    /// ftCommon_8007E7E4(gobj, 1) -> ftData_OnItemPickup: Fighter_OnItemPickup
    /// with the catch flag (ft/inlines.h:143).
    fn show_held_item_in_hand(&mut self, assets: &FighterAssets) {
        let Some(held) = self.core.held_item else {
            return;
        };
        if held.heavy {
            return;
        }
        self.core.pose_hand_for_item(held.hand_hold_kind, assets);
        let hand = assets.item_hand.expect("ftData_OnItemPickup for this kind");
        super::commands::show_part_selection(
            &mut self.core.animation,
            &mut self.core.skeleton,
            assets,
            hand.shown,
        );
    }

    /// ftCo_800CD2C4 (800CD2C4): off the floor (ft_800827A0) the hand takes
    /// the item again (ft_800CD31C) and the fighter falls.
    pub(super) fn swing_collision(
        &mut self,
        map: &mut melee_mp::CollMap,
        assets: &FighterAssets,
        on_leaving_floor: fn(&mut Fighter, &FighterAssets),
    ) -> Result<()> {
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
            on_leaving_floor(self, assets);
            self.leave_ground();
            self.change_motion_state(S::Fall.into(), assets)?;
        }
        Ok(())
    }
}
