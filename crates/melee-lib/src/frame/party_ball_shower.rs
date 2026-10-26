//! it_80289BE8 (80289BE8): an opened Party Ball's shower, which the scene
//! runs on [`melee_it::ItemEvent::PartyBallContents`].
use super::*;
use gekko_math::fma::fmsubs;
use melee_types::ItemKind;

/// it_80289BE8_inline's drift scale and the halves sdata2 holds for it
/// (5.0, 1.2, 0.6, 0.3, 0.1, 0.05, 0.025, 1.4, 0.7, 0.35, 1.8, 0.9, 0.45):
/// x = fmsubs(scale, r, scale / 2), y = fmsubs(scale / 2, r, scale / 4).
#[derive(Clone, Copy)]
struct Drift {
    scale: f32,
    half: f32,
    quarter: f32,
}
/// The monster's drift.
const MONSTER_DRIFT: Drift = Drift { scale: 0.1, half: 0.05, quarter: 0.025 };
/// The attribute kind's.
const KIND_DRIFT: Drift = Drift { scale: 1.4, half: 0.7, quarter: 0.35 };
/// The Foods'.
const FOODS_DRIFT: Drift = Drift { scale: 1.8, half: 0.9, quarter: 0.45 };
/// A random draw's.
const RANDOM_DRIFT: Drift = Drift { scale: 1.2, half: 0.6, quarter: 0.3 };
/// it_80289BE8: the shower drops from 5 below the ball (pos.y -= 5.0f).
const DROP: f32 = 5.0;

/// What the weights chose.
enum Shower {
    Kind(ItemKind, i32),
    Foods,
    Random,
}

impl Drift {
    /// it_80289BE8_inline: two Randf draws, the x first.
    fn draw(self, rng: &mut gekko_math::HsdRng) -> Vec3 {
        let x = fmsubs(self.scale, rng.randf(), self.half);
        let y = fmsubs(self.half, rng.randf(), self.quarter);
        Vec3::new(x, y, 0.0)
    }
}

impl Runtime {
    /// it_80289BE8 (80289BE8): an opened Party Ball's shower (see
    /// [`melee_it::ItemEvent::PartyBallContents`]). A coin match's coins
    /// (it_8026F8B4) are not a Versus rule; ftLib_800874CC's tally of what
    /// came out is the owner's match statistics only.
    pub(super) fn release_party_ball(
        &mut self,
        ball: u32,
        release: melee_it::ItemEvent,
        world: &mut World,
    ) -> Result<()> {
        let melee_it::ItemEvent::PartyBallContents {
            weights: [kind_weight, food_weight, random_weight],
            kind,
            count,
            center,
            position,
        } = release
        else {
            unreachable!("a Party Ball's shower")
        };
        let lowered = Vec3::new(position.x, position.y - DROP, position.z);
        // it_80289BE8_inline(0.1), then lowered once more for it_8026F6BC.
        let drift = MONSTER_DRIFT.draw(&mut self.state.rng);
        let monster_at = Vec3::new(lowered.x, lowered.y - DROP, lowered.z);
        if self.shower_monster(ball, center, monster_at, drift, world)? {
            return Ok(());
        }
        let state = &mut self.state;
        let spawner = state.item_spawner.expect("a Party Ball without the spawner");
        let food_weight = if spawner.switched_on(ItemKind::Foods) {
            food_weight
        } else {
            0
        };
        let roll = state.rng.randi(kind_weight + food_weight + random_weight);
        // The food-or-random label: Foods under their weight, else random.
        let food_or_random = |roll: i32| {
            if roll < food_weight {
                Shower::Foods
            } else {
                Shower::Random
            }
        };
        let shower = if roll < kind_weight {
            let kind = ItemKind::try_from(kind).expect("the Party Ball's item kind");
            if kind == ItemKind::MBall {
                unimplemented!("it_80289BE8: Poke Balls from a Party Ball (it_8026C704)");
            }
            if spawner.switched_on(kind) {
                Shower::Kind(kind, count)
            } else {
                food_or_random(state.rng.randi(food_weight + random_weight))
            }
        } else {
            food_or_random(roll - kind_weight)
        };
        match shower {
            Shower::Kind(kind, count) => {
                for _ in 0..count {
                    self.shower_one(ball, Some(kind), KIND_DRIFT, center, lowered, world)?;
                }
            }
            Shower::Foods => {
                let count = self.state.rng.randi(5) + 10;
                for _ in 0..count {
                    let foods = Some(ItemKind::Foods);
                    self.shower_one(ball, foods, FOODS_DRIFT, center, lowered, world)?;
                }
            }
            Shower::Random => {
                let count = self.state.rng.randi(2) + 3;
                for _ in 0..count {
                    self.shower_one(ball, None, RANDOM_DRIFT, center, lowered, world)?;
                }
            }
        }
        Ok(())
    }

    /// it_8026F6BC(gobj, pos, vel, true): under the hold-kind 6 limit, one
    /// shower in ItCo +124 is a monster from it_804A0E60, created at the
    /// ball and given the small drift. Whether one came out.
    fn shower_monster(
        &mut self,
        ball: u32,
        center: Vec3,
        at: Vec3,
        drift: Vec3,
        world: &mut World,
    ) -> Result<bool> {
        let state = &mut self.state;
        let common = state.items.common().random_items;
        let monsters = state.items.iter().filter(|item| item.hold_kind == 6).count();
        if monsters as i32 >= common.monster_limit || state.rng.randi(common.monster_odds) != 0 {
            return Ok(false);
        }
        let spawner = state.item_spawner.expect("the random item tables");
        let kind = spawner
            .monsters
            .pick(false, &mut state.rng)
            .expect("it_804A0E60 monster table");
        // it_8026F5C8: facing from the stage-plane position.
        let previous_position = Vec3::new(at.x, at.y, 0.0);
        let facing =
            crate::scene_items::facing_toward_fighters(previous_position, &mut state.fighters, &mut state.rng);
        let attack = state.items.get_mut(ball).expect("the Party Ball").stale_source;
        let spawned = crate::scene_items::spawn_released_monster(
            &mut state.items,
            &mut state.effects.gobjs,
            &state.assets.items,
            &mut state.map,
            world,
            &mut self.item_objects,
            crate::scene_items::Released {
                kind,
                position: center,
                previous_position,
                facing,
                attack,
            },
            &mut state.fighters,
            &mut state.rng,
        );
        let Some(id) = spawned else {
            return Ok(false);
        };
        state.items.get_mut(id).expect("the monster").velocity = drift;
        Ok(true)
    }

    /// One item of a Party Ball's shower: `kind`, or a draw from the
    /// contents table (it_8026F3AC, whose empty draw creates nothing and
    /// draws no more), with its drift, then created (it_8026F5C8) and given
    /// that drift (it_8026F53C with chk set).
    fn shower_one(
        &mut self,
        ball: u32,
        kind: Option<ItemKind>,
        drift: Drift,
        center: Vec3,
        lowered: Vec3,
        world: &mut World,
    ) -> Result<()> {
        let state = &mut self.state;
        let kind = match kind {
            Some(kind) => kind,
            None => {
                let spawner = state.item_spawner.expect("the spawner");
                let poke_balls = state.items.iter().filter(|item| item.kind == ItemKind::MBall).count();
                let limit = state.items.common().hold_limits[9].unwrap_or(0);
                let barred = !spawner.switched_on(ItemKind::MBall) || poke_balls >= limit;
                if barred && poke_balls > 0 {
                    unimplemented!("it_8026C75C: Item_804A0C64.x1C counts Poke Balls (item.c:408)");
                }
                let Some(kind) = spawner.contents.pick(barred, &mut state.rng) else {
                    return Ok(());
                };
                if kind == ItemKind::MBall {
                    unimplemented!("it_80289BE8_spawn_random: a Poke Ball (it_8026C704)");
                }
                kind
            }
        };
        let velocity = drift.draw(&mut state.rng);
        // it_8026F5C8: facing from the stage-plane position.
        let previous_position = Vec3::new(lowered.x, lowered.y, 0.0);
        let facing =
            crate::scene_items::facing_toward_fighters(previous_position, &mut state.fighters, &mut state.rng);
        let attack = state.items.get_mut(ball).expect("the Party Ball").stale_source;
        let spawned = crate::scene_items::spawn_released_item(
            &mut state.items,
            &mut state.effects.gobjs,
            &state.assets.items,
            &mut state.map,
            world,
            &mut self.item_objects,
            crate::scene_items::Released {
                kind,
                position: center,
                previous_position,
                facing,
                attack,
            },
            &mut state.rng,
        );
        if let Some(id) = spawned {
            state.items.get_mut(id).expect("the shower's item").velocity = velocity;
        }
        Ok(())
    }
}
