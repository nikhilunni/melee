//! Stage line hits (ftCo_Bury.c's ftCo_800C0B20): a fighter touching a
//! line the stage marks (StageData.on_touch_line, Ground_801C5700) takes
//! the line's hit, logged as a stage entry (DmgLogEntry kind 3) that
//! ftColl_8007A06C resolves with the fighter's own facing and no effects.
use super::{assets::FighterAssets, FighterCore};
use melee_types::{
    combat::{LineHazard, ThrowHitbox},
    mp::collide,
};

/// ftCo_800C0B20's DmgLogEntry.kind (ftColl_80076764's second argument).
const LINE_HIT_KIND: u32 = 1;

/// PlCo +5DC (`bury_timer_unk1`): frames after a line's hit before the
/// next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Parameters {
    pub cooldown: u32,
}
impl Parameters {
    pub fn read(archive: &hsd_archive::Archive, base: u32) -> hsd_archive::desc::Result<Self> {
        Ok(Self {
            cooldown: archive.reader().u32(base + 0x5DC)?,
        })
    }
}

/// lbColl_80008D30_arg1: the line's hit record, the same layout a stage
/// device hands back.
fn stage_hit(hazard: &LineHazard) -> super::stage_hazard::StageHit {
    super::stage_hazard::StageHit {
        record: ThrowHitbox {
            damage: hazard.damage as f32,
            angle: hazard.angle as u16,
            growth: hazard.growth as u16,
            weight_knockback: hazard.weight_knockback as u16,
            base_knockback: hazard.base_knockback as u16,
            element: hazard.element,
            sound_severity: hazard.sound_severity as u8,
            sound_kind: hazard.sound_kind as u8,
        },
        count: hazard.damage as i32,
    }
}

impl FighterCore {
    /// ftCo_800C0A98's first step (Fighter_procUpdate, fighter.c:2173):
    /// bury_timer_1 counts down.
    pub(super) fn tick_hazard_cooldown(&mut self) {
        if self.status.hazard_cooldown != 0 {
            self.status.hazard_cooldown -= 1;
        }
    }

    /// ftCo_800C0B20 (0x800C0B20), from ftColl_8007BAC0 in the hit
    /// detection proc: with no cooldown running, the line under each
    /// touched surface (floor, then ceiling, then walls; the last touched
    /// decides, even when it is unmarked) names the hit `touched` returns.
    pub fn touch_line_hazards(
        &mut self,
        touched: impl Fn(i32) -> Option<LineHazard>,
        assets: &FighterAssets,
    ) {
        // ftColl_8007BAC0: only with ftColl_GetHitStatus zero (x1988, the
        // subaction's body status; x198C, the timers' — revival
        // invincibility and ledge intangibility).
        if !self.hurtable_by_stage() {
            return;
        }
        if self.status.hazard_cooldown != 0 {
            return;
        }
        let data = &self.collision.data;
        let env = data.env_flags as u32;
        let mut hazard = None;
        if env & collide::FLOOR_MASK != 0 {
            hazard = touched(data.floor.index);
        }
        if env & collide::CEILING_MASK != 0 {
            hazard = touched(data.ceiling.index);
        }
        if env & collide::RIGHT_WALL_MASK != 0 {
            hazard = touched(data.right_facing_wall.index);
        }
        if env & collide::LEFT_WALL_MASK != 0 {
            hazard = touched(data.left_facing_wall.index);
        }
        let Some(hazard) = hazard else {
            return;
        };
        if hazard.sound_kind == 0x16A {
            // ftColl_8007A06C reads the entry's hit0->kb_angle, the
            // union's DynamicsDesc +0x20: 0x16A takes the direction from
            // the capsule.
            unimplemented!("ftColl_8007A06C (ftcoll.c:2882): a stage line hit's +0x20 is 0x16A");
        }
        // ftColl_80076764(3, 1, NULL, ..): DmgLogEntry.kind 1; bury_timer_1
        // = PlCo +5DC.
        self.log_stage_hit(&stage_hit(&hazard), LINE_HIT_KIND, assets, |core| {
            core.status.hazard_cooldown = assets.hazard.cooldown;
        });
    }
}
