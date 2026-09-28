//! Special entry boundary shared by the existing attack-input paths.
use crate::fighter::{assets::FighterAssets, Fighter, SpecialSlot};
use crate::input::{WaitContext, WaitPredicate, WaitTransition};

impl Fighter {
    /// ftCo_SpecialAir_CheckInput (8009665C) as a character IASA's first check:
    /// a B press enters the stick's aerial special.
    pub fn try_air_special(&mut self, assets: &FighterAssets) -> bool {
        if !self.air_special_pressed(assets) {
            return false;
        }
        self.enter_buffered_special(assets, true);
        true
    }

    /// ftCo_SpecialAir_CheckInput's gate: B pressed, and the stick's aerial
    /// special exists (ftData_SpecialAirHi/Lw/S/N[kind] != NULL; Nana has
    /// none for Hi and S). Without one the IASA goes on to its other checks.
    pub fn air_special_pressed(&self, assets: &FighterAssets) -> bool {
        if !self.core.input.pressed.intersects(crate::input::Buttons::B) {
            return false;
        }
        let slot = air_special_slot(self.core.input.current.stick, &assets.input);
        // Fighter capabilities list S/Hi/N/Lw.
        let index = match slot {
            SpecialSlot::Side => 0,
            SpecialSlot::Up => 1,
            SpecialSlot::Neutral => 2,
            SpecialSlot::Down => 3,
        };
        self.core.capabilities.specials[index]
    }

    /// ftCo_SpecialS_CheckInput / ftCo_Attack100_CheckInput / ftCo_800D6824 /
    /// ftCo_800D68C0 consult ftData_SpecialS/Hi/N/Lw[kind], respectively.
    /// In the air, ftCo_SpecialAir_CheckInput reads the stick itself.
    pub(crate) fn enter_buffered_special(&mut self, assets: &FighterAssets, airborne: bool) {
        if airborne {
            let slot = air_special_slot(self.core.input.current.stick, &assets.input);
            self.enter_special(slot, true, assets);
            return;
        }
        let context = WaitContext {
            facing: self.core.physics.facing,
            specials_available: self.core.capabilities.specials,
            shield_health: self.core.status.shield_health,
            ..WaitContext::default()
        };
        const GROUND: [(WaitPredicate, SpecialSlot); 4] = [
            (WaitPredicate::SpecialSide, SpecialSlot::Side),
            (WaitPredicate::SpecialUp, SpecialSlot::Up),
            (WaitPredicate::SpecialNeutral, SpecialSlot::Neutral),
            (WaitPredicate::SpecialDown, SpecialSlot::Down),
        ];
        for (predicate, slot) in GROUND {
            if crate::input::iasa::evaluate(predicate, &self.core.input, &assets.input, &context)
                == WaitTransition::Special(slot)
            {
                self.enter_special(slot, false, assets);
                return;
            }
        }
        unimplemented!("ftCo special input: entry without a supported special buffer");
    }

    /// The entry shared by every special check once its slot is known.
    pub(crate) fn enter_special(
        &mut self,
        slot: SpecialSlot,
        airborne: bool,
        assets: &FighterAssets,
    ) {
        // ftCo_SpecialS_CheckInput / ftCo_SpecialAir_CheckInput:
        // compare the signed stick product, then ftCommon_UpdateFacing.
        if slot == SpecialSlot::Side
            && self.core.input.current.stick.x * self.core.physics.facing
                < -assets.input.special_reverse_threshold
        {
            self.core.physics.facing = if self.core.input.current.stick.x >= 0.0 {
                1.0
            } else {
                -1.0
            };
        }
        if airborne
            && slot == SpecialSlot::Neutral
            && i32::from(self.input.horizontal.since_crossing) < assets.input.neutral_reverse_window
            && (self.physics.facing > 0.0) != self.input.last_horizontal_positive
        {
            // ftCo_SpecialAir_CheckInput: buffered stick direction, fneg.
            self.physics.facing = -self.physics.facing;
        }
        if slot == SpecialSlot::Side && !airborne {
            // ftCo_SpecialS doEnter, 80096614/1C/24: fsubs, fmuls, fmadds.
            let retention = self
                .core
                .attributes
                .specials
                .specials_ground_speed_retention;
            let speed = self.core.physics.ground_velocity;
            let reduction = -(speed * (1.0 - retention));
            let terrain = crate::physics::grounded::floor_friction(&self.core.collision.data);
            self.core.physics.ground_velocity = gekko_math::fma::fmadds(reduction, terrain, speed);
        }
        (self.character.table().enter_special)(self, slot, airborne, assets);
    }
}

/// ftCo_SpecialAir_CheckInput (8009665C): the current stick, inclusive bounds
/// (retail 800966F4 `cror eq,lt,eq`): up at PlCo +21C, down at its negation,
/// sideways at |x| >= PlCo +218, else neutral. Unlike the ground checks it
/// reads no tilt timers, so a stick exactly on a bound still counts.
fn air_special_slot(
    stick: crate::input::pad::Stick,
    input: &crate::input::common::InputCommonData,
) -> SpecialSlot {
    let vertical = input.special_vertical_threshold;
    if stick.y >= vertical {
        SpecialSlot::Up
    } else if stick.y <= -vertical {
        SpecialSlot::Down
    } else if gekko_math::msl::fabsf(stick.x) >= input.special_side_threshold {
        SpecialSlot::Side
    } else {
        SpecialSlot::Neutral
    }
}
