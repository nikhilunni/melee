//! Fire Breath, ftkoopaspecialn.c (80134ACC..80135780), and Kirby's copy
//! of it (ftkirbyspecialkoopa.c, 800FA588..800FB3C8): the same states over
//! either user's rows, generic over [`FireBreathUser`].
//!
//! A start, a looping breath and an end, on the ground and in the air.
//! While the loop runs the mouth spawns a flame every third frame; both of
//! the breath's fuels (the flames' reach and life) drain by one a frame,
//! down to their floor, and refill outside the move ([`refill`]). The loop
//! lasts at least the attribute's frames and one animation cycle, then for
//! as long as B is held.
use crate::common::{self, change};
use hsd_archive::Reader;
use hsd_types::Vec3;
use melee_ft::{
    anim::WaitChoice,
    desc::FighterDescError,
    fighter::{
        assets::{FighterAssets, Result},
        state::{callbacks, AnimationPhase, CollisionPhase, InputPhase, MotionRow},
        ActionId, Fighter, MotionEntryFlags,
    },
    input::Buttons,
};
use melee_it::{ItemRequest, SpawnItem};
use melee_types::ItemKind;

/// The loop's ground/air switch: ftCommon_GroundAirColl_MF with
/// Ft_MF_SkipRumble (0x0C4C5880; ftKp_MF_SpecialN_Coll, and the constant
/// Kirby's loop rows pass).
const LOOP_SWITCH: MotionEntryFlags = MotionEntryFlags(0x0C4C_5880);
/// A flame leaves on every third frame of the loop.
const FLAME_INTERVAL: i32 = 3;
/// Flames per hit group and attack instance.
const FLAMES_PER_CYCLE: i32 = 12;
/// Flames between the breath's sounds.
const SOUND_INTERVAL: i32 = 3;
/// ft_PlaySFX ids for a weak, middling and full breath (both users).
const WEAK_SOUND: u32 = 0x24A2B;
const MIDDLE_SOUND: u32 = 0x24A28;
const FULL_SOUND: u32 = 0x24A25;
/// The breath's share of full below which each weaker sound plays
/// (@230, @231; Kirby's @243, @244).
const WEAK_SHARE: f32 = 0.3333;
const MIDDLE_SHARE: f32 = 0.6666;

/// ftKp_Init_803CF2A0 / ftKb_Init_803CB540: which of the four flame
/// generators the next flame carries, drawn by HSD_Randi(32) from one of
/// three rows. The first row picks between the other two.
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

/// The Fire Breath's attributes: ftKoopaAttributes +0x04..+0x28 and
/// ftKb_DatAttrs +118..+13C (`specialn_kp_*`, whose header names are
/// guesses), the same ten words in the same order. The breath has two
/// fuels: the flame's reach (Bowser's u.kp.x222C, Kirby's u.kb.x84) and
/// its lifetime (u.kp.x2230, u.kb.x88).
#[derive(Clone, Debug, PartialEq)]
pub struct FireBreathAttributes {
    /// +0x00: ticks of breath before releasing B may end it.
    pub minimum_ticks: i32,
    /// +0x04: reach regained per tick outside the move.
    pub reach_recharge: f32,
    /// +0x08: lifetime regained per tick outside the move.
    pub life_recharge: f32,
    /// +0x0C / +0x10: the reach's full and spent values.
    pub reach_max: f32,
    pub reach_min: f32,
    /// +0x14 / +0x18: the lifetime's full and spent values.
    pub life_max: f32,
    pub life_min: f32,
    /// +0x1C: ticks between camera quakes.
    pub quake_interval: i32,
    /// +0x20 / +0x24: the flame's spawn offset from the mouth, facing- and
    /// scale-scaled.
    pub spawn_offset_x: f32,
    pub spawn_offset_y: f32,
}
impl FireBreathAttributes {
    /// The ten words at `base`.
    pub fn read(r: &Reader<'_>, base: u32) -> std::result::Result<Self, FighterDescError> {
        Ok(Self {
            minimum_ticks: r.s32(base)?,
            reach_recharge: r.f32(base + 0x04)?,
            life_recharge: r.f32(base + 0x08)?,
            reach_max: r.f32(base + 0x0C)?,
            reach_min: r.f32(base + 0x10)?,
            life_max: r.f32(base + 0x14)?,
            life_min: r.f32(base + 0x18)?,
            quake_interval: r.s32(base + 0x1C)?,
            spawn_offset_x: r.f32(base + 0x20)?,
            spawn_offset_y: r.f32(base + 0x24)?,
        })
    }
}

/// The breath's two fuels, kept by the user between moves.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fuels {
    /// The flames' reach left.
    pub reach: f32,
    /// The flames' lifetime left.
    pub life: f32,
}
impl Fuels {
    /// Both full: the users' resets (ftKp_Init_OnDeath,
    /// ftKb_SpecialN_800F190C).
    pub fn full(a: &FireBreathAttributes) -> Self {
        Self {
            reach: a.reach_max,
            life: a.life_max,
        }
    }
}

/// The motion scratch of the Fire Breath rows (Bowser's mv.kp.specials,
/// Kirby's mv.kb.specialn_kp).
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
    /// This frame's IASA let a flame out: the reach and life fuels as they
    /// stood before the frame's drain. The flame leaves once the IASA
    /// returns, with the RNG.
    pub pending_flame: Option<(f32, f32)>,
}

/// The six rows in table order: start, loop, end, then the aerial three.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BreathRows(pub [ActionId; 6]);
impl BreathRows {
    /// Six consecutive rows from `first`.
    pub const fn from(first: u16) -> Self {
        Self([
            ActionId(first),
            ActionId(first + 1),
            ActionId(first + 2),
            ActionId(first + 3),
            ActionId(first + 4),
            ActionId(first + 5),
        ])
    }
    const fn start(self, air: bool) -> ActionId {
        self.0[if air { 3 } else { 0 }]
    }
    const fn looping(self, air: bool) -> ActionId {
        self.0[if air { 4 } else { 1 }]
    }
    const fn end(self, air: bool) -> ActionId {
        self.0[if air { 5 } else { 2 }]
    }
}

/// What the Fire Breath's states ask of their user.
pub trait FireBreathUser {
    /// The rows the user's states change among (Kirby's by his hat).
    fn rows(f: &Fighter) -> BreathRows;
    /// Where the aerial start lands: Bowser's grounded start
    /// (ftKp_SpecialAirNStart_Coll); Kirby's grounded loop
    /// (ftKb_KpSpecialAirNStart_Coll, `li r4, 0x1bc` at 800FB218).
    const AIR_START_LANDS_IN_LOOP: bool;
    /// The last-generator word as the entry leaves it (Bowser 0, Kirby -1;
    /// both choose the next generator the same way).
    const FIRST_LAST_EFFECT: i32;
    /// The flame's item kind.
    const ITEM: ItemKind;
    /// The joint the flame leaves from (`fp->parts[n]`) and the offset
    /// lb_8000B1CC takes in it.
    const MOUTH_PART: usize;
    const MOUTH_OFFSET: Vec3;
    fn attributes(f: &Fighter) -> &FireBreathAttributes;
    fn breath(f: &mut Fighter) -> &mut Breath;
    fn fuels(f: &mut Fighter) -> &mut Fuels;
}

/// The six rows from `actions`, playing `first_animation` and the five
/// after it. The grounded rows' physics is ft_80084F3C, the aerial rows'
/// ft_80084DB0.
pub const fn rows<C: FireBreathUser>(actions: BreathRows, first_animation: i32) -> [MotionRow; 6] {
    let a = actions.0;
    [
        common::row(
            a[0],
            first_animation,
            start_anim::<C, false>,
            common::no_input,
            common::ground_friction,
            start_ground_collision::<C>,
        ),
        common::row(
            a[1],
            first_animation + 1,
            loop_anim::<C>,
            loop_input::<C, false>,
            common::ground_friction,
            loop_ground_collision::<C>,
        ),
        common::row(
            a[2],
            first_animation + 2,
            end_ground_anim,
            common::no_input,
            common::ground_friction,
            end_ground_collision::<C>,
        ),
        common::row(
            a[3],
            first_animation + 3,
            start_anim::<C, true>,
            common::no_input,
            callbacks::physics::fall,
            start_air_collision::<C>,
        ),
        common::row(
            a[4],
            first_animation + 4,
            loop_anim::<C>,
            loop_input::<C, true>,
            callbacks::physics::fall,
            loop_air_collision::<C>,
        ),
        common::row(
            a[5],
            first_animation + 5,
            end_air_anim,
            common::no_input,
            callbacks::physics::fall,
            end_air_collision::<C>,
        ),
    ]
}

/// ftKp_SpecialN_Enter (80134E58) / ftKp_SpecialAirN_Enter (80134ED0),
/// ftKb_SpecialNKp_800FA8B4 / 800FA958: the start row, the scratch cleared
/// with one animation cycle owed, and a hit group id for the flames
/// (Item_8026AE60).
pub fn enter<C: FireBreathUser>(f: &mut Fighter, air: bool, a: &FighterAssets) {
    let start = C::rows(f).start(air);
    change(f, start, MotionEntryFlags(0), 0.0, a).expect("Fire Breath assets");
    *C::breath(f) = Breath {
        cycles_owed: 1,
        last_effect: C::FIRST_LAST_EFFECT,
        ..Default::default()
    };
    f.core.item_requests.push(ItemRequest::NewHitGroup);
}

/// ftKp_SpecialNStart_Anim (80134F48) / ftKp_SpecialAirNStart_Anim
/// (8013507C), Kirby's ftKb_KpSpecial(Air)NStart_Anim: the loop follows
/// the start.
fn start_anim<C: FireBreathUser, const AIR: bool>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        let next = C::rows(f).looping(AIR);
        change(f, next, MotionEntryFlags(0), 0.0, p.assets)?;
    }
    Ok(None)
}

/// ftKp_SpecialN_Anim (80134F9C) / ftKp_SpecialAirN_Anim (801350D0),
/// Kirby's ftKb_KpSpecial(Air)N_Anim: each time the loop's animation comes
/// round to frame 0 one owed cycle is paid; a small camera quake at the
/// fighter every attribute +1C frames.
fn loop_anim<C: FireBreathUser>(
    f: &mut Fighter,
    p: AnimationPhase<'_>,
) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    let frame = f.animation.frame;
    let interval = C::attributes(f).quake_interval;
    let scratch = C::breath(f);
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

/// ftKp_SpecialNEnd_Anim (80135040) / ftKb_KpSpecialNEnd_Anim: Wait at the
/// animation's end (ft_8008A2BC).
fn end_ground_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::wait(f, p.assets)?;
    }
    Ok(None)
}

/// ftKp_SpecialAirNEnd_Anim (80135174) / ftKb_KpSpecialAirNEnd_Anim: Fall
/// at the animation's end.
fn end_air_anim(f: &mut Fighter, p: AnimationPhase<'_>) -> Result<Option<WaitChoice>> {
    f.step_animation(p.assets);
    if !f.animation.frames_remaining(&f.skeleton) {
        common::fall(f, p.assets)?;
    }
    Ok(None)
}

/// ftKp_SpecialN_IASA (801351B4) / ftKp_SpecialAirN_IASA (801352F8),
/// Kirby's ftKb_KpSpecial(Air)N_IASA: past the minimum frames, with B
/// released and no cycle owed, the end row; otherwise a flame on every
/// third frame. Then the frame's bookkeeping: the flame phase, both fuels
/// down by one to their floors (fsubs), the frame count up to the minimum.
fn loop_input<C: FireBreathUser, const AIR: bool>(f: &mut Fighter, p: InputPhase<'_>) {
    let (minimum, reach_min, life_min) = {
        let a = C::attributes(f);
        (a.minimum_ticks, a.reach_min, a.life_min)
    };
    let held = f.input.current.held.intersects(Buttons::B);
    let scratch = *C::breath(f);
    if scratch.frames >= minimum && !held && scratch.cycles_owed == 0 {
        let end = C::rows(f).end(AIR);
        change(f, end, MotionEntryFlags(0), 0.0, p.assets).expect("Fire Breath end assets");
    } else if scratch.flame_phase == 0 {
        let fuels = *C::fuels(f);
        C::breath(f).pending_flame = Some((fuels.reach, fuels.life));
    }
    let breath = C::breath(f);
    breath.flame_phase += 1;
    if breath.flame_phase >= FLAME_INTERVAL {
        breath.flame_phase = 0;
    }
    let fuels = C::fuels(f);
    fuels.reach -= 1.0;
    if fuels.reach < reach_min {
        fuels.reach = reach_min;
    }
    fuels.life -= 1.0;
    if fuels.life < life_min {
        fuels.life = life_min;
    }
    let breath = C::breath(f);
    breath.frames += 1;
    if breath.frames > minimum {
        breath.frames = minimum;
    }
}

/// ftKp_SpecialLw_80134ACC (80134ACC) / ftKb_SpecialNKp_800FA588
/// (800FA588), once the IASA that let the flame out has returned: the
/// flame's generator from the table (one or two HSD_Randi(32) draws), the
/// flame from the mouth, and every twelfth flame a new hit group and
/// attack instance; every third, the breath's sound for the life fuel
/// left.
pub fn release_flame<C: FireBreathUser>(f: &mut Fighter, rng: &mut gekko_math::HsdRng) {
    let Some((reach, life)) = C::breath(f).pending_flame.take() else {
        return;
    };
    let (offset_x, offset_y, life_min, life_max) = {
        let a = C::attributes(f);
        (a.spawn_offset_x, a.spawn_offset_y, a.life_min, a.life_max)
    };
    // lb_8000B1CC(fp->parts[n].joint, offset, &v).
    let c = &mut f.core;
    let mut mouth = melee_ft::fighter::caches::part_position(
        &mut c.skeleton,
        &c.animation,
        C::MOUTH_PART,
        C::MOUTH_OFFSET,
    );
    if f.player.scale != 1.0 {
        unimplemented!(
            "ftKp_SpecialLw_80134ACC / ftKb_SpecialNKp_800FA588: a scaled breather's flame offset (x34_scale.y)"
        );
    }
    let scale = f.player.scale;
    let facing = f.physics.facing;
    // Bowser 80134B10 / Kirby 800FA5E4: fmuls; 80134B18 / 80134B2C and
    // 800FA5EC / 800FA600: fmadds.
    mouth.x = gekko_math::fma::fmadds(scale, offset_x * facing, mouth.x);
    mouth.y = gekko_math::fma::fmadds(offset_y, scale, mouth.y);

    // After generator 1 or 2 the next comes from the second row; otherwise
    // (0, 3, or Kirby's -1 at the start) the first row picks the row.
    let effect = match C::breath(f).last_effect {
        1 | 2 => FLAME_EFFECTS[1][rng.randi(32) as usize],
        _ => {
            if FLAME_EFFECTS[0][rng.randi(32) as usize] == 0 {
                FLAME_EFFECTS[1][rng.randi(32) as usize]
            } else {
                FLAME_EFFECTS[2][rng.randi(32) as usize]
            }
        }
    };
    C::breath(f).last_effect = effect;

    // itKoopaFlame_Spawn: prev_pos is the mouth on the plane; the spawn
    // sweep starts at the fighter's ECB centre (it_8026BB68 ->
    // ftLib_80086990).
    mouth.z = 0.0;
    let mut spawn = SpawnItem::ray(C::ITEM, f.player.id, mouth, facing);
    spawn.previous_position = mouth;
    // Retail 800869AC..BC: fadds, fmuls, fadds.
    let ecb = &f.collision.data.ecb;
    let midpoint = 0.5 * (ecb.top.y + ecb.bottom.y);
    spawn.position = Vec3::new(
        f.physics.position.x + 0.0,
        f.physics.position.y + midpoint,
        f.physics.position.z + 0.0,
    );
    // 80134BCC / 80134BD4, 800FA6A0 / 800FA6A8: fctiwz of both fuels,
    // itKoopaFlame_Spawn's s32 arguments.
    spawn.spawn_argument = it_koopaflame::spawn_argument(
        effect,
        gekko_math::msl::fctiwz(reach),
        gekko_math::msl::fctiwz(life),
    );
    // The flame takes the attack instance current at its spawn, before a
    // cycle's first flame starts the next one below.
    spawn.stale_source = f.combat.stale.attack();
    f.core.item_requests.push(ItemRequest::SpawnInGroup(spawn));

    let flames = C::breath(f).cycle_flames;
    if flames == 0 {
        // Item_8026AE60, ft_80089824's statistics, then ft_800892A0.
        f.core.item_requests.push(ItemRequest::NewHitGroup);
        f.combat.stale.new_instance();
    }
    if flames % SOUND_INTERVAL == 0 {
        // 80134C44..80134C50 / 800FA718..800FA724: fsubs, fsubs, fdivs.
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
    let scratch = C::breath(f);
    scratch.cycle_flames += 1;
    scratch.cycle_flames %= FLAMES_PER_CYCLE;
}

/// ftKp_SpecialLw_80134D78 (80134D78) / ftKb_SpecialNKp_800FA7D4
/// (800FA7D4), outside the user's Fire Breath rows: both fuels regain
/// their per-frame share, up to full (fadds, then the clamp by fcmpo).
pub fn refill<C: FireBreathUser>(f: &mut Fighter) {
    let (reach_recharge, reach_max, life_recharge, life_max) = {
        let a = C::attributes(f);
        (a.reach_recharge, a.reach_max, a.life_recharge, a.life_max)
    };
    let fuels = C::fuels(f);
    fuels.reach += reach_recharge;
    if fuels.reach > reach_max {
        fuels.reach = reach_max;
    }
    fuels.life += life_recharge;
    if fuels.life > life_max {
        fuels.life = life_max;
    }
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

/// ftKp_SpecialNStart_Coll (801354F8) / ftKb_KpSpecialNStart_Coll.
fn start_ground_collision<C: FireBreathUser>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let next = C::rows(f).start(true);
    leave_floor(f, p, next, common::GROUND_AIR)
}
/// ftKp_SpecialN_Coll (80135564) / ftKb_KpSpecialN_Coll.
fn loop_ground_collision<C: FireBreathUser>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let next = C::rows(f).looping(true);
    leave_floor(f, p, next, LOOP_SWITCH)
}
/// ftKp_SpecialNEnd_Coll (801355D0) / ftKb_KpSpecialNEnd_Coll.
fn end_ground_collision<C: FireBreathUser>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let next = C::rows(f).end(true);
    leave_floor(f, p, next, common::GROUND_AIR)
}
/// ftKp_SpecialAirNStart_Coll (8013563C) / ftKb_KpSpecialAirNStart_Coll:
/// Bowser lands in the grounded start, Kirby in the grounded loop.
fn start_air_collision<C: FireBreathUser>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let rows = C::rows(f);
    let next = if C::AIR_START_LANDS_IN_LOOP {
        rows.looping(false)
    } else {
        rows.start(false)
    };
    reach_floor(f, p, next, common::GROUND_AIR)
}
/// ftKp_SpecialAirN_Coll (801356A8) / ftKb_KpSpecialAirN_Coll.
fn loop_air_collision<C: FireBreathUser>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let next = C::rows(f).looping(false);
    reach_floor(f, p, next, LOOP_SWITCH)
}
/// ftKp_SpecialAirNEnd_Coll (80135714) / ftKb_KpSpecialAirNEnd_Coll.
fn end_air_collision<C: FireBreathUser>(f: &mut Fighter, p: CollisionPhase<'_>) -> Result<()> {
    let next = C::rows(f).end(false);
    reach_floor(f, p, next, common::GROUND_AIR)
}
