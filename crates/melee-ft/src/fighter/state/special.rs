//! Special entry boundary shared by the existing attack-input paths.
use crate::fighter::{assets::FighterAssets, Fighter, SpecialSlot};
use crate::input::{WaitContext, WaitPredicate, WaitTransition};

impl Fighter {
    /// ftCo_SpecialS_CheckInput / ftCo_Attack100_CheckInput / ftCo_800D6824 /
    /// ftCo_800D68C0 consult ftData_SpecialS/Hi/N/Lw[kind], respectively.
    pub(crate) fn enter_buffered_special(&mut self, assets: &FighterAssets, airborne: bool) {
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
        const AIR: [(WaitPredicate, SpecialSlot); 4] = [
            (WaitPredicate::SpecialUp, SpecialSlot::Up),
            (WaitPredicate::SpecialDown, SpecialSlot::Down),
            (WaitPredicate::SpecialSide, SpecialSlot::Side),
            (WaitPredicate::SpecialNeutral, SpecialSlot::Neutral),
        ];
        for (predicate, slot) in if airborne { AIR } else { GROUND } {
            if crate::input::iasa::evaluate(predicate, &self.core.input, &assets.input, &context)
                == WaitTransition::Special(slot)
            {
                self.enter_special(slot, airborne, assets);
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
