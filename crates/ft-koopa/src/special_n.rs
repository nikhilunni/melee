//! Fire Breath, ftkoopaspecialn.c (80134ACC..80135780).
//!
//! A start, a looping breath and an end, on the ground and in the air.
//! While the loop runs Bowser's mouth spawns a flame every third frame;
//! both of the breath's fuels (the flames' reach and life) drain by one a
//! frame, down to their floor, and refill outside the move
//! (`init::refill_breath`). The loop lasts at least the attribute's frames
//! and one animation cycle, then for as long as B is held.
use crate::{
    common::{self, change},
    init::Koopa,
};
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, MotionRow},
        ActionId, Fighter, MotionEntryFlags,
    },
    input::Buttons,
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::ItemKind;

/// ftKp_MS_SpecialNStart (341) .. ftKp_MS_SpecialAirNEnd (346): the rows
/// in which the breath's fuels drain instead of refilling.
pub const FIRST_ROW: u16 = 341;
pub const LAST_ROW: u16 = 346;

const START: ActionId = ActionId(341);
const LOOP: ActionId = ActionId(342);
const END: ActionId = ActionId(343);
const AIR_START: ActionId = ActionId(344);
const AIR_LOOP: ActionId = ActionId(345);
const AIR_END: ActionId = ActionId(346);

/// ftKp_MF_SpecialN_Coll: the loop's ground/air switch,
/// ftCommon_GroundAirColl_MF with Ft_MF_SkipRumble (0x0C4C5880).
const LOOP_SWITCH: MotionEntryFlags = MotionEntryFlags(0x0C4C_5880);
/// fp->parts[48]: the mouth the flames leave from.
const MOUTH_PART: usize = 48;
/// A flame leaves on every third frame of the loop.
const FLAME_INTERVAL: i32 = 3;
/// Flames per hit group and attack instance (ftKp_SpecialLw_80134ACC).
const FLAMES_PER_CYCLE: i32 = 12;
/// Flames between the breath's sounds.
const SOUND_INTERVAL: i32 = 3;
/// ft_PlaySFX ids for a weak, middling and full breath (FTKIND_KOOPA).
const WEAK_SOUND: u32 = 0x24A2B;
const MIDDLE_SOUND: u32 = 0x24A28;
const FULL_SOUND: u32 = 0x24A25;
/// The breath's share of full below which each weaker sound plays
/// (@230, @231).
const WEAK_SHARE: f32 = 0.3333;
const MIDDLE_SHARE: f32 = 0.6666;

/// ftKp_Init_803CF2A0: which of the four flame generators the next flame
/// carries, drawn by HSD_Randi(32) from one of three rows. The first row
/// picks between the other two.
const FLAME_EFFECTS: [[i32; 32]; 3] = [
    [
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
        1, 1,
    ],
    [
        0, 0, 0, 0, 0, 0, 0, 0, 3, 3, 3, 3, 3, 3, 3, 3, 0, 0, 0, 0, 0, 0, 0, 0, 3, 3, 3, 3, 3, 3,
        3, 3,
    ],
    [
        1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 2,
        2, 2,
    ],
];

/// mv.kp.specials for the Fire Breath rows (Fighter +2340..+235C).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Breath {
    /// +0x0: frames since the last flame, 0..2; a flame leaves at 0.
    pub flame_phase: i32,
    /// +0x8: the last flame's generator, which selects the next one's row.
    pub last_effect: i32,
    /// +0xC: frames breathed, up to the attribute minimum.
    pub frames: i32,
    /// +0x10: animation cycles the loop still owes once B is released.
    pub cycles_owed: i32,
    /// +0x14: flames since the hit group and attack instance last changed.
    pub cycle_flames: i32,
    /// +0x18: frames since the last camera quake.
    pub quake_phase: i32,
    /// This frame's IASA let a flame out: the reach and life fuels
    /// (u.kp.x222C and x2230) as they stood before the frame's drain. The
    /// flame leaves once the IASA returns, with the RNG.
    pub pending_flame: Option<(f32, f32)>,
}

/// ftKp_Init_MotionStateTable[0..6]: submotions 295..300. The grounded
/// rows' physics is ft_80084F3C, the aerial rows' ft_80084DB0.
pub const fn rows() -> [MotionRow; 6] {
    [
        common::row(
            START,
            295,
            start_anim::<false>,
            common::no_input,
            common::ground_friction,
            start_ground_collision,
        ),
        common::row(
            LOOP,
            296,
            loop_anim,
            loop_input::<false>,
            common::ground_friction,
            loop_ground_collision,
        ),
        common::row(
            END,
            297,
            end_ground_anim,
            common::no_input,
            common::ground_friction,
            end_ground_collision,
        ),
        common::row(
            AIR_START,
            298,
            start_anim::<true>,
            common::no_input,
            callbacks::physics::fall,
            start_air_collision,
        ),
        common::row(
            AIR_LOOP,
            299,
            loop_anim,
            loop_input::<true>,
            callbacks::physics::fall,
            loop_air_collision,
        ),
        common::row(
            AIR_END,
            300,
            end_air_anim,
            common::no_input,
            callbacks::physics::fall,
            end_air_collision,
        ),
    ]
}

fn breath(f: &mut Fighter) -> &mut Breath {
    &mut f.character.get_mut::<Koopa>().breath
}

/// ftKp_SpecialN_Enter (80134E58) / ftKp_SpecialAirN_Enter (80134ED0): the
/// start row, the scratch cleared with one animation cycle owed, and a
/// hit group id for the flames (Item_8026AE60).
pub fn enter(f: &mut Fighter, air: bool, a: &FighterAssets) {
    change(
        f,
        if air { AIR_START } else { START },
        MotionEntryFlags(0),
        0.0,
        a,
    )
    .expect("Fire Breath assets");
    *breath(f) = Breath {
        cycles_owed: 1,
        ..Default::default()
    };
    f.core.item_requests.push(ItemRequest::NewHitGroup);
}

/// ftKp_SpecialNStart_Anim (80134F48) / ftKp_SpecialAirNStart_Anim
/// (8013507C): the loop follows the start.
fn start_anim<const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let next = if AIR { AIR_LOOP } else { LOOP };
        change(f, next, MotionEntryFlags(0), 0.0, p.assets)?;
    }
    Ok(None)
}

/// ftKp_SpecialN_Anim (80134F9C) / ftKp_SpecialAirN_Anim (801350D0): each
/// time the loop's animation comes round to frame 0 one owed cycle is
/// paid; a small camera quake at the fighter every attribute x20 frames.
fn loop_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let frame = f.animation.frame;
    let interval = f
        .character
        .get::<Koopa>()
        .attributes
        .fire_breath
        .quake_interval;
    let scratch = breath(f);
    if frame == 0.0 {
        scratch.cycles_owed -= 1;
        if scratch.cycles_owed <= 0 {
            scratch.cycles_owed = 0;
        }
    }
    let quake = scratch.quake_phase == 0;
    scratch.quake_phase += 1;
    scratch.quake_phase %= interval;
    if quake {
        f.core.quake_request = Some(melee_cm::QuakeKind::Small);
    }
    Ok(None)
}

/// ftKp_SpecialNEnd_Anim (80135040): Wait at the animation's end
/// (ft_8008A2BC).
fn end_ground_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftKp_SpecialAirNEnd_Anim (80135174): Fall at the animation's end.
fn end_air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftKp_SpecialN_IASA (801351B4) / ftKp_SpecialAirN_IASA (801352F8): past
/// the minimum frames, with B released and no cycle owed, the end row;
/// otherwise a flame on every third frame. Then the frame's bookkeeping:
/// the flame phase, both fuels down by one to their floors (fsubs), the
/// frame count up to the minimum.
fn loop_input<const AIR: bool>(f: &mut Fighter, p: InputPhase<'_>) {
    let (minimum, reach_min, life_min) = {
        let a = &f.character.get::<Koopa>().attributes.fire_breath;
        (a.minimum_ticks, a.reach_min, a.life_min)
    };
    let held = f.input.current.held.intersects(Buttons::B);
    let scratch = *breath(f);
    if scratch.frames >= minimum && !held && scratch.cycles_owed == 0 {
        let end = if AIR { AIR_END } else { END };
        change(f, end, MotionEntryFlags(0), 0.0, p.assets).expect("Fire Breath end assets");
    } else if scratch.flame_phase == 0 {
        let koopa = f.character.get::<Koopa>();
        let fuels = (koopa.breath_reach, koopa.breath_life);
        breath(f).pending_flame = Some(fuels);
    }
    let koopa = f.character.get_mut::<Koopa>();
    koopa.breath.flame_phase += 1;
    if koopa.breath.flame_phase >= FLAME_INTERVAL {
        koopa.breath.flame_phase = 0;
    }
    koopa.breath_reach -= 1.0;
    if koopa.breath_reach < reach_min {
        koopa.breath_reach = reach_min;
    }
    koopa.breath_life -= 1.0;
    if koopa.breath_life < life_min {
        koopa.breath_life = life_min;
    }
    koopa.breath.frames += 1;
    if koopa.breath.frames > minimum {
        koopa.breath.frames = minimum;
    }
}

/// ftKp_SpecialLw_80134ACC (80134ACC), once the IASA that let the flame
/// out has returned: the flame's generator from the table (one or two
/// HSD_Randi(32) draws), the flame from the mouth, and every twelfth
/// flame a new hit group and attack instance; every third, the breath's
/// sound for the life fuel left.
pub(crate) fn release_flame(f: &mut Fighter, rng: &mut gekko_math::HsdRng) {
    let Some((reach, life)) = breath(f).pending_flame.take() else {
        return;
    };
    let (offset_x, offset_y, life_min, life_max) = {
        let a = &f.character.get::<Koopa>().attributes.fire_breath;
        (a.spawn_offset_x, a.spawn_offset_y, a.life_min, a.life_max)
    };
    // lb_8000B1CC(fp->parts[48].joint, NULL, &v).
    let c = &mut f.core;
    let mut mouth = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        MOUTH_PART,
        Vec3::ZERO,
    );
    if f.player.scale != 1.0 {
        unimplemented!("ftKp_SpecialLw_80134ACC: a scaled Bowser's flame offset (x34_scale.y)");
    }
    let scale = f.player.scale;
    let facing = f.physics.facing;
    // 80134B10: fmuls; 80134B18 / 80134B2C: fmadds.
    mouth.x = gekko_math::fma::fmadds(scale, offset_x * facing, mouth.x);
    mouth.y = gekko_math::fma::fmadds(offset_y, scale, mouth.y);

    // ftKp_SpecialLw_80134ACC_inline: after generator 1 or 2 the next comes
    // from the second row; otherwise the first row picks the row.
    let effect = match breath(f).last_effect {
        1 | 2 => FLAME_EFFECTS[1][rng.randi(32) as usize],
        _ => {
            if FLAME_EFFECTS[0][rng.randi(32) as usize] == 0 {
                FLAME_EFFECTS[1][rng.randi(32) as usize]
            } else {
                FLAME_EFFECTS[2][rng.randi(32) as usize]
            }
        }
    };
    breath(f).last_effect = effect;

    // itKoopaFlame_Spawn: prev_pos is the mouth on the plane; the spawn
    // sweep starts at the fighter's ECB centre (it_8026BB68 ->
    // ftLib_80086990).
    mouth.z = 0.0;
    let mut spawn = SpawnItem::ray(ItemKind::KoopaFlame, f.player.id, mouth, facing);
    spawn.previous_position = mouth;
    // Retail 800869AC..BC: fadds, fmuls, fadds.
    let ecb = &f.collision.data.ecb;
    let midpoint = 0.5 * (ecb.top.y + ecb.bottom.y);
    spawn.position = Vec3::new(
        f.physics.position.x + 0.0,
        f.physics.position.y + midpoint,
        f.physics.position.z + 0.0,
    );
    // 80134BCC / 80134BD4: fctiwz of both fuels, itKoopaFlame_Spawn's s32
    // arguments.
    spawn.spawn_argument = it_koopaflame::spawn_argument(
        effect,
        gekko_math::msl::fctiwz(reach),
        gekko_math::msl::fctiwz(life),
    );
    // The flame takes the attack instance current at its spawn, before a
    // cycle's first flame starts the next one below.
    spawn.stale_source = f.combat.stale.attack();
    f.core.item_requests.push(ItemRequest::SpawnInGroup(spawn));

    let flames = breath(f).cycle_flames;
    if flames == 0 {
        // Item_8026AE60, ft_80089824's statistics, then ft_800892A0.
        f.core.item_requests.push(ItemRequest::NewHitGroup);
        f.combat.stale.new_instance();
    }
    if flames % SOUND_INTERVAL == 0 {
        // 80134C44..80134C50: fsubs, fsubs, fdivs.
        let share = (life - life_min) / (life_max - life_min);
        let sound = if share < WEAK_SHARE {
            WEAK_SOUND
        } else if share < MIDDLE_SHARE {
            MIDDLE_SOUND
        } else {
            FULL_SOUND
        };
        play_sound(f, sound);
    }
    let scratch = breath(f);
    scratch.cycle_flames += 1;
    scratch.cycle_flames %= FLAMES_PER_CYCLE;
}

/// ft_PlaySFX(fp, id, 127, 64) for an id outside the footstep range (no
/// pitch draw).
fn play_sound(f: &mut Fighter, id: u32) {
    use melee_ft::fighter::commands::{FootstepSound, SoundChannel};
    f.commands.footstep_sounds.push(FootstepSound {
        channel: SoundChannel::Ordinary,
        id,
        volume: 127,
        pan: 64,
    });
}

/// ftCommon_GroundToAirStateChange into `next` when the floor is lost
/// (ft_80082708).
fn leave_floor(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
    next: ActionId,
    flags: MotionEntryFlags,
) -> Result<()> {
    if common::stays_grounded(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Fire Breath collision assets");
    common::ground_to_air(f, next, flags, assets)
}

/// ftCommon_AirToGroundStateChange into `next` on landing (ft_80081D0C).
fn reach_floor(
    f: &mut Fighter,
    mut p: CollisionPhase<'_>,
    next: ActionId,
    flags: MotionEntryFlags,
) -> Result<()> {
    if !common::lands(f, &mut p) {
        return Ok(());
    }
    let assets = p.assets.expect("Fire Breath collision assets");
    common::air_to_ground(f, next, flags, assets)
}

/// ftKp_SpecialNStart_Coll (801354F8).
fn start_ground_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    leave_floor(f, p, AIR_START, common::GROUND_AIR)
}
/// ftKp_SpecialN_Coll (80135564).
fn loop_ground_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    leave_floor(f, p, AIR_LOOP, LOOP_SWITCH)
}
/// ftKp_SpecialNEnd_Coll (801355D0).
fn end_ground_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    leave_floor(f, p, AIR_END, common::GROUND_AIR)
}
/// ftKp_SpecialAirNStart_Coll (8013563C).
fn start_air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    reach_floor(f, p, START, common::GROUND_AIR)
}
/// ftKp_SpecialAirN_Coll (801356A8).
fn loop_air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    reach_floor(f, p, LOOP, LOOP_SWITCH)
}
/// ftKp_SpecialAirNEnd_Coll (80135714).
fn end_air_collision(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    reach_floor(f, p, END, common::GROUND_AIR)
}
