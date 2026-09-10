//! Entry / EntryStart / EntryEnd, ft/ft_0C31.c:25-331.
use super::FighterCore;
use super::{
    assets::{FighterAssets, Result},
    effects::EffectRequest,
    CharacterCallbacks, Fighter, MotionData,
};
use hsd_types::Vec3;
use melee_types::{mp::FtCollisionBox, CommonMotionState, GroundOrAir};

#[derive(Clone, Copy, Debug)]
pub struct EntryParameters {
    /// PlCo +6BC / +6C0; these are local entry timers, not the GO timer.
    pub grow_frames: i32,
    pub shrink_frames: i32,
    /// PlCo +6C4.
    pub initial_scale_y: f32,
}
#[derive(Clone, Debug)]
pub struct EntryState {
    /// mv.co.entry.timer, +2340.
    pub timer: i32,
    /// +2344: initial spawn height, preserved across entry states.
    pub origin_y: f32,
    /// +2348, +2354: original and animated model scale.
    pub original_scale: Vec3,
    pub current_scale: Vec3,
    /// +2360, +2364, +2368: trophy height, scale and current lift.
    pub trophy_height: f32,
    pub trophy_scale: f32,
    pub lift: f32,
    /// +236C: ECB captured before the model starts growing.
    pub collision_box: FtCollisionBox,
}
impl EntryState {
    /// ftCo_Entry_Anim (0x800C6370): check first, then decrement even after
    /// the next state replaces the timer. Start/End decrement before checking.
    pub fn advance(
        &mut self,
        state: CommonMotionState,
        parameters: EntryParameters,
    ) -> Option<CommonMotionState> {
        if state == CommonMotionState::Entry {
            let transition = if self.timer == 0 {
                self.timer = parameters.grow_frames;
                Some(CommonMotionState::EntryStart)
            } else {
                None
            };
            self.timer = self.timer.wrapping_sub(1);
            transition
        } else {
            self.timer = self.timer.wrapping_sub(1);
            if self.timer != 0 {
                return None;
            }
            if state == CommonMotionState::EntryStart {
                self.timer = parameters.shrink_frames;
                Some(CommonMotionState::EntryEnd)
            } else {
                Some(CommonMotionState::Fall)
            }
        }
    }
}
impl<C: CharacterCallbacks> Fighter<C> {
    /// ftCo_800C61B0 (0x800C61B0), scene supplies Player_GetUnk4C delay.
    pub fn enter_match(&mut self, delay: i32, assets: &FighterAssets) -> Result<()> {
        let scale = self.core.skeleton.scale(self.core.animation.root);
        let ecb = self.core.collision.data.ecb;
        let current_scale = Vec3::new(scale.x, assets.entry.initial_scale_y, scale.z);
        self.core.state_data = MotionData::Entry(EntryState {
            timer: delay,
            origin_y: self.core.physics.position.y,
            original_scale: scale,
            current_scale,
            trophy_height: 0.0,
            trophy_scale: 0.0,
            lift: 0.0,
            collision_box: FtCollisionBox {
                top: ecb.top.y,
                bottom: ecb.bottom.y,
                left: ecb.left,
                right: ecb.right,
            },
        });
        self.core
            .skeleton
            .set_scale(self.core.animation.root, &current_scale);
        self.change_motion_state(CommonMotionState::Entry, assets)
    }
    pub(super) fn entry_animation(&mut self, assets: &FighterAssets) -> Result<()> {
        let MotionData::Entry(entry) = &mut self.core.state_data else {
            panic!("entry data missing")
        };
        let Some(mut next) = entry.advance(self.core.motion_state.id, assets.entry) else {
            return Ok(());
        };
        match next {
            CommonMotionState::EntryStart => {
                // ftCo_800C6408: separate fmuls and double fmul (no FMA).
                entry.trophy_scale =
                    self.core.player.scale * self.core.attributes.size.trophy_scale;
                entry.trophy_height = (1.497345_f64 * f64::from(entry.trophy_scale)) as f32;
                self.core.effects.push(EffectRequest::EntryWarp {
                    id: 0x43E,
                    scale: entry.original_scale,
                });
            }
            CommonMotionState::EntryEnd => {
                self.core
                    .skeleton
                    .set_scale(self.core.animation.root, &entry.original_scale);
                self.core.physics.position.y = entry.origin_y + entry.trophy_height;
            }
            CommonMotionState::Fall => {
                // ftCommon_8007D92C (0x8007D92C), ftcommon.c:596-604.
                if self.core.physics.ground_or_air == GroundOrAir::Ground {
                    next = CommonMotionState::Wait;
                }
            }
            _ => unreachable!(),
        }
        self.change_motion_state(next, assets)
    }
}
impl FighterCore {
    /// Entry_Phys is empty. Start (0x800C6740) grows the trophy/model;
    /// End (0x800C6D38) shrinks the trophy while retaining model scale.
    pub(super) fn entry_physics(&mut self, parameters: EntryParameters) {
        if self.motion_state.id == CommonMotionState::Entry {
            return;
        }
        let MotionData::Entry(entry) = &mut self.state_data else {
            panic!("entry data missing")
        };
        let fraction = if self.motion_state.id == CommonMotionState::EntryStart {
            (parameters.grow_frames - entry.timer) as f32 / parameters.grow_frames as f32
        } else {
            entry.timer as f32 / parameters.grow_frames as f32
        };
        if self.motion_state.id == CommonMotionState::EntryStart {
            // retail 0x800C67AC: fmadds f0,f31,f0,f4.
            entry.current_scale.y = gekko_math::fma::fmadds(
                fraction,
                entry.original_scale.y - parameters.initial_scale_y,
                parameters.initial_scale_y,
            );
            self.skeleton
                .set_scale(self.animation.root, &entry.current_scale);
        }
        // retail EntryStart/End Phys: separate fmuls, store, fadds.
        entry.lift = entry.trophy_height * fraction;
        self.physics.position.y = entry.origin_y + entry.lift;
        entry.collision_box.bottom = -entry.lift;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{anim::FighterAnimation, fighter::RetailTrig};
    use hsd_anim::{
        aobj::{AObj, AOBJ_FIRST_PLAY},
        jobj::{JObjTree, JointSpec},
    };

    #[test]
    fn entry_checks_before_decrement_but_start_and_end_check_after() {
        let parameters = EntryParameters {
            grow_frames: 30,
            shrink_frames: 30,
            initial_scale_y: 0.001,
        };
        let mut entry = EntryState {
            timer: 5,
            origin_y: 10.0,
            original_scale: Vec3::new(1.0, 1.0, 1.0),
            current_scale: Vec3::ZERO,
            trophy_height: 0.0,
            trophy_scale: 0.0,
            lift: 0.0,
            collision_box: FtCollisionBox::default(),
        };
        let mut state = CommonMotionState::Entry;
        let mut transitions = Vec::new();
        for tick in 1..=65 {
            if let Some(next) = entry.advance(state, parameters) {
                transitions.push((tick, next));
                state = next;
            }
        }
        assert_eq!(
            transitions,
            [
                (6, CommonMotionState::EntryStart),
                (35, CommonMotionState::EntryEnd),
                (65, CommonMotionState::Fall)
            ]
        );
    }

    #[test]
    fn completed_entry_animation_holds_without_advancing_the_state_timer() {
        let mut tree = JObjTree::new();
        let root = tree.load_joint(&JointSpec::new());
        let mut animation = FighterAnimation::new(&tree, root);
        tree.get_mut(root).aobj = Some(AObj {
            flags: AOBJ_FIRST_PLAY,
            curr_frame: 0.0,
            end_frame: 10.0,
            framerate: 1.0,
            ..AObj::default()
        });
        animation.motion_id = 238;
        for frame in 0..29 {
            animation.step::<RetailTrig>(&mut tree);
            assert_eq!(animation.frame.to_bits(), (frame.min(10) as f32).to_bits());
        }
        // EntryEnd uses SM_None: the completed pose survives but its frame
        // resets to anim_start - speed (-1); there is no animation to step.
        animation.clear_motion(&mut tree);
        for _ in 0..30 {
            animation.step::<RetailTrig>(&mut tree);
            assert_eq!(animation.frame.to_bits(), (-1.0f32).to_bits());
        }
    }
}
