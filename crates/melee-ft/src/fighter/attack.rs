//! Shared ground attack entry/callbacks, ftCo_Attack1.c / ftCo_AttackHi3.c.
pub mod aerial;
pub mod combo;
pub mod stale;
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use crate::input::{pad::Buttons, WaitContext, WaitPredicate as P, WaitTransition as T};
use melee_gr::wind::Wind;
use melee_types::CommonMotionState as S;

/// mv.co.attack1: the combo input was pressed (x0) and the rapid-jab A
/// edges counted so far. The window itself is FighterCore::jab_countdown.
#[derive(Clone, Debug)]
pub struct JabState {
    pub followup_pressed: bool,
    pub rapid_edges: i32,
    /// The jabs write only mv.co.attack1.x0 (ftCo_Attack1.c:126, 194), so
    /// mv+4 is the predecessor's word (`None` where the port does not model it).
    pub retained_word: Option<f32>,
}
impl Fighter {
    /// Grounded attack priority; checkAttack11 (8008ABC0), AttackHi3 doEnter (8008BA38).
    pub(super) fn enter_ground_attack(&mut self, assets: &FighterAssets) -> Result<()> {
        // Attack predicates share a transition enum; preserve the retail priority.
        let context = WaitContext {
            facing: self.core.physics.facing,
            ..WaitContext::default()
        };
        if let Some(held) = self.core.held_item {
            return self.enter_held_item_attack(held, assets, &context);
        }
        if self.first_ground_transition(assets, &context, &[P::SmashSide]) != T::None {
            return self.enter_forward_smash(assets);
        }
        for (predicate, state) in [
            (P::SmashUp, S::AttackHi4),
            (P::SmashDown, S::AttackLw4),
            (P::TiltSide, S::AttackS3S),
            (P::TiltUp, S::AttackHi3),
            (P::TiltDown, S::AttackLw3),
        ] {
            if self.first_ground_transition(assets, &context, &[predicate]) != T::None {
                // decideAngle (8008B788): an item in reach is picked up instead.
                if state == S::AttackS3S && self.try_item_pickup(assets)? {
                    return Ok(());
                }
                let state = if state == S::AttackS3S {
                    self.side_tilt_state(assets)
                } else {
                    state
                };
                return self.enter_simple_attack(state, assets);
            }
        }
        self.enter_jab_or_combo(assets)
    }
    /// Wait's attack checks with an item in hand. A smash with A is already
    /// ftCo_Catch_CheckInput's throw, so these are C-stick smashes: side
    /// (ftCo_AttackS4.c checkItemThrow) throws LightThrowF4/B4 by the stick's
    /// sign, up and down (ftCo_AttackHi4/Lw4_CheckInput with ftCo_800DF30C /
    /// ftCo_800DF3DC) LightThrowHi4/Lw4. Tilts throw LightThrowF/Hi/Lw
    /// (ftCo_AttackS3/Hi3/Lw3_CheckInput); ftCo_Attack1_CheckInput
    /// (8008A9F8) throws a throwable item forward.
    fn enter_held_item_attack(
        &mut self,
        held: super::item_pickup::HeldItem,
        assets: &FighterAssets,
        context: &WaitContext,
    ) -> Result<()> {
        for predicate in [P::SmashSide, P::SmashUp, P::SmashDown] {
            if self.first_ground_transition(assets, context, &[predicate]) == T::None {
                continue;
            }
            if predicate == P::SmashSide {
                let (sign, main) = self.side_smash_sign(assets);
                return self.enter_side_smash_with_item(held, sign, main, assets);
            }
            if held.use_kind != 0 {
                unimplemented!(
                    "ftCo_AttackHi4/Lw4_CheckInput: a smash with a held item of kind {}",
                    held.use_kind
                );
            }
            let state = match predicate {
                P::SmashUp => S::LightThrowHi4,
                _ => S::LightThrowLw4,
            };
            return self.enter_item_throw(state, assets);
        }
        for predicate in [P::TiltSide, P::TiltUp, P::TiltDown] {
            if self.first_ground_transition(assets, context, &[predicate]) == T::None {
                continue;
            }
            // ftCo_AttackS3_CheckInput throws on a held shoulder or a
            // throwable item; AttackHi3/Lw3 on ftCo_80094E54 (the same test).
            let throws = self.core.item_throw_pressed()
                || (predicate == P::TiltSide
                    && self.core.input.current.held.intersects(Buttons::SHIELD));
            if !throws && predicate == P::TiltSide && held.use_kind == 2 {
                // ftCo_AttackS3_CheckInput: a battering item swings.
                return self.enter_item_swing(super::item_swing::SwingInput::Tilt, assets);
            }
            if !throws {
                unimplemented!(
                    "a tilt with a non-throwable held item of kind {}",
                    held.use_kind
                );
            }
            let state = match predicate {
                P::TiltSide => S::LightThrowF,
                P::TiltUp => S::LightThrowHi,
                _ => S::LightThrowLw,
            };
            return self.enter_item_throw(state, assets);
        }
        // ftCo_Attack1_CheckInput (8008A9F8): a throwable item is thrown
        // forward; otherwise a shoulder drops it, and a battering item swings.
        if held.use_kind == 0 {
            return self.enter_item_throw(S::LightThrowF, assets);
        }
        if self.core.input.current.held.intersects(Buttons::SHIELD) {
            return self.enter_item_throw(S::LightThrowDrop, assets);
        }
        if held.use_kind == 2 {
            return self.enter_item_swing(super::item_swing::SwingInput::Jab, assets);
        }
        unimplemented!(
            "ftCo_Attack1_CheckInput: using a held item of kind {}",
            held.use_kind
        );
    }
    /// checkItemThrow (8008C22C), ftCo_AttackS4.c: a side smash with an item
    /// in hand. A shoulder, a throwable item or a C-stick smash
    /// (ftCo_800DF21C) throws it, forward or backward by the stick's sign
    /// against the facing, which does not turn; a battering item swings
    /// toward the stick. `main` says the main stick made the smash.
    pub(super) fn enter_side_smash_with_item(
        &mut self,
        held: super::item_pickup::HeldItem,
        sign: f32,
        main: bool,
        assets: &FighterAssets,
    ) -> Result<()> {
        let shoulder = self.core.input.current.held.intersects(Buttons::SHIELD);
        if held.use_kind == 2 && !shoulder {
            if !main {
                unimplemented!("checkItemThrow: a C-stick smash with a swing item");
            }
            self.core.physics.facing = sign;
            return self.enter_item_swing(super::item_swing::SwingInput::Smash, assets);
        }
        if held.use_kind != 0 && !shoulder {
            unimplemented!(
                "checkItemThrow: a smash with a held item of kind {}",
                held.use_kind
            );
        }
        let state = if sign * self.core.physics.facing >= 0.0 {
            S::LightThrowF4
        } else {
            S::LightThrowB4
        };
        self.enter_item_throw(state, assets)
    }
    /// ftCo_AttackS4_CheckInput's stick_x_sign: checkLStick picks the main
    /// stick's sign, else the C-stick's (ftCo_800DF1C8). Also whether the
    /// main stick made the smash.
    fn side_smash_sign(&self, assets: &FighterAssets) -> (f32, bool) {
        let input = &self.core.input;
        let t = &assets.input.thresholds;
        let main = input.pressed.intersects(Buttons::A)
            && gekko_math::msl::fabsf(input.current.stick.x) >= t.dash_smash_stick_threshold
            && i32::from(input.horizontal.tilt) < t.dash_smash_window;
        let x = if main {
            input.current.stick.x
        } else {
            input.current.cstick.x
        };
        (if x >= 0.0 { 1.0 } else { -1.0 }, main)
    }
    /// decideAttack11 / doAttack12Rapid (ftCo_Attack1.c:89-98, 171-181): the
    /// kind's own first jab, or checkAttack11 (8008ABC0), also used by a
    /// looping jab combo.
    fn enter_jab(&mut self, assets: &FighterAssets) -> Result<()> {
        if let Some(enter) = self.character.table().enter_jab {
            return enter(self, assets);
        }
        let variant = self.character.jab_variant();
        // getMotionFlags installs onPkPc21EC, which this very state change
        // runs before the frame-zero script: ft_800892A0's new attack
        // instance (ft_80089824 and Ft_MF_SkipAttackCount only touch
        // statistics).
        let new_instance = variant == super::JabVariant::Repeating;
        self.enter_jab_row(S::Attack11.into(), new_instance, assets)?;
        Ok(())
    }
    /// checkAttack11's body (8008ABC0) for the row `action`, which a kind's
    /// own entry shares (ftGw_Attack11_Enter, 8014C07C). False when an item
    /// in reach was picked up instead.
    pub fn enter_jab_row(
        &mut self,
        action: super::ActionId,
        new_instance: bool,
        assets: &FighterAssets,
    ) -> Result<bool> {
        if self.try_item_pickup(assets)? {
            return Ok(false);
        }
        if new_instance {
            self.core.combat.stale.new_instance();
        }
        self.core.commands.jab_followup = false;
        self.core.commands.rapid_jab = false;
        let retained_word = self.inherited_scratch_word();
        self.change_motion_state(action, assets)?;
        self.step_animation(assets);
        self.core.jab_countdown = self.core.attributes.combat.jab_2_input_window;
        self.core.last_jab = Some(S::Attack11);
        self.core.status.interaction = super::Interaction::Attack;
        self.core.state_data = MotionData::Jab(JabState {
            followup_pressed: false,
            rapid_edges: 0,
            retained_word,
        });
        Ok(true)
    }
    /// ftCo_Attack1_CheckInput (8008A9F8) with A: an open combo window
    /// (hitlag_mul, which survives into Wait and Walk) with the script's
    /// combo flag (x2218_b1) continues the last jab (unk_msid), else a new
    /// jab starts (decideAttack11).
    fn enter_jab_or_combo(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.core.jab_countdown > 0.0 && self.core.commands.jab_followup {
            let last = self.core.last_jab.expect("a jab window without a jab");
            return self.continue_jab_combo(last, assets);
        }
        self.enter_jab(assets)
    }
    /// doAttack12 / doAttack13 (8008AE30 / 8008B194): Attack11 continues into
    /// Attack12, Attack12 into the character's third jab (Marth restarts
    /// Attack11 through doAttack12Rapid -> checkAttack11).
    fn continue_jab_combo(&mut self, last: S, assets: &FighterAssets) -> Result<()> {
        let state = if last == S::Attack11 {
            // doAttack12: Pikachu and Pichu restart through doAttack12Rapid.
            match self.character.jab_variant() {
                super::JabVariant::Standard => S::Attack12,
                super::JabVariant::Repeating => S::Attack11,
            }
        } else {
            self.character.third_jab_state()
        };
        if state == S::Attack11 {
            // checkAttack11's restart includes ftAnim_8006EBA4 and the jab-2
            // window, unlike jabs 2 and 3.
            return self.enter_jab(assets);
        }
        // doAttack12Normal / doAttack13: an item in reach is picked up
        // instead (ftpickupitem_80094790).
        if self.try_item_pickup(assets)? {
            return Ok(());
        }
        // mv.co.attack1 keeps the rapid-jab edges the union still holds.
        let edges = match &self.core.state_data {
            MotionData::Jab(jab) => jab.rapid_edges,
            _ => 0,
        };
        let retained_word = self.inherited_scratch_word();
        self.core.commands.jab_followup = false;
        self.change_motion_state(state.into(), assets)?;
        if state == S::Attack12 {
            self.core.jab_countdown = self.core.attributes.combat.jab_3_input_window;
            self.core.last_jab = Some(S::Attack12);
        }
        self.core.state_data = MotionData::Jab(JabState {
            followup_pressed: false,
            rapid_edges: edges,
            retained_word,
        });
        Ok(())
    }
    /// ftCo_Attack1_CheckInput reached without A counts the combo window
    /// down; it runs only when every earlier predicate of `predicates` fails.
    pub(super) fn count_down_jab_window(
        &mut self,
        assets: &FighterAssets,
        context: &WaitContext,
        predicates: &[P],
    ) {
        if self.core.jab_countdown <= 0.0 {
            return;
        }
        for &predicate in predicates {
            if predicate == P::Jab {
                if !self.core.input.pressed.intersects(Buttons::A) {
                    self.core.jab_countdown -= 1.0;
                }
                return;
            }
            if crate::input::iasa::evaluate(predicate, &self.core.input, &assets.input, context)
                != T::None
            {
                return;
            }
        }
    }
    /// ftCo_AttackHi4 doEnter (8008CA38), AttackLw4 (8008CC5C),
    /// AttackHi3 (8008BA38), AttackLw3 (8008BC70), AttackDash (8008B4D4).
    pub(super) fn enter_simple_attack(&mut self, state: S, assets: &FighterAssets) -> Result<()> {
        // ftCo_AttackLw3.c decideFighter (:67-76): the kind's own down tilt.
        if state == S::AttackLw3 {
            if let Some(enter) = self.character.table().enter_down_tilt {
                return enter(self, assets);
            }
        }
        // AttackLw3 doEnter (8008BC70): an item in reach is picked up instead.
        if state == S::AttackLw3 && self.try_item_pickup(assets)? {
            return Ok(());
        }
        self.core.commands.allow_interrupt = false;
        self.core.commands.variables[0] = 0;
        if state == S::AttackLw3 {
            // doEnter (8008BC70) installs x21EC = callUnk (8008BC00), which
            // this state change runs (retail 0x8008BC14: bl ft_800892A0): a
            // new attack instance, so a down tilt repeated out of a down
            // tilt takes its own stale entry (ft_800890D0 alone keeps the
            // instance while the move id is unchanged).
            self.core.combat.stale.new_instance();
        }
        // AttackHi4, AttackLw4 and AttackHi3 write no mv field (AttackLw3
        // and AttackDash only +2340), so mv+4 stays the predecessor's.
        let retained_word = self.inherited_scratch_word();
        self.change_motion_state(state.into(), assets)?;
        self.step_animation(assets);
        self.core.status.interaction = super::Interaction::Attack;
        self.core.state_data = match state {
            S::AttackLw3 => MotionData::DownTilt {
                repeat_pressed: false,
                retained_word,
            },
            // doEnter (8008B4D4): the dash-grab window starts closed.
            S::AttackDash => MotionData::DashAttack {
                grab_window: 0,
                retained_word,
            },
            _ => MotionData::Tilt { retained_word },
        };
        Ok(())
    }
    /// ftCo_AttackDash_CheckInput from Dash, Run and RunDirect, then
    /// ftCo_AttackDash_SetMv0 (8008B570): open the dash-grab window (PlCo +68).
    pub(super) fn enter_dash_attack(&mut self, assets: &FighterAssets) -> Result<()> {
        self.enter_simple_attack(S::AttackDash, assets)?;
        let MotionData::DashAttack { grab_window, .. } = &mut self.core.state_data else {
            unreachable!("enter_simple_attack(AttackDash) installs DashAttack")
        };
        *grab_window = gekko_math::msl::fctiwz(assets.running.shield_grab_delay);
        Ok(())
    }
    /// ftCo_AttackDash_IASA (8008B5C4) -> ftCo_800D8AE0 (800D8AE0): holding
    /// shield while the window is open cancels into CatchDash; otherwise the
    /// window closes by one frame and the ordinary interrupt applies.
    pub(super) fn dash_attack_input(
        &mut self,
        assets: &FighterAssets,
        context: &WaitContext,
    ) -> Result<()> {
        // ftCo_800952DC throws a held item first; no supported fighter holds one here.
        let shield = self
            .core
            .input
            .current
            .held
            .intersects(crate::input::Buttons::SHIELD);
        // fn_800D8E94 returns before the window's countdown.
        if self.core.tether_article {
            return self.tilt_input(assets, context);
        }
        let MotionData::DashAttack { grab_window, .. } = &mut self.core.state_data else {
            panic!("dash attack scratch")
        };
        if shield && *grab_window != 0 {
            return self.enter_catch_motion(S::CatchDash, assets);
        }
        if *grab_window != 0 {
            *grab_window -= 1;
        }
        self.tilt_input(assets, context)
    }
    /// ftCo_AttackLw3_Anim (8008BCFC): repeat latch, then SquatWait on completion.
    pub(super) fn down_tilt_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::DownTilt { repeat_pressed, .. } = self.core.state_data else {
            panic!("down tilt scratch")
        };
        if self.core.commands.variables[0] != 0 && repeat_pressed {
            return self.enter_simple_attack(S::AttackLw3, assets);
        }
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.change_motion_state(S::SquatWait.into(), assets)?;
            self.core.state_data = MotionData::Squat(super::squat::SquatState::default());
        }
        Ok(())
    }
    /// A kind's own grounded IASA: the common ftCo_*_CheckInput calls in
    /// its order (ftGw_AttackLw3_IASA, 8014AE78). ftCo_Attack1_CheckInput
    /// reached without A counts the jab window down.
    pub fn interrupt_ground(&mut self, assets: &FighterAssets, predicates: &[P]) -> Result<()> {
        let context = self.core.wait_context();
        let transition = self.first_ground_transition(assets, &context, predicates);
        self.count_down_jab_window(assets, &context, predicates);
        self.apply_ground_transition(assets, transition)
    }
    /// ftCo_AttackLw3_IASA (8008BD80): latch A before ordinary movement interrupts.
    pub(super) fn down_tilt_input(
        &mut self,
        assets: &FighterAssets,
        context: &WaitContext,
    ) -> Result<()> {
        if self.core.commands.allow_interrupt
            && self.first_ground_transition(
                assets,
                context,
                &[
                    P::SmashSide,
                    P::SmashUp,
                    P::SmashDown,
                    P::TiltSide,
                    P::TiltUp,
                ],
            ) != T::None
        {
            return self.enter_ground_attack(assets);
        }
        if self.core.input.pressed.intersects(Buttons::A) {
            if self.core.commands.variables[0] != 0 {
                return self.enter_simple_attack(S::AttackLw3, assets);
            }
            let MotionData::DownTilt { repeat_pressed, .. } = &mut self.core.state_data else {
                panic!("down tilt scratch")
            };
            *repeat_pressed = true;
        }
        if self.core.commands.allow_interrupt {
            let transition = self.first_ground_transition(
                assets,
                context,
                &[
                    P::TiltDown,
                    P::Jab,
                    P::Jump,
                    P::Dash,
                    // ftCo_AttackLw3_IASA calls the pure check: down held
                    // keeps the tilt and skips Turn/Walk.
                    P::SquatHeld,
                    P::Turn,
                    P::Walk,
                ],
            );
            self.apply_ground_transition(assets, transition)?;
        }
        Ok(())
    }
    /// ftCo_Attack100Start_Anim (800D6C0C): loop entry does not advance animation.
    pub(super) fn rapid_start_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.change_motion_state(S::Attack100Loop.into(), assets)?;
        }
        Ok(())
    }
    /// ftCo_Attack100Loop_Anim (800D6D48): evaluate the script's end flag before IASA.
    pub(super) fn rapid_loop_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if self.rapid_loop_ended(assets)? {
            self.change_motion_state(S::Attack100End.into(), assets)?;
        }
        Ok(())
    }
    /// ftCo_800D6C60 (800D6C60) up to its callback: true when the loop ends
    /// and the caller's end entry runs (ftGw_Attack100Loop_Anim passes
    /// ftGw_Attack100End_Enter).
    pub fn rapid_loop_ended(&mut self, assets: &FighterAssets) -> Result<bool> {
        let MotionData::RapidJab(rapid) = &mut self.core.state_data else {
            panic!("rapid jab scratch")
        };
        if self.core.animation.frame >= 0.0 && self.core.animation.frame < self.core.animation.speed
        {
            rapid.loop_started = true;
            self.core.combat.stale.new_instance();
        }
        if std::mem::take(&mut self.core.commands.rapid_jab_loop_end) {
            if rapid.loop_started && !rapid.edge_pressed {
                return Ok(true);
            } else if !self.try_item_pickup(assets)? {
                let MotionData::RapidJab(rapid) = &mut self.core.state_data else {
                    panic!("rapid jab scratch")
                };
                rapid.edge_pressed = false;
            }
        }
        Ok(false)
    }
    /// ftCo_Attack11_Anim (8008AC9C).
    pub(super) fn jab_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        if !self.core.animation.frames_remaining(&self.core.skeleton) {
            self.change_motion_state(S::Wait.into(), assets)?;
        }
        Ok(())
    }
    /// ftCo_AttackHi3_IASA (8008BAD4): Wait predicates after script unlock.
    pub(super) fn tilt_input(
        &mut self,
        assets: &FighterAssets,
        context: &WaitContext,
    ) -> Result<()> {
        if self.core.commands.allow_interrupt {
            let transition = crate::input::wait_iasa(&self.core.input, &assets.input, context);
            self.apply_ground_transition(assets, transition)?;
        }
        Ok(())
    }
    /// ftCo_AttackS4_IASA (8008C55C): Link's smash42 combo (ftCo_800CECE8)
    /// even before the interrupt, then Wait's checks without the spot dodge.
    pub(super) fn forward_smash_input(
        &mut self,
        assets: &FighterAssets,
        context: &WaitContext,
    ) -> Result<()> {
        // ftCo_800CECE8: the script's combo window and a fresh A press.
        if self.core.commands.variables[0] != 0
            && self.core.input.pressed.intersects(crate::input::Buttons::A)
        {
            return self.enter_forward_smash_combo(assets);
        }
        if self.core.commands.allow_interrupt {
            let transition = crate::input::iasa_with_predicates(
                crate::input::FORWARD_SMASH_PREDICATES,
                &self.core.input,
                &assets.input,
                context,
            );
            self.apply_ground_transition(assets, transition)?;
        }
        Ok(())
    }
    /// ftCo_800CED30 (800CED30): the second forward smash, entered at frame
    /// zero without motion flags, then ftAnim_8006EBA4. The smash rows
    /// write no mv field, so mv+4 stays the first hit's.
    fn enter_forward_smash_combo(&mut self, assets: &FighterAssets) -> Result<()> {
        let Some(state) = self.character.table().forward_smash_combo else {
            unimplemented!("ftCo_800CED30: don't have smash42 motion!!!");
        };
        let retained_word = self.inherited_scratch_word();
        self.core.commands.allow_interrupt = false;
        self.change_motion_state_with_flags(state, assets, super::MotionEntryFlags(0), 0.0, 1.0)?;
        self.step_animation(assets);
        self.core.state_data = MotionData::Smash { retained_word };
        Ok(())
    }
    /// ftCo_Attack13_IASA (8008B390): the rapid-jab check runs before the
    /// interrupt (bl ftCo_Attack_800D6A50 at 8008B3AC), then Wait's checks
    /// once the script unlocks them (8008B3C8).
    pub(super) fn third_jab_input(
        &mut self,
        assets: &FighterAssets,
        context: &WaitContext,
    ) -> Result<()> {
        if self.try_rapid_jab(assets)? {
            return Ok(());
        }
        self.tilt_input(assets, context)
    }
    /// ftCo_Attack_800D6A50 (800D6A50): count both A edges (x1A54); once the
    /// script opens the rapid jab (x2218_b2) with enough edges, enter
    /// Attack100Start (fn_800D6AC4 -> ftCo_800D6B00). True when the check
    /// consumed the tick, including an item picked up instead.
    fn try_rapid_jab(&mut self, assets: &FighterAssets) -> Result<bool> {
        let MotionData::Jab(jab) = &mut self.core.state_data else {
            panic!("jab scratch missing")
        };
        if (self.core.input.pressed | self.core.input.released).intersects(Buttons::A) {
            jab.rapid_edges += 1;
        }
        if !self.core.commands.rapid_jab
            || jab.rapid_edges < self.core.attributes.combat.rapid_jab_window
        {
            return Ok(false);
        }
        // fn_800D6AC4 (800D6AC4): the kind's own start, or fn_800D6B8C.
        match self.character.table().enter_rapid_jab {
            Some(enter) => enter(self, assets)?,
            None => {
                self.enter_rapid_jab_row(S::Attack100Start.into(), assets)?;
            }
        }
        Ok(true)
    }
    /// ftCo_800D6B00 (800D6B00) for the row `action`: an item in reach is
    /// picked up instead (false).
    pub fn enter_rapid_jab_row(
        &mut self,
        action: super::ActionId,
        assets: &FighterAssets,
    ) -> Result<bool> {
        if self.try_item_pickup(assets)? {
            return Ok(false);
        }
        self.core.commands.rapid_jab_loop_end = false;
        self.change_motion_state(action, assets)?;
        self.step_animation(assets);
        self.core.state_data = MotionData::RapidJab(RapidJabState::default());
        Ok(true)
    }
    /// ftCo_Attack11_IASA (8008ACD8), checkAttack12 (8008AF0C).
    pub(super) fn jab_input(
        &mut self,
        assets: &FighterAssets,
        context: &WaitContext,
    ) -> Result<()> {
        if self.core.commands.allow_interrupt
            && self.first_ground_transition(
                assets,
                context,
                &[
                    P::SmashSide,
                    P::SmashUp,
                    P::SmashDown,
                    P::TiltSide,
                    P::TiltUp,
                    P::TiltDown,
                ],
            ) != T::None
        {
            return self.enter_ground_attack(assets);
        }
        if self.try_rapid_jab(assets)? {
            return Ok(());
        }
        let MotionData::Jab(jab) = &mut self.core.state_data else {
            panic!("jab scratch missing")
        };
        // checkAttack12 / checkAttack13: the window counts down, noting A.
        if self.core.jab_countdown > 0.0 {
            self.core.jab_countdown -= 1.0;
            if self.core.input.pressed.intersects(Buttons::A) {
                jab.followup_pressed = true;
            }
        }
        if jab.followup_pressed && self.core.commands.jab_followup {
            let current = self.core.motion_state.id;
            return self.continue_jab_combo(current, assets);
        }
        if self.core.commands.allow_interrupt {
            let transition = self.first_ground_transition(
                assets,
                context,
                &[P::Jump, P::Dash, P::Squat, P::Turn, P::Walk],
            );
            self.apply_ground_transition(assets, transition)?;
        }
        Ok(())
    }
}
impl FighterCore {
    /// ftCo_AttackS3 decideAngle (8008B788): absent animation data rejects angles.
    fn side_tilt_state(&self, assets: &FighterAssets) -> S {
        let stick = self.input.current.stick;
        let angle = melee_lb::trigf::atan2f(stick.y, gekko_math::msl::fabsf(stick.x));
        let [high, high_middle, low_middle, low] = assets.attacks.tilt_angles;
        for (selected, state) in [
            (angle > high, S::AttackS3Hi),
            (angle > high_middle, S::AttackS3HiS),
            (angle < low, S::AttackS3Lw),
            (angle < low_middle, S::AttackS3LwS),
        ] {
            if selected && assets.motions.contains_key(&(state as i32 + 2)) {
                return state;
            }
        }
        S::AttackS3S
    }
    /// ftCo_AttackDash_Phys (8008B600): fixed friction, root-motion path shared with jab.
    pub(super) fn dash_attack_physics(
        &mut self,
        assets: &FighterAssets,
        map: &melee_mp::CollMap,
        wind: Wind,
    ) {
        if self
            .animation
            .flags
            .contains(crate::anim::MotionFlags::ROOT_MOTION)
        {
            return self.jab_physics(assets, map, wind);
        }
        use crate::physics::{
            friction::friction_acceleration,
            grounded::{self, GroundedParameters},
        };
        let params = GroundedParameters::from_attributes(&self.attributes, &assets.common);
        // retail ftCo_AttackDash_Phys: fmuls, no fused instructions.
        let friction =
            assets.attacks.dash_friction_multiplier * self.attributes.ground.ground_friction;
        self.physics.ground_acceleration =
            friction_acceleration(self.physics.ground_velocity, friction);
        grounded::apply_ground_movement(
            &mut self.physics,
            self.collision.data.floor.normal,
            map.floor_speed_scale(&self.collision.data),
        );
        grounded::finish_ground_update(&mut self.physics, &self.collision.data, &params, map, wind);
    }
    /// ftCo_Attack11_Phys (8008ADF0) -> ft_80084FA8 (80084FA8).
    pub(super) fn jab_physics(
        &mut self,
        assets: &FighterAssets,
        map: &melee_mp::CollMap,
        wind: Wind,
    ) {
        use crate::physics::grounded::{self, GroundedParameters};
        let params = GroundedParameters::from_attributes(&self.attributes, &assets.common);
        if self
            .animation
            .flags
            .contains(crate::anim::MotionFlags::ROOT_MOTION)
        {
            let offset = self
                .animation
                .root_motion
                .as_ref()
                .expect("jab TransN")
                .primary_history
                .offset
                .z;
            // retail ft_80085030, 8008505C: fmsubs.
            self.physics.ground_acceleration =
                gekko_math::fma::fmsubs(offset, self.physics.facing, self.physics.ground_velocity);
            grounded::apply_ground_movement(
                &mut self.physics,
                self.collision.data.floor.normal,
                map.floor_speed_scale(&self.collision.data),
            );
        } else {
            grounded::friction_physics(
                &mut self.physics,
                &params,
                self.collision.data.floor.normal,
                map.floor_speed_scale(&self.collision.data),
            );
        }
        grounded::finish_ground_update(&mut self.physics, &self.collision.data, &params, map, wind);
    }

    /// ft_80085004 (80085004), the grounded throws' physics: ft_80085030
    /// with facing_dir1, so the root motion keeps the entry facing through
    /// the script's reversal, and the plain ground friction (no scaling
    /// above walk speed) when the animation has none.
    pub(super) fn throw_physics(
        &mut self,
        assets: &FighterAssets,
        map: &melee_mp::CollMap,
        wind: Wind,
    ) {
        use crate::physics::grounded::{self, GroundedParameters};
        let params = GroundedParameters::from_attributes(&self.attributes, &assets.common);
        if self
            .animation
            .flags
            .contains(crate::anim::MotionFlags::ROOT_MOTION)
        {
            let offset = self
                .animation
                .root_motion
                .as_ref()
                .expect("throw TransN")
                .primary_history
                .offset
                .z;
            // retail ft_80085030, 8008505C: fmsubs.
            self.physics.ground_acceleration = gekko_math::fma::fmsubs(
                offset,
                self.physics.entry_facing,
                self.physics.ground_velocity,
            );
        } else {
            self.physics.ground_acceleration = crate::physics::friction::friction_acceleration(
                self.physics.ground_velocity,
                self.attributes.ground.ground_friction,
            );
        }
        grounded::apply_ground_movement(
            &mut self.physics,
            self.collision.data.floor.normal,
            map.floor_speed_scale(&self.collision.data),
        );
        grounded::finish_ground_update(&mut self.physics, &self.collision.data, &params, map, wind);
    }
}

/// PlCo direction thresholds and dash-attack friction (ftCo_AttackS3/AttackDash).
pub struct AttackParameters {
    pub tilt_angles: [f32; 4],
    pub smash_angles: [f32; 4],
    pub dash_friction_multiplier: f32,
    pub charge_sound_frame: f32,
}
impl AttackParameters {
    pub fn read(archive: &hsd_archive::Archive, offset: u32) -> Result<Self> {
        let r = archive.reader();
        Ok(Self {
            tilt_angles: [
                r.f32(offset + 0x9C)?,
                r.f32(offset + 0xA0)?,
                r.f32(offset + 0xA4)?,
                r.f32(offset + 0xA8)?,
            ],
            smash_angles: [
                r.f32(offset + 0xB8)?,
                r.f32(offset + 0xBC)?,
                r.f32(offset + 0xC0)?,
                r.f32(offset + 0xC4)?,
            ],
            dash_friction_multiplier: r.f32(offset + 0x50)?,
            charge_sound_frame: r.f32(offset + 0x7C8)?,
        })
    }
}

#[derive(Clone, Debug, Default)]
pub struct RapidJabState {
    pub loop_started: bool,
    pub edge_pressed: bool,
}
