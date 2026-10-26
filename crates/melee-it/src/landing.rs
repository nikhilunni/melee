//! An item falling to rest (itgroundcoll.c): the airborne pass that bounces
//! off walls and ceilings and lands on floors with a thrown item's break
//! check (it_8026E15C), and the grounded pass of an item that may slide or
//! leave the floor (it_8026E8C4). The settling itself (it_8026DC24,
//! it_8026DD5C, it_80277040) is map.rs's.
use crate::{desc::ItemAssets, EnteredAir, ItemCore};
use gekko_math::HsdRng;

/// What an airborne pass with landings (it_8026E15C) did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AirLanding {
    /// Still in the air (or bounced off a wall or ceiling, or touched a
    /// floor too fast to settle).
    Airborne,
    /// Settled on a floor: the caller's callback runs.
    Landed,
    /// The first landing after a throw broke it (Item_8026A8EC).
    Broken,
    /// Settled on a floor it slides down: the kind's EnteredAir has run
    /// (it_8026DD5C, Item_8026ADC0) and the caller's callback does not.
    Sliding,
}

/// What a sliding state's grounded pass (it_8026E8C4) found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SlidePass {
    /// Off the floor (it_802762BC): the caller's airborne callback runs.
    Airborne,
    /// The slide is over: the caller's resting callback runs.
    Stopped,
    /// Still sliding: no callback.
    Sliding,
}

impl ItemCore {
    /// it_8026E15C (8026E15C): an airborne pass (it_8026DAA8's bits); a wall
    /// or ceiling reflects the velocity (it_80276FC4); a floor counts the
    /// landing (it_8026DDFC) and, once the item settles (it_8026DC24), its
    /// hitboxes go (it_802725D4) and it comes to rest (it_8026DD5C).
    pub fn air_collision_with_landing(
        &mut self,
        map: &mut melee_mp::CollMap,
        assets: &ItemAssets,
        entered_air: Option<EnteredAir>,
        rng: &mut HsdRng,
    ) -> AirLanding {
        self.air_collision_with_landing_after(map, assets, entered_air, rng, |_| {})
    }

    /// [`Self::air_collision_with_landing`] for a kind whose own pass
    /// inlines it and does `on_contact` first on any contact (Whispy's
    /// apple's HSD_Randi(3) of its landing sound).
    pub fn air_collision_with_landing_after(
        &mut self,
        map: &mut melee_mp::CollMap,
        assets: &ItemAssets,
        entered_air: Option<EnteredAir>,
        rng: &mut HsdRng,
        on_contact: impl FnOnce(&mut HsdRng),
    ) -> AirLanding {
        let bits = self.air_contact_bits(map);
        if bits & 0xF == 0 {
            return AirLanding::Airborne;
        }
        on_contact(rng);
        // it_80276FC4's spark draws (it_80278800) precede the landing
        // count's (it_8026DDFC, retail 0x8026DE58).
        self.bounce_off_surfaces_drawing(bits, map, assets, rng);
        if bits & 1 == 0 {
            return AirLanding::Airborne;
        }
        if !self.count_landing(assets, rng) {
            return AirLanding::Broken;
        }
        if !self.settle(assets) {
            return AirLanding::Airborne;
        }
        self.clear_hitboxes();
        if self.come_to_rest(assets, entered_air) {
            AirLanding::Landed
        } else {
            AirLanding::Sliding
        }
    }

    /// it_8026E8C4 (8026E8C4): a sliding state's grounded pass
    /// (mpColl_8004B108). Off the floor the item is airborne (it_802762BC).
    /// On it the slope test runs (it_80277040) and a wall ends the slide for
    /// good (xD5C 0); once the slide no longer moves the item the surface is
    /// remembered (it_80276CEC) and it has stopped.
    pub fn ground_collision_with_slide(
        &mut self,
        map: &mut melee_mp::CollMap,
        assets: &ItemAssets,
    ) -> SlidePass {
        let mut collision = self.refresh_collision();
        let grounded = map.ground_collide_pass(&mut collision, None);
        self.position = collision.cur_pos;
        self.floor_line_from(&collision, grounded);
        self.collision = Some(collision);
        if !grounded {
            self.enter_air();
            return SlidePass::Airborne;
        }
        self.slides(assets);
        if self.wall_bits() != 0 {
            self.slide_mode = 0;
        }
        if self.sliding && self.slide_mode != 0 {
            return SlidePass::Sliding;
        }
        self.remember_rest_surface();
        SlidePass::Stopped
    }

    /// it_8026DDFC (8026DDFC): the landing counts; the first after a throw
    /// breaks the item once it has been thrown `throw_break >> 4` times or
    /// when HSD_Randi(`throw_break & 0xF`) draws zero. False when it broke.
    fn count_landing(&mut self, assets: &ItemAssets, rng: &mut HsdRng) -> bool {
        self.land_count += 1;
        if self.land_count != 1 || self.throw_count == 0 {
            return true;
        }
        let limit = assets.throw_break;
        if self.throw_count == u32::from(limit >> 4) || rng.randi(i32::from(limit & 0xF)) == 0 {
            // destroy_type = 1, Item_8026A8EC.
            return false;
        }
        true
    }

    /// it_802725D4 (802725D4): every hitbox goes.
    pub fn clear_hitboxes(&mut self) {
        if self.hitboxes.iter().any(Option::is_some) {
            self.park_hitboxes();
            for history in &mut self.reflection_history {
                history.clear();
            }
        }
    }
}
