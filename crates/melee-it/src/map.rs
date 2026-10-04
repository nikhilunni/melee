//! Item map collision: itmaplib.c and itgroundcoll.c. An item that falls and
//! lands keeps its own CollData (Item.x378_itemColl) with a fixed ECB box
//! from its ItemAttr, resolved by the same mpColl passes fighters use.
use crate::{desc::ItemAssets, ItemCore, SpawnItem};
use gekko_math::fma::fmadds;
use melee_types::{
    mp::{collide, CollData},
    GroundOrAir, ItemKind,
};

/// Item_802674AC's hold kind for character articles (It_Kind_Mario_Fire
/// up to It_Kind_Unk4).
const ARTICLE_HOLD_KIND: u8 = 8;
/// efAsync 0x405: an item's bounce spark (efLib_CreateGenerator 0x2C).
const BOUNCE_SPARK: u16 = 0x405;

/// What an airborne pass touched (it_8026E414's accumulated result bits).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AirContact {
    /// Bit 0: mpColl_800471F8 found a floor.
    pub floor: bool,
    /// Bit 1 (it_802763E0).
    pub ceiling: bool,
    /// Bits 3 / 2 (it_80276308); the right wall wins the stored line.
    pub left_wall: bool,
    pub right_wall: bool,
}
impl AirContact {
    /// The result word it_80276FC4 receives: 1 floor, 2 ceiling, 4 right
    /// wall, 8 left wall.
    fn bits(self) -> u32 {
        u32::from(self.floor)
            | u32::from(self.ceiling) << 1
            | u32::from(self.right_wall) << 2
            | u32::from(self.left_wall) << 3
    }
}

/// it_80275E98's ECB category (CollData.x34 b1234) by item kind range.
fn collision_category(kind: ItemKind, hold_kind: u8) -> u8 {
    // Character articles (hold kind 8) are category 5; common items below
    // It_Kind_L_Gun_Ray are category 2; stage enemies (It_Kind_Old_Kuri up
    // to It_Kind_Arwing_Laser, hold kind 4) are 3. Other ranges are not
    // ported yet.
    let stage_enemy =
        (ItemKind::OldKuri as u32..ItemKind::ArwingLaser as u32).contains(&(kind as u32));
    match hold_kind {
        8 => 5,
        0 if (kind as u32) < ItemKind::LGunRay as u32 => 2,
        4 if stage_enemy => 3,
        _ => unimplemented!("it_80275E98: collision category for {kind:?}"),
    }
}

impl ItemCore {
    /// Item_80267130 -> it_80275E98 (80275E98): the item's CollData with its
    /// fixed ECB box, then (x44_flag.b0) an initial pass from the spawn
    /// point. Airborne spawns only; it_80276174's grounded pass is unported.
    /// Even a clear spawn path passes through mpColl's six-unit
    /// subdivisions; assigning the position directly misses their rounding.
    pub fn initialize_collision(
        &mut self,
        spawn: SpawnItem,
        assets: &ItemAssets,
        map: &mut melee_mp::CollMap,
    ) {
        let mut collision = CollData {
            cur_pos: spawn.previous_position,
            ..Default::default()
        };
        map.coll_data_init(&mut collision);
        collision.x34_flags.b1234 = collision_category(self.kind, spawn.hold_kind);
        let b = assets.collision_box;
        melee_mp::set_ecb_source_fixed(
            &mut collision,
            b.top * self.scale,
            b.bottom * self.scale,
            b.right * self.scale,
            b.left * self.scale,
        );
        // Item_80267130 sets the facing before the kind's spawned callback
        // (Item_8026A810) may change it.
        melee_mp::set_facing_dir(&mut collision, if spawn.facing == -1.0 { -1 } else { 1 });
        collision.x50 = assets.collision_damage_multiplier;
        collision.last_pos = spawn.position;
        melee_mp::mark_ecb_clear(&mut collision);
        if spawn.initial_collision {
            assert_eq!(
                spawn.ground_or_air,
                GroundOrAir::Air,
                "it_80276174: grounded initial item collision is not ported"
            );
            // it_80276100.
            map.air_collide_pass(&mut collision, None);
        }
        self.position = collision.cur_pos;
        self.collision = Some(collision);
    }

    /// it_80276214 (80276214): the pass starts from the last resolved
    /// position and moves to the item's current one. xDCE_flag.b7 (always set
    /// at creation) also re-aims the fixed ECB at the model's rotation.
    /// The CollData is taken for the pass and stored back by the caller.
    pub(crate) fn refresh_collision(&mut self) -> CollData {
        let angle = self.ecb_angle();
        let mut collision = self.collision.take().expect("item map collision");
        collision.last_pos = collision.cur_pos;
        collision.cur_pos = self.position;
        // it_80276278 -> mpColl_800436E4 with it_80274990's joint rotation.
        melee_mp::set_ecb_angle(&mut collision, angle);
        collision
    }

    /// it_80274990: the rotation about xDC8 x17's axis (Z, X, then Y).
    fn ecb_angle(&self) -> f32 {
        match self.rotation_axis {
            0 => self.rotation.z,
            1 => self.rotation.x,
            _ => self.rotation.y,
        }
    }

    /// it_8026D62C (8026D62C) without its callbacks: a grounded pass
    /// (mpColl_8004B108). Returns false when the item left the ground, having
    /// switched it to the air (it_802762BC); the caller runs its state's
    /// callback. The xD5C fall-through-platform check (it_80277544) is not
    /// ported: xD5C stays 0 for the supported items.
    pub fn stay_grounded(&mut self, map: &mut melee_mp::CollMap) -> bool {
        let mut collision = self.refresh_collision();
        let grounded = map.ground_collide_pass(&mut collision, None);
        self.position = collision.cur_pos;
        self.floor_line_from(&collision, grounded);
        self.collision = Some(collision);
        if grounded {
            assert_eq!(self.platform_drop, 0, "it_80277544: item platform drop");
        } else {
            self.enter_air();
        }
        grounded
    }

    /// it_8026D8A4 (8026D8A4) without its callback: it_80276214 twice, so
    /// the pass starts where it ends, then a grounded pass that stops at
    /// edges (mpColl_8004B2DC). Returns whether the item stayed grounded and
    /// whether it stopped at an edge (it_802762D8). Unlike it_8026D62C a
    /// loss of ground does not switch the item to the air.
    pub fn walk_ground_pass(&mut self, map: &mut melee_mp::CollMap) -> (bool, bool) {
        let mut collision = self.refresh_collision();
        collision.last_pos = collision.cur_pos;
        collision.cur_pos = self.position;
        let grounded = map.ground_collide_stop_at_edge(&mut collision, None);
        self.position = collision.cur_pos;
        self.floor_line_from(&collision, grounded);
        let env = collision.env_flags as u32;
        let edge = env & (collide::LEFT_EDGE | collide::RIGHT_EDGE) != 0;
        self.collision = Some(collision);
        (grounded, edge)
    }

    /// it_8026DA08 (8026DA08): an airborne pass (mpColl_800471F8) that
    /// moves the item to the resolved position; a touched floor's line
    /// becomes xC30. Nothing lands or bounces.
    pub fn air_pass(&mut self, map: &mut melee_mp::CollMap) -> bool {
        let mut collision = self.refresh_collision();
        let floor = map.air_collide_pass(&mut collision, None);
        self.position = collision.cur_pos;
        self.floor_line_from(&collision, floor);
        self.collision = Some(collision);
        floor
    }

    /// it_8026DA70 (8026DA70): an airborne pass (mpColl_800471F8) that only
    /// senses the map: whether it found a floor. Unlike it_8026E414 the
    /// item keeps its own position; the CollData keeps the resolved one, so
    /// the next pass starts there.
    pub fn sense_air_collision(&mut self, map: &mut melee_mp::CollMap) -> bool {
        let mut collision = self.refresh_collision();
        let floor = map.air_collide_pass(&mut collision, None);
        self.collision = Some(collision);
        floor
    }

    /// it_80276308 (80276308): 8 for a left wall, 4 for a right one (which
    /// wins), whose line becomes xC30.
    pub fn wall_bits(&mut self) -> u32 {
        let collision = self.collision.as_ref().expect("item map collision");
        let env = collision.env_flags as u32;
        let mut bits = 0;
        if env & collide::LEFT_WALL_MASK != 0 {
            bits = 8;
            self.floor_line = collision.left_facing_wall.index;
        }
        if env & collide::RIGHT_WALL_MASK != 0 {
            self.floor_line = collision.right_facing_wall.index;
            bits = 4;
        }
        bits
    }

    /// it_802763E0 (802763E0): 2 for a ceiling, whose line becomes xC30.
    pub fn ceiling_bits(&mut self) -> u32 {
        let collision = self.collision.as_ref().expect("item map collision");
        if collision.env_flags as u32 & collide::CEILING_MASK == 0 {
            return 0;
        }
        self.floor_line = collision.ceiling.index;
        2
    }

    /// it_80276CB8 (80276CB8) -> it_802765BC(gobj, 0): on a floor, the model
    /// leans with its slope about xDC8 x17's axis (it_8027649C: the angle
    /// between the floor normal and up, signed by the normal's X and the
    /// facing).
    pub fn lean_with_floor(&mut self) {
        let collision = self.collision.as_ref().expect("item map collision");
        if collision.env_flags as u32 & collide::FLOOR_MASK == 0 {
            return;
        }
        let normal = collision.floor.normal;
        let angle = melee_lb::vector::angle(normal, hsd_types::Vec3::new(0.0, 1.0, 0.0));
        let direction = if normal.x < 0.0 { -1.0 } else { 1.0 };
        let lean = self.facing * (angle * direction);
        match self.rotation_axis {
            0 => self.rotation.z = -self.facing * lean,
            1 => self.rotation.x = lean,
            _ => self.rotation.y = lean,
        }
    }

    /// it_802762B0 (802762B0): the item counts as grounded.
    pub fn land_on_floor(&mut self) {
        self.ground_or_air = GroundOrAir::Ground;
    }

    /// it_8026E0F4 (8026E0F4): an airborne pass that never lands
    /// (mpColl_800477E0); a touched floor's line becomes xC30.
    pub fn airborne_pass(&mut self, map: &mut melee_mp::CollMap) -> bool {
        let mut collision = self.refresh_collision();
        let floor = map.air_collide_stay(&mut collision, None);
        self.position = collision.cur_pos;
        self.floor_line_from(&collision, floor);
        self.collision = Some(collision);
        floor
    }

    /// it_8026DA08 (8026DA08): an airborne pass (mpColl_800471F8) that
    /// never lands; a touched floor's line becomes xC30. Returns the
    /// collision's environment flags.
    pub fn airborne_contacts(&mut self, map: &mut melee_mp::CollMap) -> u32 {
        let mut collision = self.refresh_collision();
        let floor = map.air_collide_pass(&mut collision, None);
        self.position = collision.cur_pos;
        self.floor_line_from(&collision, floor);
        let env = collision.env_flags as u32;
        self.collision = Some(collision);
        env
    }

    /// it_8026E414 (8026E414) without its callbacks: an airborne pass
    /// (mpColl_800471F8), then the wall and ceiling bits. A landing restores
    /// the grounded ECB box (it_80275DFC) and grounds the item (it_802762B0).
    pub fn airborne_collision(
        &mut self,
        map: &mut melee_mp::CollMap,
        assets: &ItemAssets,
    ) -> AirContact {
        let mut collision = self.refresh_collision();
        let floor = map.air_collide_pass(&mut collision, None);
        let env = collision.env_flags as u32;
        let contact = AirContact {
            floor,
            ceiling: env & collide::CEILING_MASK != 0,
            left_wall: env & collide::LEFT_WALL_MASK != 0,
            right_wall: env & collide::RIGHT_WALL_MASK != 0,
        };
        self.position = collision.cur_pos;
        self.floor_line_from(&collision, floor);
        // it_80276308 / it_802763E0: a wall, then a ceiling, becomes xC30.
        if contact.left_wall {
            self.floor_line = collision.left_facing_wall.index;
        }
        if contact.right_wall {
            self.floor_line = collision.right_facing_wall.index;
        }
        if contact.ceiling {
            self.floor_line = collision.ceiling.index;
        }
        self.collision = Some(collision);
        if contact.ceiling || contact.left_wall || contact.right_wall {
            self.bounce_off_surfaces(contact.bits(), map, assets);
        }
        if floor {
            self.restore_collision_box(assets);
            self.ground_or_air = GroundOrAir::Ground;
        }
        contact
    }

    /// it_8026DAA8 (8026DAA8): an airborne pass (mpColl_800471F8) that
    /// neither lands nor bounces; the result word it_80276FC4 takes (1 floor,
    /// 2 ceiling, 4 right wall, 8 left wall) with the touched line as xC30.
    pub fn air_contact_bits(&mut self, map: &mut melee_mp::CollMap) -> u32 {
        let mut collision = self.refresh_collision();
        let floor = map.air_collide_pass(&mut collision, None);
        self.position = collision.cur_pos;
        self.floor_line_from(&collision, floor);
        self.collision = Some(collision);
        // it_80276308 (walls), then it_802763E0 (ceiling).
        let mut bits = u32::from(floor) | self.wall_bits();
        let collision = self.collision.as_ref().expect("item map collision");
        if collision.env_flags as u32 & collide::CEILING_MASK != 0 {
            self.floor_line = collision.ceiling.index;
            bits |= 2;
        }
        bits
    }

    /// it_8026E71C (8026E71C) without its callback: it_8026DAA8's pass
    /// and bits; a wall touched on the last pass as well nudges the item
    /// (it_80276D9C), and a floor restores the grounded box and grounds it
    /// (it_80275DFC, it_802762B0). True on any contact, when retail calls
    /// the callback.
    pub fn air_contact_any(&mut self, map: &mut melee_mp::CollMap, assets: &ItemAssets) -> bool {
        let bits = self.air_contact_bits(map);
        if bits & 0xF == 0 {
            return false;
        }
        if bits & 0xC != 0 {
            self.leave_repeated_contact(bits);
        }
        if bits & 1 != 0 {
            self.restore_collision_box(assets);
            self.land_on_floor();
        }
        true
    }

    /// it_8026E248 (8026E248) without its callback: it_8026DAA8's pass and
    /// bits; any contact bounces (it_80276FC4), and a floor then brings the
    /// item to rest once it has slowed (it_8026DE98, it_8026DC24,
    /// it_8026DD5C). True when retail calls the callback.
    pub fn bounce_to_rest(&mut self, map: &mut melee_mp::CollMap, assets: &ItemAssets) -> bool {
        let bits = self.air_contact_bits(map);
        if bits & 0xF == 0 {
            return false;
        }
        self.bounce_off_surfaces(bits, map, assets);
        if bits & 1 == 0 {
            return false;
        }
        // it_8026DE98 -> it_8026DDFC: a first landing of a thrown item may
        // break it (xD54_throwNum); fighter articles are never thrown.
        self.land_count += 1;
        self.settle(assets) && self.come_to_rest(assets)
    }

    /// it_8026DC24 (8026DC24): a first landing sets the landing spin and the
    /// grounded box; near-zero speeds are zero; true (and no velocity) once
    /// both axes are within ItemAttr x5C, or when the kind keeps no bounce
    /// speed (x58 zero). xDCD b4 is never set for the ported kinds.
    pub(crate) fn settle(&mut self, assets: &ItemAssets) -> bool {
        if self.land_count <= 1 {
            self.update_spin(assets.landing_spin_degrees);
            self.restore_collision_box(assets);
        }
        use gekko_math::msl::fabsf;
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
            self.velocity = hsd_types::Vec3::ZERO;
            return true;
        }
        false
    }

    /// it_8026DD5C (8026DD5C): the landing count resets and the item is
    /// grounded; unless it slides (it_80277040) its spin stops
    /// (it_80274740). it_80276CEC's stored normal only feeds the slide.
    pub(crate) fn come_to_rest(&mut self, assets: &ItemAssets) -> bool {
        self.land_count = 0;
        self.land_on_floor();
        if self.slides(assets) {
            unimplemented!("it_8026DD5C: a landed item sliding (entered_air)");
        }
        self.spin_speed = 0.0;
        match self.rotation_axis {
            0 => self.rotation.z = 0.0,
            1 => self.rotation.x = 0.0,
            _ => self.rotation.y = 0.0,
        }
        true
    }

    /// it_80277040 (80277040): whether the item slides down its floor. A
    /// kind without a slide speed (x50) never does; neither does anything
    /// on a floor flatter than ItCo +C0, which covers every ported stage's
    /// floors.
    pub(crate) fn slides(&mut self, assets: &ItemAssets) -> bool {
        if gekko_math::msl::fabsf(assets.slide_speed) < 0.00001 {
            return false;
        }
        let normal = self.collision.as_ref().expect("item map collision").floor.normal;
        if normal.x != 0.0 {
            unimplemented!("it_80277040: an item's slide on a sloped floor");
        }
        false
    }

    /// it_80276FC4 (80276FC4): a wall or ceiling contact reflects the item's
    /// velocity; unless the contact is a repeat, the bounce plays the kind's
    /// sound, sparks and scales the hitboxes' damage by ItemAttr x58.
    pub fn bounce_off_surfaces(&mut self, bits: u32, map: &melee_mp::CollMap, assets: &ItemAssets) {
        self.bounce(bits, map, assets, None);
    }

    /// `bounce_off_surfaces` for a caller that draws afterwards in the same
    /// callback (it_8026E15C's landing count): the spark's spread is drawn
    /// here, where it_80278800 draws it, not when the scene resolves it.
    pub(crate) fn bounce_off_surfaces_drawing(
        &mut self,
        bits: u32,
        map: &melee_mp::CollMap,
        assets: &ItemAssets,
        rng: &mut gekko_math::HsdRng,
    ) {
        self.bounce(bits, map, assets, Some(rng));
    }

    fn bounce(
        &mut self,
        bits: u32,
        map: &melee_mp::CollMap,
        assets: &ItemAssets,
        rng: Option<&mut gekko_math::HsdRng>,
    ) {
        self.reflect_velocity(map, assets);
        if !self.leave_repeated_contact(bits) {
            return;
        }
        // it_8027321C: xDCD b2 (muted) is never set for the ported kinds.
        self.sound_requests.push(assets.bounce_sound);
        self.push_bounce_spark(bits, rng);
        self.scale_hitbox_damage(assets.bounce_scale);
    }

    /// it_8027781C (8027781C) on its own, after the caller's map pass: true
    /// when the item touched a surface it was moving into (Mario's fireball
    /// bouncing along the floor).
    pub fn bounce_velocity(&mut self, map: &melee_mp::CollMap, assets: &ItemAssets) -> bool {
        self.reflect_velocity(map, assets)
    }

    /// it_8027781C (8027781C): the velocity mirrored off every touched
    /// surface it points into, summed, normalized and given back the old XY
    /// speed times ItemAttr x58. Moving lines' speeds become x64.
    fn reflect_velocity(&mut self, map: &melee_mp::CollMap, assets: &ItemAssets) -> bool {
        use melee_lb::vector::{length_xy, mirror, normalize_xy};
        let collision = self.collision.as_ref().expect("item map collision");
        let velocity = self.velocity;
        let speed = length_xy(velocity);
        let env = collision.env_flags as u32;
        let surfaces = [
            (
                collide::LEFT_WALL_MASK,
                collision.left_facing_wall.normal,
                map.left_wall_speed(collision),
            ),
            (
                collide::RIGHT_WALL_MASK,
                collision.right_facing_wall.normal,
                map.right_wall_speed(collision),
            ),
            (
                collide::CEILING_MASK,
                collision.ceiling.normal,
                map.ceiling_speed(collision),
            ),
            (
                collide::FLOOR_MASK,
                collision.floor.normal,
                map.floor_speed(collision),
            ),
        ];
        // it_803B857C / it_803B8570: both sums start at zero.
        let mut direction = hsd_types::Vec3::ZERO;
        let mut line_speed = hsd_types::Vec3::ZERO;
        let mut touched = false;
        for (mask, normal, surface_speed) in surfaces {
            // retail 8027791C / 80277928: fmuls, then fmadds.
            if env & mask == 0 || fmadds(velocity.x, normal.x, velocity.y * normal.y) >= 0.0 {
                continue;
            }
            // lbVector_Add_xy: two fadds.
            let mirrored = mirror(velocity, normal);
            direction.x += mirrored.x;
            direction.y += mirrored.y;
            if let Some(surface_speed) = surface_speed {
                line_speed.x += surface_speed.x;
                line_speed.y += surface_speed.y;
            }
            touched = true;
        }
        if !touched {
            return false;
        }
        if length_xy(direction) < 0.01 {
            direction.x = velocity.x;
            // retail 80277BB4 fmuls by -1.0: an exact negation.
            direction.y = -velocity.y;
        }
        let direction = normalize_xy(direction);
        // retail 80277BD0..E0: speed * x58 once, then per axis.
        let speed = speed * assets.bounce_scale;
        self.velocity = hsd_types::Vec3::new(direction.x * speed, direction.y * speed, direction.z);
        self.platform_velocity = line_speed;
        true
    }

    /// it_80276D9C (80276D9C): a contact already touching last pass nudges
    /// the item 1.5 away instead of counting as a bounce.
    fn leave_repeated_contact(&mut self, bits: u32) -> bool {
        let collision = self.collision.as_ref().expect("item map collision");
        let env = collision.env_flags as u32;
        if env & collide::LEFT_WALL_MASK != 0 && env & collide::RIGHT_WALL_MASK != 0 {
            unimplemented!("it_80276D9C: an item pressed between two walls");
        }
        let previous = collision.prev_env_flags as u32;
        let mut bounced = true;
        if bits & 4 != 0 && previous & collide::RIGHT_WALL_MASK != 0 {
            bounced = false;
            self.position.x += 1.5;
        }
        if bits & 8 != 0 && previous & collide::LEFT_WALL_MASK != 0 {
            bounced = false;
            self.position.x -= 1.5;
        }
        if bits & 2 != 0 && previous & collide::CEILING_MASK != 0 {
            bounced = false;
            self.position.y -= 1.5;
        }
        if bits & 1 != 0 && previous & collide::FLOOR_MASK != 0 {
            bounced = false;
            self.position.y += 1.5;
        }
        bounced
    }

    /// it_80277C40 (80277C40): effect 0x405 at the ECB point on the touched
    /// side, through it_80278800 with no spread. xDCF b0 (no spark) is never
    /// set for the ported kinds.
    fn push_bounce_spark(&mut self, bits: u32, rng: Option<&mut gekko_math::HsdRng>) {
        let ecb = self.collision.as_ref().expect("item map collision").ecb;
        let mut point = hsd_types::Vec2::default();
        if bits & 8 != 0 {
            point = ecb.right;
        }
        if bits & 4 != 0 {
            point = ecb.left;
        }
        if bits & 2 != 0 {
            point = ecb.top;
        }
        if bits & 1 != 0 {
            point = ecb.bottom;
        }
        let offset = hsd_types::Vec3::new(point.x, point.y, 0.0);
        if let Some(rng) = rng {
            // it_80278800's three draws (retail 0x80278A30, 0x80278A54,
            // 0x80278A78) scale a zero spread; then efAsync kind 2 queues
            // on the item, as the scene does for a ScriptEffect.
            for _ in 0..3 {
                rng.randf();
            }
            self.queued_events.push(crate::ItemEvent::JointEffect {
                id: BOUNCE_SPARK,
                joint: 0,
                offset,
            });
            return;
        }
        self.events.push(crate::ItemEvent::ScriptEffect(
            melee_types::combat::GraphicsCommand {
                bone: 0,
                common_bone: false,
                item_bone: false,
                destroy_on_state_change: false,
                id: BOUNCE_SPARK,
                parameter: 0.0,
                offset,
                range: hsd_types::Vec3::ZERO,
                issued_facing: None,
            },
        ));
    }

    /// it_80275640 (80275640) -> it_80272460: each live hitbox's damage
    /// times `scale`, truncated to its count and restaled for the owner.
    fn scale_hitbox_damage(&mut self, scale: f32) {
        let stale = self.stale_multiplier;
        for hit in self.hitboxes.iter_mut().flatten() {
            hit.knockback_damage = gekko_math::msl::fctiwz(hit.descriptor.damage * scale) as u32;
            hit.descriptor.damage = hit.knockback_damage as f32 * stale;
        }
    }

    pub(crate) fn floor_line_from(&mut self, collision: &CollData, floor: bool) {
        if floor {
            self.floor_line = collision.floor.index;
        }
    }

    /// it_80275DFC (80275DFC): the fixed ECB from the ItemAttr box again.
    pub fn restore_collision_box(&mut self, assets: &ItemAssets) {
        let b = assets.collision_box;
        let scale = self.scale;
        let facing = if self.facing == -1.0 { -1 } else { 1 };
        let collision = self.collision.as_mut().expect("item map collision");
        melee_mp::set_ecb_source_fixed(
            collision,
            b.top * scale,
            b.bottom * scale,
            b.right * scale,
            b.left * scale,
        );
        melee_mp::set_facing_dir(collision, facing);
    }

    /// it_8027429C (8027429C): the holder lets go with `velocity` where
    /// the hand is (it_80273B50), then the hold ends (it_80273F34).
    /// The caller releases the holder's side (Item_8026A848 ->
    /// ftCommon_8007E6DC) once the callback returns.
    pub fn release_from_holder(
        &mut self,
        velocity: hsd_types::Vec3,
        holder: &mut crate::ItemHolder<'_>,
        map: &mut melee_mp::CollMap,
        assets: &ItemAssets,
    ) {
        // it_80273B50: an article (hold kind 8) leaves from its attach
        // joint's offset through the hand, anything else from the hand.
        let hand = holder.part_position();
        let matrix = holder.part_matrix();
        let position = self.drop_release_point(hand, &matrix, assets);
        self.leave_hand(velocity, position, assets);
        self.end_hold(holder.center, holder.attack, map, assets);
        // it_80272460 reads xD88 when a hitbox is made, so one the release
        // callback makes in this same update (a Bob-omb exploding in hand)
        // is staled for the attack just taken over.
        self.stale_multiplier = holder.attack_stale;
    }

    /// it_80273B50 / it_80273748 (80273748): out of the hand at `position`
    /// with `velocity`. The model returns to rest, the spin restarts from
    /// the held angle, and the item faces its motion. Heavy items, which
    /// hang below the hand, are not ported.
    pub fn leave_hand(
        &mut self,
        velocity: hsd_types::Vec3,
        position: hsd_types::Vec3,
        assets: &ItemAssets,
    ) {
        assert!(!assets.heavy, "it_80273748: heavy item hand offset");
        // it_80275070 drops the hand constraint; it_8026B6C8's enemy kinds
        // stay unpickable; it_802756E0 lets hits land again.
        self.grabbable = true;
        self.hurt_intangible = false;
        // it_80274990 reads the spin axis before lb_8000B804 resets the pose.
        let rotation = self.ecb_angle();
        self.rotation = crate::engine::rest_rotation(assets);
        self.model_scale = hsd_types::Vec3::new(self.scale, self.scale, self.scale);
        self.update_spin(rotation);
        // retail 80273C3C..54: three fmuls by ItemAttr x4.
        let multiplier = assets.throw_speed_multiplier;
        self.velocity = hsd_types::Vec3::new(
            velocity.x * multiplier,
            velocity.y * multiplier,
            velocity.z * multiplier,
        );
        // A NaN speed counts as moving, as the retail compare falls through.
        let still = gekko_math::msl::fabsf(self.velocity.x) < 0.00001;
        if !still || self.facing == 0.0 {
            self.facing = if self.velocity.x >= 0.0 { 1.0 } else { -1.0 };
        }
        self.face_spin_axis();
        self.position = hsd_types::Vec3::new(position.x, position.y, 0.0);
        self.root_translation = self.position;
    }

    /// it_80273748's release point for a throw from `position`: an article
    /// (hold kind 8) hangs from the hand by its attach joint, so the offset
    /// of that joint's negated translation through `hand` from the hand
    /// itself is added (fsubs, then fadds). Other light items leave at
    /// `position`.
    pub fn throw_release_point(
        &self,
        position: hsd_types::Vec3,
        hand: &hsd_types::Mtx,
        assets: &ItemAssets,
    ) -> hsd_types::Vec3 {
        if self.hold_kind != ARTICLE_HOLD_KIND {
            return position;
        }
        let hung = self.hung_from(hand, assets);
        let at = hsd_types::Vec3::new(hand.0[0][3], hand.0[1][3], hand.0[2][3]);
        hsd_types::Vec3::new(
            position.x + (hung.x - at.x),
            position.y + (hung.y - at.y),
            0.0,
        )
    }

    /// it_80273B50's release point for a drop at `hand`: an article's
    /// attach joint offset through the hand, else the hand itself.
    pub fn drop_release_point(
        &self,
        position: hsd_types::Vec3,
        hand: &hsd_types::Mtx,
        assets: &ItemAssets,
    ) -> hsd_types::Vec3 {
        if self.hold_kind != ARTICLE_HOLD_KIND {
            return position;
        }
        self.hung_from(hand, assets)
    }

    /// lb_8000B1CC(hand, -it_80272C90's translation).
    fn hung_from(&self, hand: &hsd_types::Mtx, assets: &ItemAssets) -> hsd_types::Vec3 {
        let t = assets.attach_translation();
        let offset = hsd_types::Vec3::new(-t.x, -t.y, -t.z);
        let mut hung = hsd_types::Vec3::ZERO;
        hsd_anim::mtx::mtx_mult_vec(hand, &offset, &mut hung);
        hung
    }

    /// it_80273F34 (80273F34): the hold ends; a sweep from the holder's body
    /// (`center`) to the release point keeps the item out of walls, and the
    /// holder's attack becomes the item's (it_8027B070).
    pub fn end_hold(
        &mut self,
        center: hsd_types::Vec3,
        attack: Option<melee_types::combat::AttackInstance>,
        map: &mut melee_mp::CollMap,
        assets: &ItemAssets,
    ) {
        self.held = false;
        self.holder_part = 0;
        self.speed_damage = true;
        self.throw_count += 1;
        self.land_count = 0;
        self.enter_air();
        self.sweep_from_holder(center, map, assets);
        self.face_spin_axis();
        self.stale_source = attack;
        self.enter_air();
        // it_80273F34: HSD_JObjSetTranslate(jobj, &pos) after the sweep.
        self.root_translation = self.position;
    }

    /// xDC8 x19 (HSD_JObjSetRotationY): a facing-locked model turns to face
    /// its direction, (float) (M_PI_2 * facing) in double precision.
    pub(crate) fn face_spin_axis(&mut self) {
        if self.spin_ignores_facing {
            self.rotation.y = (std::f64::consts::FRAC_PI_2 * f64::from(self.facing)) as f32;
        }
    }

    /// it_80275BC8 (80275BC8): grow the saved ECB box by the common release
    /// scale (it_80275D5C, as xDCE b7 is set at creation), then an airborne
    /// pass from the holder's centre to the hand (it_80276100).
    fn sweep_from_holder(
        &mut self,
        center: hsd_types::Vec3,
        map: &mut melee_mp::CollMap,
        assets: &ItemAssets,
    ) {
        let box_scale = assets.release_box_scale;
        let b = assets.collision_box;
        let (top, bottom, right, left) = (
            b.top * box_scale,
            b.bottom * box_scale,
            b.right * box_scale,
            b.left * box_scale,
        );
        let scale = self.scale;
        let facing = if self.facing == -1.0 { -1 } else { 1 };
        let mut collision = self.collision.take().expect("item map collision");
        collision.cur_pos = self.position;
        melee_mp::set_ecb_source_fixed(
            &mut collision,
            top * scale,
            bottom * scale,
            right * scale,
            left * scale,
        );
        melee_mp::set_facing_dir(&mut collision, facing);
        collision.last_pos = center;
        melee_mp::mark_ecb_clear(&mut collision);
        map.air_collide_pass(&mut collision, None);
        self.position = collision.cur_pos;
        self.collision = Some(collision);
    }

    /// itColl_BounceOffVictim (80272DB0): the item pops back off what it hit.
    pub fn bounce_off_victim(&mut self, bounce: crate::desc::VictimBounce) {
        // retail 80272DC0: fmuls; 80272DD8: fmadds.
        self.velocity.x *= bounce.horizontal_scale;
        self.velocity.y =
            gekko_math::fma::fmadds(self.velocity.y, bounce.vertical_scale, bounce.vertical_pop);
    }

    /// it_80272980 (80272980): unless the item is all but still (|vel.x| <
    /// 0.00001) with a facing, it faces along its horizontal velocity; the
    /// collision takes the facing (mpCollSetFacingDir).
    pub fn face_velocity(&mut self) {
        let speed = gekko_math::msl::fabsf(self.velocity.x);
        // NaN counts as moving, as fcmpo with bge would.
        let still = speed.partial_cmp(&0.00001) == Some(core::cmp::Ordering::Less);
        if !still || self.facing == 0.0 {
            self.facing = if self.velocity.x >= 0.0 { 1.0 } else { -1.0 };
        }
        let facing = if self.facing == -1.0 { -1 } else { 1 };
        if let Some(collision) = &mut self.collision {
            melee_mp::set_facing_dir(collision, facing);
        }
    }

    /// it_8027770C (8027770C): off a wall the item moves into (a left wall,
    /// then a right one, whose line becomes xC30), the velocity mirrors in
    /// the wall's normal; any wall scales it by ItemAttr x58. Retail
    /// 802777A8..C0: fmuls, then two fmadds for the dot product.
    pub fn bounce_off_wall(&mut self, assets: &ItemAssets) -> bool {
        let collision = self.collision.as_ref().expect("item map collision");
        let env = collision.env_flags as u32;
        let mut normal = None;
        if env & collide::LEFT_WALL_MASK != 0 {
            self.floor_line = collision.left_facing_wall.index;
            normal = Some(collision.left_facing_wall.normal);
        }
        if env & collide::RIGHT_WALL_MASK != 0 {
            self.floor_line = collision.right_facing_wall.index;
            normal = Some(collision.right_facing_wall.normal);
        }
        let Some(normal) = normal else {
            return false;
        };
        let v = self.velocity;
        let dot = gekko_math::fma::fmadds(
            v.z,
            normal.z,
            gekko_math::fma::fmadds(v.x, normal.x, v.y * normal.y),
        );
        if dot < 0.0 {
            self.velocity = melee_lb::vector::mirror(self.velocity, normal);
        }
        let scale = assets.bounce_scale;
        self.velocity.x *= scale;
        self.velocity.y *= scale;
        self.velocity.z *= scale;
        true
    }

    /// mpCollSetFacingDir on the item's CollData.
    pub fn set_collision_facing(&mut self, facing: i32) {
        if let Some(collision) = &mut self.collision {
            melee_mp::set_facing_dir(collision, facing);
        }
    }

    /// it_802762BC (802762BC).
    pub fn enter_air(&mut self) {
        self.ground_or_air = GroundOrAir::Air;
    }
}
