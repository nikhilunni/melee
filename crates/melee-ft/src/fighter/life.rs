//! Stock loss and revival, ft_0D31.c / ft_0D4D.c.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    Fighter, MotionData,
};
use gekko_math::{fma::fmsubs, HsdRng};
use hsd_types::Vec3;
use melee_ef::request::EffectRequest;
use melee_types::{CommonMotionState as S, GroundOrAir};

#[derive(Clone, Copy, Debug)]
pub struct Arena {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
    pub camera_top: f32,
    pub revival_positions: [Vec3; 4],
    /// grLast stage initialization (801DFF18): stage_info.unk8C.b4.
    pub player_revival_markers: bool,
}
#[derive(Clone, Debug)]
pub enum LifeState {
    /// DeadDown / DeadLeft / DeadRight (and a star KO once it vanishes): the x40
    /// countdown to the GM respawn request.
    Dead {
        remaining: i32,
    },
    /// DeadUpStar before the vanish: x40 countdown, x44 phase 0 (hold) / 1 (flight).
    /// `camera_top` is Stage_GetCamBoundsTopOffset, sampled at entry for the flight aim.
    StarKo {
        remaining: i32,
        flying: bool,
        camera_top: f32,
    },
    /// DeadUpFall and its camera hit: a flight in the camera's space.
    ScreenKo(ScreenKo),
    AwaitingRespawn,
    Revival {
        remaining: i32,
        target: Vec3,
    },
    PlatformWait {
        remaining: i32,
        target: Vec3,
    },
}
impl FighterCore {
    /// x2219_b1: a death entry (ftCo_800D3680..ftCo_800D481C) or the respawn
    /// wait (ftCo_800D4F24) sets it after its motion change, and every
    /// motion change clears it. Fighter_8006CB94 skips all hit detection
    /// while it is set; Revival does not set it.
    pub fn out_of_play(&self) -> bool {
        matches!(
            self.state_data,
            MotionData::Life(
                LifeState::Dead { .. }
                    | LifeState::StarKo { .. }
                    | LifeState::ScreenKo(_)
                    | LifeState::AwaitingRespawn
            )
        )
    }
}
impl LifeState {
    /// Between ftCo_800D34E0's stock loss and the respawn request: the HUD
    /// percent explodes (ifStatus_PercentOnDeathAnimationThink) in this window.
    pub fn stock_lost(&self) -> bool {
        match self {
            LifeState::Dead { .. } => true,
            LifeState::ScreenKo(ko) => ko.phase == ScreenKoPhase::Vanished,
            _ => false,
        }
    }
}
/// Fighter accessory JObj, loaded from PlCo ftLoadCommonData[8].
#[derive(Clone, Debug)]
pub struct RevivalPlatform {
    pub tree: hsd_anim::jobj::JObjTree,
    pub root: hsd_anim::jobj::JObjId,
}
pub struct LifeParameters {
    /// +500: frames from a side/bottom death entry to the respawn request.
    pub death_delay: i32,
    pub revival_duration: i32,
    pub platform_duration: i32,
    pub invincibility_duration: i32,
    /// +4F4: uniform scale of the blast-zone explosion (efSync 0x42B).
    pub death_effect_scale: f32,
    /// +4F0: a top exit only counts with upward knockback above this (or grounded).
    pub top_knockback_threshold: f32,
    pub star: StarKoParameters,
    pub screen_ko: ScreenKoParameters,
    pub death_sounds: DeathSounds,
}

/// PlCo +504..+514: the star KO (ftCo_DeadUpStar_Anim).
#[derive(Clone, Copy, Debug)]
pub struct StarKoParameters {
    /// +504: frames held at the exit position.
    pub hold: i32,
    /// +508: frames of flight toward the background.
    pub flight: i32,
    /// +50C: frames from the vanish to the respawn request.
    pub vanish_delay: i32,
    /// +510: total z travel over the flight.
    pub depth: f32,
    /// +514: the flight aims at this fraction of the camera-bounds top.
    pub height_ratio: f32,
}

/// PlCo +520..+55C: the screen KO (ftCo_DeadUpFall_*).
#[derive(Clone, Copy, Debug)]
pub struct ScreenKoParameters {
    /// +520: `HSD_Randi(100) + 1 <= threshold` picks the screen KO over the star KO.
    pub threshold: i32,
    /// +524: frames held at the start position before the approach.
    pub hold: i32,
    /// +528: frames of the approach toward the screen.
    pub approach_frames: i32,
    /// +52C: frames stuck to the screen.
    pub impact_hold: i32,
    /// +530: frames of the fall down the screen.
    pub fall_frames: i32,
    /// +534: frames from the vanish to the respawn request.
    pub vanish_delay: i32,
    /// +538 / +544: the approach's camera-space endpoints.
    pub start: Vec3,
    pub end: Vec3,
    /// +550 / +55C: the fall's initial vertical and depth speed.
    pub fall_speed_y: f32,
    pub fall_speed_z: f32,
    /// +554 / +558: the fall's gravity and terminal speed.
    pub gravity: f32,
    pub terminal_velocity: f32,
}

/// mv.co.unk_deadup, fighter +2340..+2368 (non-ice variant, x68 = 0).
#[derive(Clone, Debug)]
pub struct ScreenKo {
    /// +2340: frames left in this phase.
    pub remaining: i32,
    /// +2344.
    pub phase: ScreenKoPhase,
    /// +2348 / +234C: the approach's per-frame step and progress.
    pub step: f32,
    pub progress: f32,
    /// +2350: the fighter's camera-space position; each display pass places
    /// the fighter through the second camera's inverse view matrix.
    pub position: Vec3,
    /// +235C: this frame's camera-space fall displacement.
    pub displacement: Vec3,
}

/// ftCo_SM_DeadUpFallHitCamera (0) and ftCo_SM_DeadUpFallHitCameraFlat (1).
pub const MOTIONS: &[u32] = &[0, 1];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenKoPhase {
    Hold = 0,
    Approach = 1,
    Impact = 2,
    Fall = 3,
    Vanished = 4,
}

/// lbl_803B7500: the root faces into the screen ({0, pi, 0}).
const FACING_THE_SCREEN: hsd_anim::quat::Quaternion = hsd_anim::quat::Quaternion {
    x: 0.0,
    y: std::f32::consts::PI,
    z: 0.0,
    w: 0.0,
};

/// ft_data->x4C_sfx +4 / +8 / +C: the fighter's death voice ids.
#[derive(Clone, Copy, Debug)]
pub struct DeathSounds {
    /// ftCo_800D38B8 plays both on the voice channel at every death entry.
    pub cries: [u32; 2],
    /// ftCo_800D40B8: the star KO cry.
    pub star: u32,
}

/// Which side blast zone a fighter left through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Side {
    Left,
    Right,
}
impl Fighter {
    /// gm_8016719C -> Player_80032070 -> Fighter_UnkProcessDeath (80068354).
    pub fn reset_for_revival(
        &mut self,
        assets: &FighterAssets,
        arena: &Arena,
        context: super::SpawnContext<'_>,
    ) -> Result<()> {
        if !arena.player_revival_markers {
            unimplemented!("gm_80167638: shared revival marker offset allocation");
        }
        let target = arena.revival_positions[usize::from(self.core.player.id)];
        self.core.player.position = Vec3::new(target.x, arena.camera_top, 0.0);
        self.core.player.facing = if self.core.player.position.x >= 0.0 {
            -1.0
        } else {
            1.0
        };
        self.core.player.damage = 0.0;
        self.core.reset_life(assets, context.map);
        self.install_motion_row(super::state::COMMON[S::Wait as usize]);
        let scale = self.core.skeleton.scale(self.core.animation.root);
        self.reset_spawn_services(assets, context, scale);
        self.enter_revival(assets, target)
    }
    /// ftCo_800D3158 (800D3158), after Update: blast-zone exits in retail order
    /// (right, left, top, bottom).
    pub fn check_blast_zone(
        &mut self,
        assets: &FighterAssets,
        arena: &Arena,
        rng: &mut HsdRng,
    ) -> Result<()> {
        if matches!(self.core.state_data, MotionData::Life(_))
            || self.core.status.disabled
            || self.core.status.ledge_grab_disabled
        {
            return Ok(());
        }
        let p = self.core.physics.position;
        if p.x > arena.right {
            return self.enter_side_death(assets, arena, Side::Right);
        }
        if p.x < arena.left {
            return self.enter_side_death(assets, arena, Side::Left);
        }
        if p.y > arena.top {
            let counts = self.core.physics.ground_or_air == GroundOrAir::Ground
                || self.core.status.unconditional_top_exit
                || self.core.physics.knockback_velocity.y > assets.life.top_knockback_threshold;
            if counts {
                // Player_GetMoreFlagsBit5 (plain DeadUp) and Camera_8003010C (the fixed
                // camera) are both off in a Vs match; DamageIce victims stop in damage.rs.
                let roll = rng.randi(100) + 1;
                return if assets.life.screen_ko.threshold >= roll {
                    self.enter_screen_ko(assets)
                } else {
                    self.enter_star_ko(assets, arena)
                };
            }
        }
        if p.y < arena.bottom {
            return self.enter_bottom_death(assets, arena);
        }
        Ok(())
    }
    /// ftCo_800D3BC8 (800D3BC8): the bottom exit, DeadDown. The explosion is
    /// clamped to the side blast zones and drawn upright.
    fn enter_bottom_death(&mut self, assets: &FighterAssets, arena: &Arena) -> Result<()> {
        let mut effect = self.core.physics.position;
        if effect.x > arena.right {
            effect.x = arena.right;
        }
        if effect.x < arena.left {
            effect.x = arena.left;
        }
        self.enter_death(assets, S::DeadDown, 0x61, effect, 0.0)
    }
    /// ftCo_800D3680 / ftCo_800D3950 (800D3680 / 800D3950): the side exits, DeadLeft /
    /// DeadRight. The explosion is clamped to the top/bottom blast zones and rotated
    /// a quarter turn toward the exit.
    fn enter_side_death(
        &mut self,
        assets: &FighterAssets,
        arena: &Arena,
        side: Side,
    ) -> Result<()> {
        let mut effect = self.core.physics.position;
        if effect.y > arena.top {
            effect.y = arena.top;
        }
        if effect.y < arena.bottom {
            effect.y = arena.bottom;
        }
        let (state, sound, angle) = match side {
            Side::Left => (S::DeadLeft, 0x88, -std::f32::consts::FRAC_PI_2),
            Side::Right => (S::DeadRight, 0x89, std::f32::consts::FRAC_PI_2),
        };
        self.enter_death(assets, state, sound, effect, angle)
    }
    /// Shared body of the DeadDown / DeadLeft / DeadRight entries: the fighter
    /// vanishes, loses the stock immediately and counts down PlCo +500.
    fn enter_death(
        &mut self,
        assets: &FighterAssets,
        state: S,
        exit_sound: u32,
        effect_position: Vec3,
        effect_angle: f32,
    ) -> Result<()> {
        self.release_for_death(assets);
        self.change_motion_state(state.into(), assets)?;
        self.core.state_data = MotionData::Life(LifeState::Dead {
            remaining: assets.life.death_delay,
        });
        // x2219_b1 / x221E_b1 / x221E_b2 / x221F_b1 are the dead flags MotionData::Life
        // stands for; pl_8003DF44 stamps the killer's stale-move table (no compared key).
        self.core.effect_state.invisible = true;
        // Camera_RequestQuake(QuakeKind_Large, &cur_pos); the scene forwards it
        // to the camera. The ftCo_800D35FC rumble has no simulated observer.
        self.core.quake_request = Some(melee_cm::QuakeKind::Large);
        self.core.lose_stock();
        self.core.play_death_sounds(assets, exit_sound);
        self.core.effects.push(EffectRequest::Death {
            position: effect_position,
            angle: effect_angle,
            scale: assets.life.death_effect_scale,
        });
        // ftCo_800D4E50: the coin-mode payout only.
        Ok(())
    }
    /// ftCo_800D40B8 (800D40B8): the star KO entry, DeadUpStar. The stock is only
    /// lost when the star vanishes (ftCo_DeadUpStar_Anim phase 1).
    fn enter_star_ko(&mut self, assets: &FighterAssets, arena: &Arena) -> Result<()> {
        self.release_for_death(assets);
        self.change_motion_state(S::DeadUpStar.into(), assets)?;
        self.core.state_data = MotionData::Life(LifeState::StarKo {
            remaining: assets.life.star.hold,
            flying: false,
            camera_top: arena.camera_top,
        });
        // ftCo_800D40B8_inline: dead flags and ft_80088C5C (stop the fighter's channels).
        // ftCommon_8007EFC0(fp, true): the nametag countdown.
        self.core.status.name_tag_timer = 1;
        // ft_800881D8(x4C_sfx->xC, 127, 64) on the voice channel.
        self.core.push_voice(assets.life.death_sounds.star);
        // pl_8003DF44 as above; x68 = 0 marks the non-ice variant.
        Ok(())
    }
    /// ftCo_DeadUpStar_Anim (800D42E4): hold for PlCo +504, then fly toward the
    /// background for PlCo +508 frames aiming at `height_ratio` of the camera top.
    pub(super) fn star_ko_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        self.step_animation(assets);
        let star = &assets.life.star;
        let position_y = self.core.physics.position.y;
        let MotionData::Life(LifeState::StarKo {
            remaining,
            flying,
            camera_top,
        }) = &mut self.core.state_data
        else {
            // Phase 2, after the vanish: the PlCo +50C countdown to the respawn request.
            self.core.death_animation();
            return Ok(());
        };
        // The ice variant (x68) also spins the XRotN bone here.
        if *remaining != 0 {
            *remaining -= 1;
        }
        if *remaining != 0 {
            return Ok(());
        }
        if *flying {
            return self.vanish_star_ko(assets);
        }
        // retail 800D4438: fmsubs, then fdivs by the int-converted frame count.
        let frames = star.flight as f32;
        let velocity_y = fmsubs(star.height_ratio, *camera_top, position_y) / frames;
        let velocity_z = star.depth / frames;
        *remaining = star.flight;
        *flying = true;
        self.core.physics.self_velocity.y = velocity_y;
        self.core.physics.self_velocity.z = velocity_z;
        Ok(())
    }
    /// ftCo_DeadUpStar_Anim phase 1 -> 2 (800D4484..800D4530): the star vanishes.
    fn vanish_star_ko(&mut self, assets: &FighterAssets) -> Result<()> {
        // ftCommon_8007E2FC; ftCommon_8007DB24 is the ice variant only.
        self.core.clear_velocities();
        // efAsync_Spawn(gobj, &x60C, 2, 0x42D, NULL, &cur_pos): efLib_CreateGenerator
        // 0x121 at the fighter position (root-relative, zero offset).
        self.core.effects.push(EffectRequest::Landing {
            id: 0x42D,
            offset: Vec3::ZERO,
            floor_angle: 0.0,
        });
        // ftCo_800D4E50: the coin-mode payout only. x221F_b1, the vanish, ft_80088C5C:
        self.core.effect_state.invisible = true;
        self.core.play_death_sounds(assets, 0x83);
        // ftCo_800D34E0: the stock is lost here, which also fires the HUD explosion.
        self.core.lose_stock();
        self.core.state_data = MotionData::Life(LifeState::Dead {
            remaining: assets.life.star.vanish_delay,
        });
        Ok(())
    }
    /// ftCo_800D4780 -> ftCo_800D4580 (800D4580): the screen KO entry, DeadUpFall.
    fn enter_screen_ko(&mut self, assets: &FighterAssets) -> Result<()> {
        self.release_for_death(assets);
        let parameters = &assets.life.screen_ko;
        self.change_motion_state(S::DeadUpFall.into(), assets)?;
        self.core.state_data = MotionData::Life(LifeState::ScreenKo(ScreenKo {
            remaining: parameters.hold,
            phase: ScreenKoPhase::Hold,
            step: 0.0,
            progress: 0.0,
            position: parameters.start,
            displacement: Vec3::ZERO,
        }));
        // x2220_b7 (the ScreenKo state): the render callback now places the
        // fighter from its camera-space position.
        self.face_the_screen();
        // Dead flags, ft_80088C5C, ftCommon_8007EFC0(fp, true):
        self.core.status.name_tag_timer = 1;
        // ftCo_800BFFD0(fp, 0x2B, 0): the screen-KO colour animation.
        self.core
            .commands
            .color_animations
            .push(melee_cmd::ColorAnimationRequest {
                id: 0x2B,
                duration: 0,
            });
        // pl_8003DF44 as above; x68 = 0 marks the non-ice variant.
        Ok(())
    }
    /// ftCo_800D4580 / ftCo_800D481C: HSD_JObjSetRotation(lbl_803B7500) when
    /// x34_scale.z is 1 (every supported fighter).
    fn face_the_screen(&mut self) {
        let root = self.core.animation.root;
        self.core.skeleton.set_rotation(root, &FACING_THE_SCREEN);
    }
    /// ftCo_DeadUpFall_Anim (800D4A08): hold, approach the screen, stick to it,
    /// fall down it, vanish.
    pub(super) fn screen_ko_animation(
        &mut self,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) -> Result<()> {
        self.step_animation(assets);
        let parameters = assets.life.screen_ko;
        let MotionData::Life(LifeState::ScreenKo(ko)) = &mut self.core.state_data else {
            panic!("screen KO scratch");
        };
        if ko.phase == ScreenKoPhase::Approach {
            ko.progress += ko.step;
        }
        if ko.remaining != 0 {
            ko.remaining -= 1;
        }
        if ko.remaining != 0 {
            return Ok(());
        }
        match ko.phase {
            ScreenKoPhase::Hold => {
                ko.step = 1.0 / parameters.approach_frames as f32;
                ko.progress = ko.step;
                ko.remaining = parameters.approach_frames;
                ko.phase = ScreenKoPhase::Approach;
            }
            ScreenKoPhase::Approach => {
                self.hit_the_screen(assets, rng)?;
                let MotionData::Life(LifeState::ScreenKo(ko)) = &mut self.core.state_data else {
                    unreachable!()
                };
                ko.remaining = parameters.impact_hold;
                ko.phase = ScreenKoPhase::Impact;
            }
            ScreenKoPhase::Impact => {
                self.core.physics.self_velocity.y = parameters.fall_speed_y;
                self.core.physics.self_velocity.z = parameters.fall_speed_z;
                ko.remaining = parameters.fall_frames;
                ko.phase = ScreenKoPhase::Fall;
            }
            ScreenKoPhase::Fall => {
                self.core.clear_velocities();
                // x221F_b1 and fp->invisible: the fighter vanishes.
                self.core.effect_state.invisible = true;
                self.core.lose_stock();
                // ftCo_800D34E0 above; ft_80088C5C, ft_PlaySFX(0x61), ft_8008805C(0x61).
                self.core.play_death_sounds(assets, 0x61);
                // ftCommon_8007EBAC(fp, 0xD, 0) is controller rumble.
                self.core.quake_request = Some(melee_cm::QuakeKind::Large);
                let MotionData::Life(LifeState::ScreenKo(ko)) = &mut self.core.state_data else {
                    unreachable!()
                };
                ko.remaining = parameters.vanish_delay;
                ko.phase = ScreenKoPhase::Vanished;
            }
            ScreenKoPhase::Vanished => {
                // ftCo_800BFD9C: Sleep, then the GM respawn.
                if self.core.player.stocks == 0 {
                    unimplemented!("gm_80167320: final stock / elimination");
                }
                self.core.state_data = MotionData::Life(LifeState::AwaitingRespawn);
            }
        }
        Ok(())
    }
    /// ftCo_800D481C (800D481C): DeadUpFallHitCamera, the fighter hits the
    /// screen: rumble, a large quake and a heavy cry (ft_800889F4, one
    /// HSD_Randi).
    fn hit_the_screen(
        &mut self,
        assets: &FighterAssets,
        rng: &mut gekko_math::HsdRng,
    ) -> Result<()> {
        let ko = match &self.core.state_data {
            MotionData::Life(LifeState::ScreenKo(ko)) => ko.clone(),
            _ => panic!("screen KO scratch"),
        };
        // x34_scale.z != 1 would select DeadUpFallHitCameraFlat.
        self.change_motion_state(S::DeadUpFallHitCamera.into(), assets)?;
        self.core.state_data = MotionData::Life(LifeState::ScreenKo(ko));
        self.face_the_screen();
        // ftCommon_8007EBAC: rumble on this fighter (5) and every other awake
        // fighter (6); no gameplay state.
        self.core.quake_request = Some(melee_cm::QuakeKind::Large);
        let voices = &assets.heavy_voices;
        if !voices.is_empty() {
            let id = voices[rng.randi(voices.len() as i32) as usize];
            self.core.push_voice(id);
        }
        // ftCo_800D4E50 is the coin-mode payout only; accessory4 becomes
        // fn_800D4DD4 (screen_ko_accessory).
        Ok(())
    }
    /// ftCo_DeadUpFall_Phys (800D4CE8): the approach interpolates the
    /// camera-space position; the fall accumulates its velocity there.
    /// Fighter_procUpdate's tail then integrates the world position as usual.
    /// The hitlag-slowdown gate (x2222_b6, ftAnim_80070FD0) is not reachable
    /// in a Vs match.
    pub(crate) fn screen_ko_physics(&mut self, assets: &FighterAssets, wind: Vec3) {
        let parameters = assets.life.screen_ko;
        let MotionData::Life(LifeState::ScreenKo(ko)) = &mut self.core.state_data else {
            panic!("screen KO scratch");
        };
        match ko.phase {
            ScreenKoPhase::Approach => {
                // lbVector_Lerp: separate subtract, multiply and add.
                let (a, b, t) = (parameters.start, parameters.end, ko.progress);
                ko.position = Vec3::new(
                    (b.x - a.x) * t + a.x,
                    (b.y - a.y) * t + a.y,
                    (b.z - a.z) * t + a.z,
                );
            }
            ScreenKoPhase::Fall => {
                let velocity = &mut self.core.physics.self_velocity;
                velocity.y = crate::physics::airborne::gravity(
                    velocity.y,
                    parameters.gravity,
                    parameters.terminal_velocity,
                );
                let v = *velocity;
                ko.displacement = Vec3::new(
                    ko.displacement.x + v.x,
                    ko.displacement.y + v.y,
                    ko.displacement.z + v.z,
                );
                ko.position = Vec3::new(
                    ko.position.x + ko.displacement.x,
                    ko.position.y + ko.displacement.y,
                    ko.position.z + ko.displacement.z,
                );
                ko.displacement = Vec3::ZERO;
            }
            _ => {}
        }
        self.core.free_flight_physics(assets, wind);
    }
    /// fn_800D4DD4 (800D4DD4): the accessory4 that ftCo_800D481C installs once
    /// the fighter hits the screen. A fall below the camera bounds stops
    /// moving. Returns whether it owns accessory4 this tick.
    pub fn screen_ko_accessory(&mut self, camera_bottom: f32) -> bool {
        let MotionData::Life(LifeState::ScreenKo(ko)) = &self.core.state_data else {
            return false;
        };
        if self.core.motion_state.id == S::DeadUpFall {
            // Fighter_ChangeMotionState cleared accessory4 on entry.
            return true;
        }
        if ko.phase == ScreenKoPhase::Fall && self.core.physics.position.y < camera_bottom {
            self.core.clear_velocities();
        }
        true
    }
    /// ftDrawCommon_80080E18_inline2: place the fighter from its camera-space
    /// position through the inverse viewing matrix of the second camera.
    /// Returns whether the fighter is in the screen KO.
    pub fn place_screen_ko(&mut self, inverse_view: &hsd_types::Mtx) -> bool {
        let MotionData::Life(LifeState::ScreenKo(ko)) = &self.core.state_data else {
            return false;
        };
        let mut position = Vec3::ZERO;
        hsd_anim::mtx::mtx_mult_vec(inverse_view, &ko.position, &mut position);
        self.core.physics.position = position;
        let root = self.core.animation.root;
        self.core.skeleton.set_translate(root, &position);
        true
    }
    /// ftCo_800D331C (800D331C): detach everything the fighter owns before a death
    /// entry, starting with the character's death callbacks (Fox and Falco put
    /// the Blaster away). A held item is destroyed (Item_8026A8EC, whose
    /// DestroyItemInline releases the hand); x197C/x1980, metal and the
    /// x2226_b4 hat are not part of the port yet.
    fn release_for_death(&mut self, assets: &FighterAssets) {
        if let Some(death) = self.character.table().death {
            death(self);
        }
        // ftCo_800DD100 -> ftCo_800DC920: separate a grab pair. The scene
        // releases the partner (ftCommon_8007D92C) right after this proc.
        if let Some(link) = self.core.combat.grab.take() {
            assert!(
                self.core.combat.thrown_pose.is_none(),
                "ftCo_800DC920: a thrown fighter's constraint release on death"
            );
            self.core.released_link = Some(link);
        }
        self.core.clear_velocities();
        if let Some(held) = self.core.held_item {
            self.core
                .item_requests
                .push(melee_it::ItemRequest::Destroy { item: held.item });
            self.core.release_held_item(held.item, assets);
        }
        // ftCommon_8007DB24: x2219_b0 = 0, then efLib_DestroyAll.
        self.core.effect_state.destroy_on_state_change = false;
        self.core.effects.push(EffectRequest::DestroyOwned);
        // x6C / x70 keep the fatal motion id for the stale-move stats.
    }
    /// ftCo_800DD100's other half: the partner of a fighter that died while
    /// linked loses the link (ftCo_800DC920's unconstrained path) and
    /// ftCommon_8007D92C (8007D92C) settles it: Fall in the air, else Wait.
    pub fn release_from_dead_partner(&mut self, assets: &FighterAssets) -> Result<()> {
        assert!(
            self.core.combat.thrown_pose.is_none(),
            "ftCo_800DC920: a thrown fighter's constraint release"
        );
        self.core.combat.grab = None;
        let state = if self.core.physics.ground_or_air == GroundOrAir::Air {
            S::Fall
        } else {
            S::Wait
        };
        self.change_motion_state(state.into(), assets)
    }
    /// ftCo_800D4FF4 (800D4FF4), after Fighter_UnkProcessDeath reset.
    pub fn enter_revival(&mut self, assets: &FighterAssets, target: Vec3) -> Result<()> {
        self.leave_ground();
        self.change_revival_motion(assets)?;
        self.core.state_data = MotionData::Life(LifeState::Revival {
            remaining: assets.life.revival_duration,
            target,
        });
        self.core.status.input_frozen = false;
        self.core.status.ignore_fighter_nudge = true;
        self.core.commands.hurt_status = melee_types::combat::HurtStatus::Intangible;
        let platform = &mut self.core.revival_platform;
        platform.tree.req_anim_all(platform.root, 0.0);
        // ftCoD4FF4 (800D51C0): separate model-scale product, no FMA.
        let scale = self.core.player.scale
            * self.core.attributes.size.model_scaling
            * self.core.attributes.size.respawn_platform_scale;
        platform
            .tree
            .set_scale(platform.root, &Vec3::new(scale, scale, scale));
        platform
            .tree
            .set_translate(platform.root, &self.core.physics.position);
        self.core.revival_platform_active = true;
        Ok(())
    }
    /// ftCo_RebirthWait_IASA (ft_0D4D.c): leave the revival platform on input.
    ///
    /// Priority inputs first (special, item pickup with LR+A, tether catch, air
    /// dodge, item throw, aerial jump); otherwise a held shield bit, D-pad up, the
    /// squat/turn/walk stick tests or a lost partner drop into Fall. Either way
    /// the fighter receives the revival invincibility (`ftColl_8007B7A4` with
    /// PlCo +5D8) and the intangibility colour flash (`ftCo_800BFFD0(fp, 9, 0)`).
    pub(super) fn revival_input(&mut self, assets: &FighterAssets) -> Result<()> {
        use crate::input::Buttons;
        let common = &assets.input;
        let facing = self.core.physics.facing;
        let pressed = self.core.input.pressed;
        let held = self.core.input.current.held;
        let stick = self.core.input.current.stick;
        // ftCo_800D7100 after the special check: LR + A catches an item.
        if !pressed.intersects(Buttons::B) && self.try_aerial_item_catch(assets) {
            return Ok(());
        }
        // No partner (x221F_b4 is the Ice Climbers' Nana flag): var_r30 stays 0.
        let priority = if pressed.intersects(Buttons::B) {
            // ftCo_SpecialAir_CheckInput
            self.enter_buffered_special(assets, true);
            true
        } else if pressed.intersects(Buttons::DIGITAL_SHOULDERS) {
            // ftCo_800C3B10 is tether characters only; ftCo_80099A58 -> EscapeAir.
            self.enter_air_dodge(assets)?;
            true
        } else if super::attack::aerial::requested(&self.core.input, common) {
            // ftCo_RebirthWait_IASA (800D575C): AttackAir before aerial jump.
            (self.character.table().enter_aerial)(self, assets)?;
            true
        } else if self.aerial_jump_requested(assets) {
            // ftCo_800CB870 -> ftCo_JumpAerial_CheckInput.
            self.enter_aerial_jump(assets)?;
            true
        } else {
            false
        };
        if !priority {
            let thresholds = &common.thresholds;
            let fall = held.intersects(Buttons::SHIELD) // ftCo_80091A2C
                || pressed.intersects(Buttons::UP) // ftCo_800DE9B8
                || stick.y < -thresholds.squat_stick_threshold // fn_800D5F84
                || stick.x * facing <= thresholds.turn_stick_threshold // ftCo_800C97A8
                || stick.x * facing >= thresholds.walk_stick_threshold; // ftWalkCommon_800DFC70
            if !fall {
                return Ok(());
            }
            self.change_motion_state(S::Fall.into(), assets)?; // ftCo_Fall_Enter
        }
        // ftColl_8007B7A4(gobj, p_ftCommonData->x5D8): x1994 = max(x1994, dur); the x198C
        // flash-type selector is renderer state and is not modelled.
        self.core.status.revival_invincibility = self
            .core
            .status
            .revival_invincibility
            .max(assets.life.invincibility_duration);
        self.core
            .commands
            .color_animations
            .push(melee_cmd::ColorAnimationRequest { id: 9, duration: 0 });
        // pl_80040374: stamps the stale-move table's platform-exit frame (xD60);
        // it feeds no compared key or RNG draw.
        Ok(())
    }
    /// Rebirth_Anim (800D52F8), RebirthWait_Anim (800D56EC).
    pub(super) fn revival_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        match &mut self.core.state_data {
            MotionData::Life(LifeState::Revival { remaining, target }) => {
                *remaining -= 1;
                if *remaining == 0 {
                    let target = *target;
                    melee_mp::set_position(
                        &mut self.core.collision.data,
                        &self.core.physics.position,
                    );
                    self.core.physics.self_velocity.y = 0.0;
                    self.change_motion_state(S::RebirthWait.into(), assets)?;
                    self.core.state_data = MotionData::Life(LifeState::PlatformWait {
                        remaining: assets.life.platform_duration,
                        target,
                    });
                    self.core.status.ignore_fighter_nudge = true;
                    self.core.commands.hurt_status = melee_types::combat::HurtStatus::Intangible;
                }
            }
            MotionData::Life(LifeState::PlatformWait { remaining, .. }) => {
                *remaining -= 1;
                if *remaining == 0 {
                    // 800D5738 precedes Fall entry at800D5740. Unlike input
                    // exit, timeout does not call pl_80040374.
                    self.core.status.revival_invincibility = self
                        .core
                        .status
                        .revival_invincibility
                        .max(assets.life.invincibility_duration);
                    self.core
                        .commands
                        .color_animations
                        .push(melee_cmd::ColorAnimationRequest { id: 9, duration: 0 });
                    self.change_motion_state(S::Fall.into(), assets)?;
                }
            }
            _ => panic!("revival scratch"),
        }
        Ok(())
    }
}
impl FighterCore {
    /// Fighter_UnkInitReset (80067C98): retain loaded resources and reset live state.
    fn reset_life(&mut self, assets: &FighterAssets, map: &melee_mp::CollMap) {
        let player = &self.player;
        // Same audited coordinate calculation as initial preparation (80067CE8).
        let offset = 0.0 * player.scale;
        let position = Vec3::new(
            gekko_math::fma::fmadds(player.facing, offset, player.position.x),
            player.position.y,
            player.position.z,
        );
        self.animation.reset_for_spawn(&mut self.skeleton);
        self.skeleton.set_translate(self.animation.root, &position);
        self.physics = crate::physics::FighterPhysics::standing(position, player.facing);
        self.physics.percent = player.damage;
        self.input = crate::input::FighterInput::default();
        let mut collision = std::mem::take(&mut self.collision.data);
        crate::collision::ecb::reinitialize(
            map,
            &mut collision,
            position,
            &self.bones.ecb,
            player.scale,
            self.attributes.size.weight,
        );
        self.collision = super::EnvironmentCollision::new(collision);
        self.state_data = MotionData::None;
        // Fighter_UnkInitReset retains the costume geometry computed once by
        // Fighter_UnkUpdateVecFromBones_8006876C, including across stock losses.
        self.combat = super::damage::CombatState {
            capture_geometry: self.combat.capture_geometry,
            ..Default::default()
        };
        self.shield = super::shield::ShieldState::default();
        // x2220_b0, the rotating effect-bone index, is cleared only by
        // Fighter_UnkInitLoad (fighter.c:765), so it survives stock losses.
        self.effect_state = super::effects::FighterEffects {
            rotating_bone_index: self.effect_state.rotating_bone_index,
            ..Default::default()
        };
        self.effects = melee_ef::request::EffectQueue::default();
        // fighter.c:460-461: x209C and x2224_b1.
        self.catch_window = 0;
        self.item_catch_locked = false;
        // Fighter_UnkInitReset leaves 2227.b1 intact. Only a subsequent
        // grounded motion entry clears the ledge-timeout provenance.
        let ledge_timed_out = self.status.ledge_timed_out;
        // UnkInitReset clears1969, sets210C to254, and retains wall side2110.
        let wall_jump = super::wall_jump::WallJump {
            used: 0,
            contact_age: 254,
            ..self.status.wall_jump
        };
        self.status = super::Status::reset(assets.shield_health);
        self.status.ledge_timed_out = ledge_timed_out;
        self.status.wall_jump = wall_jump;
        // Fighter_UnkInitReset (80067C98) does not write cmd_vars (+2200..220C).
        // Later motion commands own initialization; retain them across stocks.
        let command_variables = self.commands.variables;
        self.commands = super::commands::CommandState::default();
        self.commands.variables = command_variables;
        self.ground_pose = crate::collision::pose::GroundPoseFlags::default();
        self.dynamics_use_floor_plane = false;
        self.player_position = position;
        self.player_facing = player.facing;
        self.joystick_count = 0;
        self.previous_collision_bounds = Vec3::ZERO;
        self.offscreen.magnified_ticks = 0;
        self.hurtboxes.clone_from_slice(&assets.hurtboxes);
        self.dynamic_colliders
            .clone_from_slice(&assets.dynamic_colliders);
        self.thrown_hitbox.clone_from(&assets.thrown_hitbox);
        self.revival_platform_active = false;
    }
    /// ftCommon_8007E2FC (8007E2FC): a death clears every velocity owner.
    pub(super) fn clear_velocities(&mut self) {
        self.physics.self_velocity = Vec3::ZERO;
        self.physics.animation_velocity = Vec3::ZERO;
        self.physics.knockback_velocity = Vec3::ZERO;
        self.physics.shield_knockback_velocity = Vec3::ZERO;
        self.physics.ground_velocity = 0.0;
        self.physics.ground_knockback_velocity = 0.0;
        self.physics.ground_shield_knockback_velocity = 0.0;
    }
    /// ftCo_800D34E0 (800D34E0): the stock-loss bookkeeping. Falls, KO/suicide
    /// counts, the match frame count and Player_SetHPByIndex(0) feed no compared
    /// key; the stock count drives the stock display.
    fn lose_stock(&mut self) {
        self.player.stocks = self.player.stocks.saturating_sub(1);
    }
    /// ft_80088C5C stops the fighter's channels, ftCo_800D38B8 plays x4C_sfx +4 and +8
    /// on the voice channel, then ft_PlaySFX(exit, 127, 64).
    fn play_death_sounds(&mut self, assets: &FighterAssets, exit_sound: u32) {
        use super::commands::{FootstepSound, SoundChannel};
        for id in assets.life.death_sounds.cries {
            self.push_voice(id);
        }
        self.commands.footstep_sounds.push(FootstepSound {
            channel: SoundChannel::Ordinary,
            id: exit_sound,
            volume: 127,
            pan: 64,
        });
    }
    /// ft_800881D8(fp, id, 127, 64): one request on the fighter's voice channel.
    fn push_voice(&mut self, id: u32) {
        use super::commands::{FootstepSound, SoundChannel};
        self.commands.footstep_sounds.push(FootstepSound {
            channel: SoundChannel::FighterVoice,
            id,
            volume: 127,
            pan: 64,
        });
    }
    /// ftCo_DeadDown_Anim (800D3E00) and the DeadLeft / DeadRight twins: GM respawn is
    /// performed at this callback boundary.
    pub(super) fn death_animation(&mut self) {
        let MotionData::Life(LifeState::Dead { remaining }) = &mut self.state_data else {
            panic!("death scratch");
        };
        *remaining -= 1;
        if *remaining == 0 {
            if self.player.stocks == 0 {
                unimplemented!("gm_80167320: final stock / elimination");
            }
            self.state_data = MotionData::Life(LifeState::AwaitingRespawn);
        }
    }
    /// fn_800D54A4 and Fighter_8006C80C: place then animate the accessory.
    pub fn update_revival_platform(&mut self) {
        if self.revival_platform_active {
            let platform = &mut self.revival_platform;
            platform
                .tree
                .set_translate(platform.root, &self.physics.position);
            platform.tree.anim_all::<super::RetailTrig>(platform.root);
            assert!(
                platform.tree.events.is_empty(),
                "revival accessory event routing"
            );
        }
    }
    /// Rebirth_Phys (800D535C) / RebirthWait_Phys (800D58F4).
    pub(super) fn revival_physics(&mut self, assets: &FighterAssets, wind: Vec3) {
        let (remaining, target) = match self.state_data {
            MotionData::Life(
                LifeState::Revival { remaining, target }
                | LifeState::PlatformWait { remaining, target },
            ) => (remaining, target),
            _ => panic!("revival physics scratch"),
        };
        // This scene's static marker has zero player offset. The retail FMA
        // at 800D53BC / 800D5954 is the moving-marker offset path.
        let inv = 1.0 / remaining as f32;
        self.physics.self_velocity.x = (target.x - self.physics.position.x) * inv;
        self.physics.self_velocity.y = (target.y - self.physics.position.y) * inv;
        self.free_flight_physics(assets, wind);
    }
    /// Fighter_procUpdate's tail for a state without its own physics: knockback
    /// decay, then the velocity and environment integration.
    pub(super) fn free_flight_physics(&mut self, assets: &FighterAssets, wind: Vec3) {
        self.decay_air_knockback(assets);
        crate::physics::integrate::integrate_velocity(&mut self.physics);
        crate::physics::integrate::integrate_environment(&mut self.physics, None, wind);
    }
}
