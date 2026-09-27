//! Item map collision: itmaplib.c and itgroundcoll.c. An item that falls and
//! lands keeps its own CollData (Item.x378_itemColl) with a fixed ECB box
//! from its ItemAttr, resolved by the same mpColl passes fighters use.
use crate::{desc::ItemAssets, ItemCore, SpawnItem};
use melee_types::{
    mp::{collide, CollData},
    GroundOrAir, ItemKind,
};

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

/// it_80275E98's ECB category (CollData.x34 b1234) by item kind range.
fn collision_category(kind: ItemKind, hold_kind: u8) -> u8 {
    // Character articles (hold kind 8) are category 5; common items below
    // It_Kind_L_Gun_Ray are category 2. Other ranges are not ported yet.
    match hold_kind {
        8 => 5,
        0 if (kind as u32) < ItemKind::LGunRay as u32 => 2,
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
        melee_mp::set_facing_dir(&mut collision, if self.facing == -1.0 { -1 } else { 1 });
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
    fn refresh_collision(&mut self) -> CollData {
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
        self.collision = Some(collision);
        if contact.ceiling || contact.left_wall || contact.right_wall {
            // it_80276FC4: bounce off walls and ceilings.
            unimplemented!("it_80276FC4: item wall/ceiling bounce");
        }
        if floor {
            self.restore_collision_box(assets);
            self.ground_or_air = GroundOrAir::Ground;
        }
        contact
    }

    fn floor_line_from(&mut self, collision: &CollData, floor: bool) {
        if floor {
            self.floor_line = collision.floor.index;
        }
    }

    /// it_80275DFC (80275DFC): the fixed ECB from the ItemAttr box again.
    fn restore_collision_box(&mut self, assets: &ItemAssets) {
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

    /// it_8027429C (8027429C): the holder lets go with `velocity`.
    /// it_80273B50 places the item at the hand and aims it; it_80273F34 ends
    /// the hold and sweeps the map from the holder's body to the hand
    /// (it_80275BC8). The caller releases the holder's side
    /// (Item_8026A848 -> ftCommon_8007E6DC) once the callback returns.
    /// Heavy items, which hang below the hand, are not ported.
    pub fn release_from_holder(
        &mut self,
        velocity: hsd_types::Vec3,
        holder: &mut crate::ItemHolder<'_>,
        map: &mut melee_mp::CollMap,
        assets: &ItemAssets,
    ) {
        assert!(!assets.heavy, "it_80273B50: heavy item hand offset");
        // it_80273B50. it_80275070 drops the hand constraint; it_8026B6C8's
        // enemy kinds stay unpickable.
        self.grabbable = true;
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
        let hand = holder.part_position();
        self.position = hsd_types::Vec3::new(hand.x, hand.y, 0.0);
        // it_80273F34.
        self.held = false;
        self.holder_part = 0;
        self.enter_air();
        self.sweep_from_holder(holder.center, map, assets);
        self.face_spin_axis();
        // it_8027B070: the holder's attack becomes the item's.
        self.stale_source = holder.attack;
        self.enter_air();
    }

    /// xDC8 x19 (HSD_JObjSetRotationY): a facing-locked model turns to face
    /// its direction, (float) (M_PI_2 * facing) in double precision.
    fn face_spin_axis(&mut self) {
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

    /// it_802762BC (802762BC).
    pub fn enter_air(&mut self) {
        self.ground_or_air = GroundOrAir::Air;
    }
}
