//! A Coin match's coins (It_Kind_Unk4, it_2E5A.c 802E5AC4..802E6AE8): the
//! spill's plan (how many coins, of which tier, how fast), the coin's
//! floor pass and its pickup sphere. The kind's states are `it-coin`'s.
use crate::{desc::ItemAssets, ItemCore, ItemScratch};
use gekko_math::{
    fma::fmadds,
    msl::{fabsf, fctiwz, sqrtf, sqrtf_accurate},
    HsdRng,
};
use hsd_types::Vec3;

/// it_2E5A_Attrs: 0x3C bytes of floats, then three 0x2C-byte tiers.
pub const SPECIAL_ATTRIBUTES: u32 = 0xC0 / 4;
/// it_2E5A_TierEntry's size in words.
const TIER_WORDS: usize = 0x2C / 4;
/// The first tier's word.
const TIERS: usize = 0x3C / 4;

/// One coin tier (it_2E5A_TierEntry): its value and look.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoinTier {
    /// +10: Item.xD84, the bounce sound (it_8027321C).
    pub bounce_sound: u32,
    /// +14: the coins this tier is worth.
    pub value: i32,
    /// +18: Item.scl.
    pub scale: f32,
    /// +1C: the ECB (it_80275D5C).
    pub collision_box: melee_types::mp::ItEcb,
}

/// it_2E5A_Attrs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CoinAttributes {
    /// x0: the lifetime (it_80275158).
    pub lifetime: f32,
    /// x4: frames before a fighter not its owner may take it (x10).
    pub pickup_delay: f32,
    /// x8: frames before its owner may (x14; it_80275444 then).
    pub owner_pickup_delay: f32,
    /// xC: the horizontal speed's random range.
    pub spread: f32,
    /// x10: the extra horizontal speed of a spill launched sideways.
    pub sideways_speed: f32,
    /// x14: what a bounce keeps of the horizontal speed.
    pub bounce_drag: f32,
    /// x18: the vertical speed.
    pub rise: f32,
    /// x1C: the launch speed's decay per coin.
    pub launch_decay: f32,
    /// x20: the vertical speed's random range.
    pub rise_spread: f32,
    /// x24: the value's random surplus, in percent.
    pub value_spread: f32,
    /// x28: the pickup sphere's radius, in 1/256ths.
    pub pickup_radius: f32,
    /// x2C: the value multiplier by the spiller's rank (gm_8016C6C0).
    pub rank_multipliers: [f32; 4],
    pub tiers: [CoinTier; 3],
}

impl CoinAttributes {
    pub fn read(words: &[f32]) -> Self {
        let int = |i: usize| words[i].to_bits() as i32;
        let tier = |t: usize| {
            let base = TIERS + t * TIER_WORDS;
            CoinTier {
                bounce_sound: words[base + 4].to_bits(),
                value: int(base + 5),
                scale: words[base + 6],
                collision_box: melee_types::mp::ItEcb {
                    top: words[base + 7],
                    bottom: words[base + 8],
                    right: words[base + 9],
                    left: words[base + 10],
                },
            }
        };
        Self {
            lifetime: words[0],
            pickup_delay: words[1],
            owner_pickup_delay: words[2],
            spread: words[3],
            sideways_speed: words[4],
            bounce_drag: words[5],
            rise: words[6],
            launch_decay: words[7],
            rise_spread: words[8],
            value_spread: words[9],
            pickup_radius: words[10],
            rank_multipliers: [words[11], words[12], words[13], words[14]],
            tiers: [tier(0), tier(1), tier(2)],
        }
    }

    /// it_802E6380's tier: the largest whose value the rest still covers.
    fn tier_for(&self, rest: i32) -> usize {
        if rest >= self.tiers[2].value {
            2
        } else if rest >= self.tiers[1].value {
            1
        } else {
            0
        }
    }
}

/// Item.xDD4_itemVar.it_2E5A.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CoinState {
    /// x0: the coins it is worth.
    pub value: i32,
    /// x4: its tier.
    pub tier: u8,
    /// Item.xD84: the bounce sound (it_8027321C), its tier's, then the
    /// first tier's from the second landing.
    pub bounce_sound: u32,
    /// x8 (it_802E5FXX_struct.x14): fighters can take it; a screen or star
    /// KO's coins cannot and fall through the stage.
    pub collectable: bool,
    /// xC: its tier's animation is on.
    pub animated: bool,
    /// x10 / x14: frames before a fighter not its owner, and its owner,
    /// may take it.
    pub pickup_delay: f32,
    pub owner_pickup_delay: f32,
    /// x18 b0 / b1: those delays have run out.
    pub open_to_others: bool,
    pub open_to_owner: bool,
    /// sub.x0: the pickup sphere's radius (also Item.x3C).
    pub radius: f32,
    /// sub.x8 / sub.x14: the sphere's centre now and last frame
    /// (lb_8000B1CC of the model's root).
    pub center: Vec3,
    pub previous_center: Vec3,
}

/// One coin of a spill (it_802E609C's loop): its tier, worth and launch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlannedCoin {
    pub tier: u8,
    pub value: i32,
    /// it_802E628C's velocity, or the spawn's for a coin without a launch.
    pub velocity: Vec3,
}

/// What it_802E5F8C hands it_802E609C (it_802E5FXX_struct).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spill {
    /// x4: the launch angle, normalized to [0, 2pi].
    pub angle: f32,
    /// x8: the launch speed.
    pub speed: f32,
    /// SpawnItem.facing_dir: -1 past pi.
    pub facing: f32,
    /// x14: fighters can take the coins.
    pub collectable: bool,
}

/// it_802E5F8C's angle and facing (802E5FE8..802E6040): pi/2 added and
/// wrapped in double precision, rounding each step.
pub fn spill(angle: f32, speed: f32, collectable: bool) -> Spill {
    let mut angle = (core::f64::consts::FRAC_PI_2 + f64::from(angle)) as f32;
    while angle < 0.0 {
        angle = (f64::from(angle) + core::f64::consts::TAU) as f32;
    }
    while f64::from(angle) > core::f64::consts::TAU {
        angle = (f64::from(angle) - core::f64::consts::TAU) as f32;
    }
    let facing = if f64::from(angle) > core::f64::consts::PI {
        -1.0
    } else {
        1.0
    };
    Spill {
        angle,
        speed,
        facing,
        collectable,
    }
}

/// it_802E61C4 (802E61C4): a spill of `count` coins is worth
/// count * the rank's multiplier, plus up to x24 percent, at least 1.
fn spill_value(a: &CoinAttributes, count: i32, rank: usize, rng: &mut HsdRng) -> i32 {
    let base = fctiwz(count as f32 * a.rank_multipliers[rank]);
    let random = rng.randf();
    // retail 802E6230 fmuls, 802E624C fmadds, 802E6250 fmuls.
    let surplus = fmadds(0.01, a.value_spread * random, 1.0);
    fctiwz(base as f32 * surplus).max(1)
}

/// it_802E628C (802E628C): a coin's launch along `angle` at `speed`.
fn launch_velocity(a: &CoinAttributes, angle: f32, speed: f32, facing: f32, rng: &mut HsdRng) -> Vec3 {
    let random = rng.randf();
    let from_vertical = (f64::from(angle) - core::f64::consts::FRAC_PI_2) as f32;
    // 802E62E4 / 802E62EC: two fmuls.
    let mut x = random * a.spread * facing;
    // "@307".."@310": pi/3, 2pi/3, 4pi/3, 5pi/3 as floats.
    #[allow(clippy::approx_constant)]
    let steep = (1.047_197_6..=2.094_395_2).contains(&angle) || (4.188_790_3..=5.235_987_7).contains(&angle);
    if steep {
        // 802E6330: fmadds.
        x = fmadds(a.sideways_speed, gekko_math::msl::cosf(from_vertical), x);
    }
    // 802E6340..48: fmuls, then two fadds.
    let y = a.rise + (random * a.rise_spread + speed);
    Vec3::new(x, y, 0.0)
}

/// it_802E609C's coins for a spill of `count` (it_802E5F8C: the value is
/// drawn first, then each coin's launch as it is made). The spiller's
/// standing is `rank` (gm_8016C6C0).
pub fn plan_spill(
    a: &CoinAttributes,
    count: i32,
    rank: usize,
    spill: Spill,
    rng: &mut HsdRng,
    out: &mut melee_types::fixed::FixedVec<PlannedCoin, MAX_SPILL>,
) -> i32 {
    out.clear();
    if count == 0 {
        return 0;
    }
    let total = spill_value(a, count, rank, rng);
    let mut rest = total;
    let mut speed = spill.speed;
    while rest != 0 {
        let tier = a.tier_for(rest);
        let value = a.tiers[tier].value;
        rest -= value;
        // it_802E6380: x8 *= x1C before each launch.
        speed *= a.launch_decay;
        let velocity = launch_velocity(a, spill.angle, speed, spill.facing, rng);
        out.push(PlannedCoin {
            tier: tier as u8,
            value,
            velocity,
        });
    }
    total
}

/// The most coins one spill may make; a larger one fails closed.
pub const MAX_SPILL: usize = 64;

/// Item_804A0C64 hold kind 12: the coins' count and limit (ItCo +20).
pub const HOLD_KIND: u8 = 12;

/// it_802E614C's SpawnItem for one coin of a spill: the spiller owns it
/// (x0 and x4 parent), at `position`, with its launch already planned. The
/// tier, worth and whether fighters can take it ride in the generic
/// fields the coin's set-up (it_802E6380, `it-coin`'s `launched`) reads
/// back with [`planned_coin`].
pub fn coin_spawn(
    owner: Option<u8>,
    position: Vec3,
    spill: Spill,
    coin: &PlannedCoin,
) -> crate::SpawnItem {
    crate::SpawnItem {
        owner,
        stale_source: None,
        attack_from_spawner: false,
        link_gobjs: 0,
        secondary_owner: owner,
        kind: melee_types::ItemKind::Unk4,
        hold_kind: HOLD_KIND,
        spawn_variant: i32::from(coin.tier),
        position,
        previous_position: position,
        velocity: coin.velocity,
        facing: spill.facing,
        damage: 0,
        auxiliary_damage: 0,
        spawn_argument: coin.value,
        // x44 b0.
        initial_collision: true,
        auxiliary_flags: [u8::from(spill.collectable), 0, 0],
        // x48 is left unset by it_802E614C.
        ground_or_air: melee_types::GroundOrAir::Air,
        launch_draws: 0,
        launch_seed: None,
    }
}

/// The coin [`coin_spawn`] describes, and whether fighters can take it.
pub fn planned_coin(spawn: &crate::SpawnItem) -> (PlannedCoin, bool) {
    (
        PlannedCoin {
            tier: spawn.spawn_variant as u8,
            value: spawn.spawn_argument,
            velocity: spawn.velocity,
        },
        spawn.auxiliary_flags[0] != 0,
    )
}

impl ItemCore {
    /// it_802E6380 (802E6380) for a new coin: its tier's worth, timers,
    /// scale and box, and its pickup sphere at the model's root.
    pub fn start_coin(
        &mut self,
        coin: &PlannedCoin,
        collectable: bool,
        attributes: &CoinAttributes,
        half_life_scale: f32,
    ) {
        let tier = &attributes.tiers[usize::from(coin.tier)];
        self.velocity = coin.velocity;
        // it_80275158.
        self.life_timer = attributes.lifetime;
        self.half_life = attributes.lifetime * half_life_scale;
        self.scale = tier.scale;
        self.model_scale = Vec3::new(tier.scale, tier.scale, tier.scale);
        self.set_collision_box(tier.collision_box);
        self.saved_collision_box = self.collision_box;
        self.collision_box.bottom *= self.scale;
        let mut state = CoinState {
            value: coin.value,
            tier: coin.tier,
            bounce_sound: tier.bounce_sound,
            collectable,
            pickup_delay: attributes.pickup_delay,
            owner_pickup_delay: attributes.owner_pickup_delay,
            ..Default::default()
        };
        if collectable {
            // 802E650C..1C: fmuls, fdivs.
            state.radius = 0.003906 * attributes.pickup_radius / self.scale;
            self.hitbox_size = state.radius;
            state.center = self.position;
            state.previous_center = state.center;
        }
        self.scratch = ItemScratch::Coin(state);
    }

    pub fn coin(&self) -> &CoinState {
        match &self.scratch {
            ItemScratch::Coin(state) => state,
            _ => panic!("coin scratch"),
        }
    }
    pub fn coin_mut(&mut self) -> &mut CoinState {
        match &mut self.scratch {
            ItemScratch::Coin(state) => state,
            _ => panic!("coin scratch"),
        }
    }

    /// it_802E6A74 (802E6A74), the coin's on_accessory: the sphere moves
    /// to the model's root.
    pub fn follow_coin_sphere(&mut self) {
        let position = self.position;
        let coin = self.coin_mut();
        if coin.collectable {
            coin.previous_center = coin.center;
            coin.center = position;
        }
    }

    /// it_802E5AC4 (802E5AC4): a point pass from the box's bottom; on a
    /// landing with `bounce`, the coin rests once it has slowed, or bounces
    /// off the floor with ItemAttr x58 of its speed (sound, spark, damage
    /// scale unless the floor was already touched), its horizontal speed
    /// cut by x14. True when it met the floor and stays (a bounce is false).
    pub fn coin_floor_pass(
        &mut self,
        map: &mut melee_mp::CollMap,
        assets: &ItemAssets,
        attributes: &CoinAttributes,
        bounce: bool,
        rng: Option<&core::cell::Cell<HsdRng>>,
    ) -> bool {
        let mut collision = self.refresh_collision();
        let bottom = self.collision_box.bottom;
        collision.cur_pos.y -= bottom;
        let landed = map.point_collide_pass(&mut collision);
        collision.cur_pos.y += bottom;
        self.position = collision.cur_pos;
        self.collision = Some(collision);
        if !landed || !bounce {
            return landed;
        }
        if fabsf(self.velocity.x) <= 0.00001 {
            self.velocity.x = 0.0;
        }
        if fabsf(self.velocity.y) <= 0.00001 {
            self.velocity.y = 0.0;
        }
        let slow = fabsf(self.velocity.x) <= assets.rest_speed
            && fabsf(self.velocity.y) <= assets.rest_speed;
        if slow || assets.bounce_scale == 0.0 {
            // itResetVelocity.
            self.velocity = Vec3::ZERO;
            self.land_count = 0;
            return landed;
        }
        let velocity = self.velocity;
        // 802E5C6C..74: fmuls, fmuls, fadds; the four-step inline sqrt.
        let speed = sqrtf_accurate(velocity.x * velocity.x + velocity.y * velocity.y);
        let collision = self.collision.as_ref().expect("item map collision");
        let normal = collision.floor.normal;
        // 802E5CE8 fmuls, 802E5CF4 fmadds.
        if fmadds(velocity.x, normal.x, velocity.y * normal.y) < 0.0 {
            // lbVector_Add_xy onto zeroed vectors (it_803B8718/8724): a
            // negative zero becomes positive.
            let mirrored = melee_lb::vector::mirror(velocity, normal);
            let mut direction = Vec3::new(0.0 + mirrored.x, 0.0 + mirrored.y, 0.0);
            let line_speed = map
                .floor_speed(collision)
                .map_or(Vec3::ZERO, |s| Vec3::new(0.0 + s.x, 0.0 + s.y, 0.0));
            direction = melee_lb::vector::normalize_xy(direction);
            // 802E5D6C..84: three fmuls, (x + y) then z + that; the
            // three-step inline sqrt.
            let length = sqrtf(direction.z * direction.z + (direction.x * direction.x + direction.y * direction.y));
            if length < 0.01 {
                direction.x = velocity.x;
                direction.y = -velocity.y;
            }
            // 802E5E08: speed * x58 once, then per axis.
            let kept = speed * assets.bounce_scale;
            direction.x *= kept;
            direction.y *= kept;
            self.platform_velocity = line_speed;
            self.velocity = direction;
        }
        if self.leave_repeated_contact(1) {
            // it_8027321C: a coin's sound is xD84; it_80277C40 with no side.
            let sound = self.coin().bounce_sound;
            self.sound_requests.push(sound);
            let mut drawn = rng.map(|cell| cell.get());
            self.push_bounce_spark(0, drawn.as_mut());
            if let (Some(cell), Some(drawn)) = (rng, drawn) {
                cell.set(drawn);
            }
            self.scale_hitbox_damage(assets.bounce_scale);
        }
        if self.land_count == 1 {
            self.coin_mut().bounce_sound = attributes.tiers[0].bounce_sound;
        }
        self.velocity.x *= attributes.bounce_drag;
        self.land_count += 1;
        if self.land_count >= 3 {
            self.bounce_spark_off = true;
        }
        // 802E5C3C: li r27, 0: a bounce is no landing.
        false
    }

    /// it_802E657C (802E657C): the lifetime runs down (true at its end);
    /// a collectable coin's two pickup delays run out, the owner's starting
    /// its blink (it_80275444).
    pub fn tick_coin_timers(&mut self) -> bool {
        if self.life_timer <= 0.0 {
            return true;
        }
        self.life_timer -= 1.0;
        let hits_owner = self.hits_owner;
        let coin = self.coin_mut();
        if !coin.collectable {
            return false;
        }
        if coin.pickup_delay <= 0.0 {
            coin.open_to_others = true;
        } else {
            coin.pickup_delay -= 1.0;
        }
        if coin.owner_pickup_delay <= 0.0 {
            if !hits_owner {
                coin.open_to_owner = true;
                // it_80275444: xDCD b5, b6, b7.
                self.hits_owner = true;
                self.strikes_kindred_items = true;
            }
        } else {
            coin.owner_pickup_delay -= 1.0;
        }
        false
    }
}
