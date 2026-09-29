//! Popo and Nana reach into each other: the Squall Hammer's callbacks read
//! the other climber (Player_GetEntityAtIndex(player, 0 | 1)) and change its
//! motion and hitlag link (x1A5C). The scene hands a climber its partner
//! around each proc (`CharacterCallbacks::OBSERVE_PARTNER` /
//! `ACT_ON_PARTNER`): `observe` records what the callbacks read, and a
//! callback that acts on the partner leaves `PartnerWork` for `act`.
use crate::climber;
use hsd_types::Vec3;
use melee_ft::fighter::{
    assets::{FighterAssets, Result},
    ActionId, Fighter, PartnerMotionChanges,
};
use melee_types::GroundOrAir;

/// The partner climber as the scene offered it before this proc.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PartnerView {
    /// Fighter.x8_spawnNum, what x1A5C names.
    pub spawn_number: u32,
    /// motion_id.
    pub action: ActionId,
    /// x2070's x2071_b0_3 class (None for a row without a known column).
    pub class: Option<u8>,
    /// cur_anim_frame.
    pub frame: f32,
    pub position: Vec3,
    pub self_velocity: Vec3,
    /// x74_anim_vel.
    pub animation_velocity: Vec3,
    pub ground_velocity: f32,
    /// xE4_ground_accel_1.
    pub ground_acceleration: f32,
    pub facing: f32,
    pub ground_or_air: GroundOrAir,
    /// ftPartGetRotX(fp, 0).
    pub root_rotation_x: f32,
    /// cmd_vars[1] (ftPp_SpecialS_8011F6FC).
    pub command_1: u32,
}

impl PartnerView {
    fn of(f: &Fighter) -> Self {
        let class = f.motion_flags().map(melee_ft::fighter::state::class_nibble);
        Self {
            spawn_number: f.spawn_number,
            action: f.motion_state.action,
            class,
            frame: f.animation.frame,
            position: f.physics.position,
            self_velocity: f.physics.self_velocity,
            animation_velocity: f.physics.animation_velocity,
            ground_velocity: f.physics.ground_velocity,
            ground_acceleration: f.physics.ground_acceleration,
            facing: f.physics.facing,
            ground_or_air: f.physics.ground_or_air,
            root_rotation_x: f.core.part_rotation_x(0),
            command_1: f.commands.variables[1],
        }
    }
}

/// What a climber's callbacks left for its partner this proc.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PartnerWork {
    /// ftNn_Init_80123954's success path: the partner joins the Squall
    /// Hammer.
    pub join: Option<crate::special_s::Join>,
    /// Fighter_UnkSetFlag_8006CFBC(partner) and partner->x1A5C = NULL.
    pub unlink: bool,
    /// Nana's SpecialS_0/_1 collision follows Popo's (ftPp_SpecialS_0_Coll's
    /// first branch), which copies his collision data.
    pub follow: bool,
}

/// OBSERVE_PARTNER.
pub fn observe(f: &mut Fighter, partner: &Fighter) {
    crate::climber::payload(f).partner = Some(PartnerView::of(partner));
}

/// The partner as observed before this proc. Every climber has one while
/// both are in play; a callback that needs it without one fails closed.
pub fn view(f: &Fighter) -> PartnerView {
    f.character
        .get::<crate::init::IceClimber>()
        .partner
        .expect("the Ice Climbers' partner fighter (Player_GetEntityAtIndex)")
}

pub fn work(f: &mut Fighter) -> &mut PartnerWork {
    &mut climber::payload(f).work
}

/// ACT_ON_PARTNER.
pub fn act(
    f: &mut Fighter,
    partner: &mut Fighter,
    assets: &FighterAssets,
    partner_assets: &FighterAssets,
    _map: &mut melee_mp::CollMap,
) -> Result<PartnerMotionChanges> {
    let work = std::mem::take(work(f));
    let mut changes = PartnerMotionChanges::default();
    if let Some(join) = work.join {
        crate::special_s::partner::join(partner, f, join, partner_assets)?;
        changes.partner = true;
    }
    if work.unlink {
        unlink(partner);
    }
    if work.follow {
        changes.fighter |= crate::special_s::partner::follow(f, partner, assets)?.fighter;
    }
    Ok(changes)
}

/// Fighter_UnkSetFlag_8006CFBC (8006CFBC) and x1A5C = NULL: a fighter held
/// in its partner's hitlag (x2219_b7) is marked to leave it (x221A_b1),
/// which the port reads from the missing link (`release_separated_hold`).
pub fn unlink(f: &mut Fighter) {
    f.core.combat.hitlag_link.partner = None;
}

/// ftPp_SpecialS_8011F68C (8011F68C) / ftNn_Init_801238E4 (801238E4): the
/// root upright, then both climbers leave each other's hitlag.
pub fn separate(f: &mut Fighter) {
    f.core
        .set_part_rotation(0, melee_ft::fighter::part_rotation::Axis::X, 0.0);
    if f.core.combat.hitlag_link.partner.is_some() {
        work(f).unlink = true;
    }
    unlink(f);
}
