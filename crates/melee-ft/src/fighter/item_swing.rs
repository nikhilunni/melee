//! Swinging a held battering item: ftswing.c (ftCo_Attack_800CCF58),
//! ft_0CD1.c (the shared swing callbacks) and ft_0CD3.c (the per-item
//! rows). Only the Beam Sword is supported; the bat, fan, star rod,
//! parasol and lipstick fail closed at entry.
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
/// [`SwingInput`], rows in fn_800CCEC4's order (sword, bat, fan, star rod,
/// parasol, lipstick).
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

/// lbl_803C6D70's sword row: the motion state per [`SwingInput`].
const SWORD_SWINGS: [Option<S>; 5] = [
    Some(S::SwordSwing1),
    Some(S::SwordSwing3),
    Some(S::SwordSwing4),
    None,
    Some(S::SwordSwingDash),
];

/// mv.co.swing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SwingState {
    /// x0: A has been held since the swing began.
    pub a_held: bool,
    /// x4: the swing's input.
    pub input: SwingInput,
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
        let (row, states) = match held.kind {
            ItemKind::Sword => (0, &SWORD_SWINGS),
            kind => unimplemented!("fn_800CCEC4: swinging {kind:?}"),
        };
        let state = states[input as usize]
            .unwrap_or_else(|| unimplemented!("get_anim_id: Swing42 ({:?})", held.kind));
        let rate = assets.swing_rates[row][input as usize];
        // ftCo_800CD350 -> ftCo_800CD140 (800CD140): the sword's rows pass no
        // motion flags.
        self.core.commands.clear_throw_flags();
        self.change_motion_state_with_rate(state.into(), assets, 0.0, rate)?;
        self.step_animation(assets);
        self.core.status.interaction = super::Interaction::Attack;
        self.core.state_data = MotionData::Swing(SwingState {
            a_held: true,
            input,
        });
        // ftCommon_8007E79C(gobj, 1) -> ftData_OnItemDrop: the swing's
        // animation takes the hand.
        self.core.release_hand_pose(assets);
        // take_dmg_cb = ft_800CD31C.
        self.core.swing_hand_armed = true;
        Ok(())
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
    /// flag with A still held would chain the second smash swing.
    pub(super) fn swing_input(&mut self) {
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
            unimplemented!("ftCo_Attack_800CCF58(gobj, 3): Swing42 needs Captain Falcon");
        }
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
            self.swing_restore_hand(assets);
            self.leave_ground();
            self.change_motion_state(S::Fall.into(), assets)?;
        }
        Ok(())
    }
}
