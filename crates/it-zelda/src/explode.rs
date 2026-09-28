//! Din's Fire's explosion (It_Kind_Zelda_DinFire_Explode),
//! itzeldadinfireexplode.c (802C4580..802C49C8): left where the fire
//! burst, sized and powered by the fire's charge.
use gekko_math::{fma::fmadds, msl::fctiwz};
use hsd_types::Vec3;
use melee_it::{
    desc::{ItemAssets, ItemCommonData},
    state_change::ANIM_UPDATE,
    DinFireState, ItemAnimationContext, ItemCollisionContext, ItemCore, ItemEvent,
    ItemEventContext, ItemLogic, ItemPhysicsContext, ItemScratch, ItemStateRow, SpawnItem,
};
use melee_types::ItemKind;

pub struct DinFireExplode;

/// it_803F7740's anim_id column.
pub const ARTICLE_STATES: [i32; 1] = [0];
/// ftData.x48_items index (ftZd_Init_OnLoad registers it second).
pub const ARTICLE_INDEX: u32 = 1;
/// itZeldaDinFireExplodeAttributes x0..x10.
pub const SPECIAL_ATTRIBUTES: u32 = 5;

/// it_802C46C4's it_80275158(gobj, 60.0f).
const LIFETIME: f32 = 60.0;
/// efSync_Spawn(0x4FA, gobj, jobj): the blast (efLib_CreateGenerator_
/// Attach_Scale 0x166).
const BLAST: u16 = 0x4FA;
/// it_802C46C4: the charge's share of the full charge that selects each
/// camera quake (below the second: none; then Camera_RequestQuake's small
/// and medium kinds, 2 and 3).
const SMALL_QUAKE_SHARE: f32 = 0.625;
const MEDIUM_QUAKE_SHARE: f32 = 0.8125;
const SMALL_QUAKE: u16 = 2;
const MEDIUM_QUAKE: u16 = 3;

/// The explosion's special attributes.
struct Attributes<'a>(&'a [f32]);
impl Attributes<'_> {
    /// x0: the full charge.
    fn full_charge(&self) -> f32 {
        self.0[0]
    }
    /// x4 / x8: the model's scale uncharged and fully charged.
    fn scale(&self) -> (f32, f32) {
        (self.0[1], self.0[2])
    }
    /// xC / x10: the hitbox's damage uncharged and per unit of charge.
    fn damage(&self) -> (f32, f32) {
        (self.0[3], self.0[4])
    }
}

/// it_802C4580's `scale` argument through SpawnItem.spawn_argument (the
/// fire's charge, a float, carried as its bits).
pub(crate) fn charge_argument(charge: f32) -> i32 {
    charge.to_bits() as i32
}

fn state(item: &mut ItemCore) -> &mut DinFireState {
    match &mut item.scratch {
        ItemScratch::DinFire(state) => state,
        _ => unreachable!("a Din's Fire explosion without its state"),
    }
}

static STATES: [ItemStateRow; 1] = [ItemStateRow {
    animation_id: ARTICLE_STATES[0],
    animation: blast_anim,
    physics: hold_still,
    collision: no_collision,
}];

/// it_3F2F.c's Logic66 row.
impl ItemLogic for DinFireExplode {
    const KIND: ItemKind = ItemKind::ZeldaDinFireExplode;
    const STATES: &'static [ItemStateRow] = &STATES;
    /// The logic row has no pickup callback.
    fn pickup_possible(_item: &ItemCore) -> bool {
        false
    }
    fn spawned(item: &mut ItemCore, _assets: &ItemAssets) {
        item.scratch = ItemScratch::DinFire(DinFireState::default());
    }
    /// it_802C4580 (802C4580) once Item_80268B18 returns, then it_802C46C4
    /// (802C46C4): the charge; not grabbable, shown, state 0, a 60-frame
    /// life; the blast; one animation step (Item_802694CC) and the
    /// animation callback; a quake for a strong charge (the rumble,
    /// it_80273598, has no simulated observer).
    fn launched(
        item: &mut ItemCore,
        assets: &ItemAssets,
        common: &ItemCommonData,
        spawn: &SpawnItem,
        _rng: &mut gekko_math::HsdRng,
    ) {
        item.command_variables = [0; 4];
        state(item).charge = f32::from_bits(spawn.spawn_argument as u32);
        item.grabbable = false;
        item.held = false;
        item.hidden = false;
        item.change_motion_with(0, ARTICLE_STATES[0], ANIM_UPDATE, assets);
        item.life_timer = LIFETIME;
        item.half_life = LIFETIME * common.half_life_scale;
        state(item).hitbox_size = 0.0;
        item.events.push(ItemEvent::OwnEffect { id: BLAST });
        state(item).effects = true;
        item.advance_animation(assets);
        // The animation callback itself, lifetime step included; its
        // result is dropped.
        blast(item, assets);
        spend_lifetime(item);
        let share = state(item).charge / Attributes(&assets.special_attributes).full_charge();
        let quake = if share < SMALL_QUAKE_SHARE {
            None
        } else if share < MEDIUM_QUAKE_SHARE {
            Some(SMALL_QUAKE)
        } else {
            Some(MEDIUM_QUAKE)
        };
        if let Some(kind) = quake {
            item.events.push(ItemEvent::Quake {
                kind,
                joint: 0,
                offset: Vec3::ZERO,
            });
        }
    }
    /// itZeldaDinFireExplode_Logic66_Clanked.
    fn clanked(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itZeldaDinFireExplode_Logic66_Absorbed.
    fn absorbed(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itZeldaDinFireExplode_Logic66_ShieldBounced.
    fn shield_bounced(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
    /// itZeldaDinFireExplode_Logic66_HitShield.
    fn hit_shield(_item: &mut ItemCore, _ctx: &ItemEventContext<'_>) -> bool {
        true
    }
}

/// itZeldadinfireexplode_UnkMotion0_Anim's body: the model's scale for the
/// charge (fsubs, fdivs, fmadds at 802C4870); a live hitbox takes the
/// charge's damage (fmadds at 802C4918, then __cvt_fp2unsigned) and its
/// authored size, read once, times the scale.
fn blast(item: &mut ItemCore, assets: &ItemAssets) {
    let a = Attributes(&assets.special_attributes);
    let (empty, full) = a.scale();
    let charge = state(item).charge;
    let scale = fmadds(charge, (full - empty) / a.full_charge(), empty);
    item.model_scale = Vec3::new(scale, scale, scale);
    if item.hitboxes[0].is_none() {
        return;
    }
    let (base, per_charge) = a.damage();
    // __cvt_fp2unsigned: the damage is positive, so it truncates.
    let damage = fctiwz(fmadds(charge, per_charge, base)) as u32;
    item.set_hitbox_damage(0, damage);
    let radius = item.hitboxes[0].as_ref().unwrap().descriptor.radius;
    let size = &mut state(item).hitbox_size;
    if *size == 0.0 {
        *size = radius;
    }
    let size = *size;
    item.hitboxes[0].as_mut().unwrap().descriptor.radius = size * scale;
}

/// itZeldadinfireexplode_UnkMotion0_Anim (802C47F8): the blast, then the
/// lifetime (it_80273130).
fn blast_anim(item: &mut ItemCore, ctx: &mut ItemAnimationContext<'_>) -> bool {
    blast(item, ctx.assets);
    spend_lifetime(item)
}

/// it_80273130: one frame of the lifetime; true once it runs out.
fn spend_lifetime(item: &mut ItemCore) -> bool {
    item.life_timer -= 1.0;
    item.life_timer <= 0.0
}

/// itZeldadinfireexplode_UnkMotion0_Phys: the vertical and depth velocity
/// clear (retail clears z twice and leaves x).
fn hold_still(item: &mut ItemCore, _ctx: &ItemPhysicsContext<'_>) {
    item.velocity.z = 0.0;
    item.velocity.y = 0.0;
}

fn no_collision(_item: &mut ItemCore, _ctx: &mut ItemCollisionContext<'_>) -> bool {
    false
}
