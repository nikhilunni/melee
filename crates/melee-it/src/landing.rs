//! An item falling to rest (itgroundcoll.c): the airborne pass that bounces
//! off walls and ceilings and lands on floors with a thrown item's break
//! check (it_8026E15C), and the grounded pass of an item that may slide or
//! leave the floor (it_8026E8C4). The settling itself (it_8026DC24,
//! it_8026DD5C, it_80277040) is map.rs's.
use crate::{desc::ItemAssets, ItemCore};
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
        rng: &mut HsdRng,
    ) -> AirLanding {
        let bits = self.air_contact_bits(map);
        if bits & 0xF == 0 {
            return AirLanding::Airborne;
        }
        self.bounce_off_surfaces(bits, map, assets);
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
        self.come_to_rest(assets);
        AirLanding::Landed
    }

    /// it_8026E8C4 (8026E8C4): a grounded pass (mpColl_8004B108). Off the
    /// floor the item is airborne (it_802762BC) and false returns for the
    /// caller's air callback; on it the slope test runs (it_80277040) and a
    /// wall ends a platform drop, before the caller's ground callback.
    pub fn ground_collision_with_slide(
        &mut self,
        map: &mut melee_mp::CollMap,
        assets: &ItemAssets,
    ) -> bool {
        let mut collision = self.refresh_collision();
        let grounded = map.ground_collide_pass(&mut collision, None);
        self.position = collision.cur_pos;
        self.floor_line_from(&collision, grounded);
        self.collision = Some(collision);
        if !grounded {
            self.enter_air();
            return false;
        }
        self.slides(assets);
        if self.wall_bits() != 0 {
            self.platform_drop = 0;
        }
        // xDC8 x1F is set only by Item_8026ADC0; with xD5C zero the ground
        // callback runs either way.
        assert_eq!(self.platform_drop, 0, "it_8026E8C4: item platform drop");
        true
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
            self.hitboxes.fill(None);
            for history in &mut self.reflection_history {
                history.clear();
            }
        }
    }
}
