//! The Great Fox's cannon shot on Corneria (It_Kind_GreatFox_Laser),
//! it/kinds/itgreatfoxlaser.c (802EAF28..802EB5A8).
//!
//! The cannon's volley (grCorneria_801E1348) fires it_802EAF34: the shot
//! charges at the muzzle, a point in the Great Fox GObj's frame, for as
//! long as its animation plays, then flies left at its type's speed (x4
//! the big shot's, xC a volley's) with its type's damage, sixty ticks in
//! all. It has no collision callback: walls and floors do not stop it, and
//! a hit it deals does not end it.
use hsd_types::Vec3;
use melee_it::{desc::ItemAssets, state_change::ANIM_UPDATE, *};
use melee_types::{GroundOrAir, ItemKind};

pub struct GreatFoxLaser;

/// it_803F8FD0's anim_id column.
pub const ARTICLE_STATES: [i32; 2] = [0, 1];
/// itGreatFoxLaser_Attrs: x0..xC, all floats.
pub const SPECIAL_ATTRIBUTES: u32 = 4;
/// it_802EB268: the shot's ticks.
const TICKS: i32 = 60;

/// itGreatFoxLaser_Attrs.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0 / x8: the big shot's and a volley's damage, floats converted by
    /// __cvt_fp2unsigned (retail 0x802EB1B4).
    fn damage(&self, kind: i32) -> u32 {
        let damage = if kind == 0 { self.0[0] } else { self.0[2] };
        gekko_math::msl::fctiwz(damage) as u32
    }
    /// x4 / xC: their speeds.
    fn speed(&self, kind: i32) -> f32 {
        if kind == 0 {
            self.0[1]
        } else {
            self.0[3]
        }
    }
}

const SHOT: ItemStateRow = ItemStateRow {
    animation_id: 0,
    animation,
    physics,
    collision: no_collision,
};
static STATES: [ItemStateRow; 2] = [
    SHOT,
    ItemStateRow {
        animation_id: 1,
        ..SHOT
    },
];

impl ItemLogic for GreatFoxLaser {
    const KIND: ItemKind = ItemKind::GreatFoxLaser;
    const STATES: &'static [ItemStateRow] = &STATES;

    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    /// it_802EB1EC (Item_8026A810, inside Item_80268B18): it_8027B730's
    /// pushes off, the model upright and facing (it_8027C56C), no blast
    /// zone (xDCC b3), no slide (xD5C), not grabbable, kindred items safe
    /// (it_8027542C), then it_802EB268.
    fn spawned(item: &mut ItemCore, assets: &ItemAssets) {
        item.pushed_by_fighters = false;
        item.pushed_by_open_palm = false;
        // it_8027C56C: M_PI_2 is a double (fmul, frsp).
        item.rotation = Vec3::new(
            0.0,
            (core::f64::consts::FRAC_PI_2 * f64::from(item.facing)) as f32,
            0.0,
        );
        item.joints_dirty = true;
        item.blast_zone_checked = false;
        item.slide_mode = 0;
        item.grabbable = false;
        item.strikes_kindred_items = false;
        item.scratch = ItemScratch::GreatFoxLaser(GreatFoxLaserState::default());
        restart(item, 0, assets);
    }
    /// itGreatFoxLaser_Logic27_DmgDealt: the shot flies on.
    fn damage_dealt(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        false
    }
    /// itGreatFoxLaser_Logic27_Clanked.
    fn clanked(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        false
    }
    /// itGreatFoxLaser_Logic27_Absorbed.
    fn absorbed(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        false
    }
    /// it_2725_Logic27_Reflected (802EB45C): a charging shot moves its
    /// muzzle toward the reflector and turns. Not witnessed.
    fn reflected(_item: &mut ItemCore, _context: &ItemEventContext<'_>) -> bool {
        unimplemented!("itgreatfoxlaser.c:148: it_2725_Logic27_Reflected, a reflector turns the Great Fox's shot")
    }
}

/// it_802EB268 (802EB268): not grabbable, sixty ticks, unturned, state
/// `state` with ITEM_ANIM_UPDATE.
fn restart(item: &mut ItemCore, state: u16, assets: &ItemAssets) {
    item.grabbable = false;
    let laser = laser(item);
    laser.ticks = TICKS;
    laser.turned = false;
    item.change_motion_with(state, ARTICLE_STATES[usize::from(state)], ANIM_UPDATE, assets);
}

fn laser(item: &mut ItemCore) -> &mut GreatFoxLaserState {
    match &mut item.scratch {
        ItemScratch::GreatFoxLaser(laser) => laser,
        _ => unreachable!("a Great Fox laser's scratch"),
    }
}

/// Ground_801C0800 -> it_8026B40C: GrCn.dat's laser article. `None` when
/// the stage has none.
pub fn load(archive: &hsd_archive::Archive) -> Result<Option<ItemAssets>, String> {
    let Some(mut assets) = ItemAssets::from_stage_item(
        archive,
        ItemKind::GreatFoxLaser,
        &ARTICLE_STATES,
        SPECIAL_ATTRIBUTES,
        &[],
    )
    .map_err(|e| e.to_string())?
    else {
        return Ok(None);
    };
    // The hitbox rides a bone of the beam's animated model.
    assets
        .read_pose(archive)
        .map_err(|e| format!("Great Fox laser pose: {e}"))?;
    Ok(Some(assets))
}

/// it_802EAF34's SpawnItem: at the muzzle, facing left, unowned and still.
pub fn spawn(muzzle: Vec3) -> SpawnItem {
    SpawnItem {
        owner: None,
        stale_source: None,
        attack_from_spawner: false,
        link_gobjs: 0,
        secondary_owner: None,
        kind: ItemKind::GreatFoxLaser,
        // Item_802674AC: kinds from It_Kind_Arwing_Laser on.
        hold_kind: 5,
        spawn_variant: 0,
        position: muzzle,
        previous_position: muzzle,
        velocity: Vec3::ZERO,
        facing: -1.0,
        damage: 0,
        auxiliary_damage: 0,
        spawn_argument: 0,
        initial_collision: false,
        auxiliary_flags: [0; 3],
        ground_or_air: GroundOrAir::Air,
        launch_draws: 0,
        launch_seed: None,
    }
}

/// it_802EAF34 (802EAF34) after Item_80268B18: the map scale on the
/// model, the muzzle and type, hitbox 0's damage (it_80272460), then
/// it_802EB268 again.
pub fn launch(item: &mut ItemCore, offset: Vec3, kind: i32, map_scale: f32, assets: &ItemAssets) {
    item.scale = map_scale;
    item.model_scale = Vec3::new(map_scale, map_scale, map_scale);
    item.joints_dirty = true;
    let muzzle = item.position;
    *laser(item) = GreatFoxLaserState {
        offset,
        muzzle,
        turned: false,
        ticks: TICKS,
        kind,
    };
    let damage = Attributes(&assets.special_attributes).damage(kind);
    item.set_hitbox_damage(0, damage);
    restart(item, 0, assets);
}

/// The Great Fox moved: where the muzzle is now (lb_8000B1CC on its GObj's
/// JObj), for the next animation step.
pub fn follow(item: &mut ItemCore, muzzle: Vec3) {
    laser(item).muzzle = muzzle;
}

/// The muzzle's offset in the Great Fox's frame.
pub fn offset(item: &ItemCore) -> Vec3 {
    match &item.scratch {
        ItemScratch::GreatFoxLaser(laser) => laser.offset,
        _ => unreachable!("a Great Fox laser's scratch"),
    }
}

/// itGreatfoxlaser_UnkMotion1_Anim (802EB2C0): once the animation has
/// played out (it_80272C6C) the shot flies at its type's speed and damage;
/// until then it sits at the muzzle. It ends after sixty ticks.
fn animation(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    let a = Attributes(&ctx.assets.special_attributes);
    let charging = !item.article_animation_ended(ctx.assets);
    let state = *laser(item);
    if !charging {
        // retail 0x802EB31C: fmuls.
        item.velocity.x = a.speed(state.kind) * item.facing;
        item.set_hitbox_damage(0, a.damage(state.kind));
    } else {
        item.position = state.muzzle;
    }
    let laser = laser(item);
    if laser.ticks == 0 {
        return true;
    }
    laser.ticks -= 1;
    false
}

/// itGreatfoxlaser_UnkMotion1_Phys (802EB3C0): only a reflection's turn
/// (guarded in [`ItemLogic::reflected`]) does anything.
fn physics(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    assert!(
        !laser(item).turned,
        "itgreatfoxlaser.c:117: a reflected Great Fox shot"
    );
}

fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}
