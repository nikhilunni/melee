//! Kirby's efAlt rows (efalt.c:212-323), bank 5 (EfKbData.dat).
use super::*;

/// efAlt_Spawn 0x495's model: the dash attack's flame.
const DASH_ATTACK_MODEL: u32 = 0x138B;
/// efAlt_Spawn 0x49D's model: one fist of the rapid jab.
const RAPID_JAB_MODEL: u32 = 0x1390;
/// efAlt 0x49D starts the model's child this far behind the root's own Z
/// (efalt.c:308-309) and efLib_Cb_SetJObjOffsetZ moves it this far forward
/// on every update (eflib.c:1138).
const RAPID_JAB_CHILD_START: f32 = 6.0;
const RAPID_JAB_CHILD_STEP: f32 = 2.0;
/// efAlt_Spawn 0x49A's generator, in Kirby's bank: the Inhale's wind.
const INHALE_WIND_GENERATOR: u32 = 0x138B;
/// efAlt_Spawn 0x498 / 0x499's models: the hammer article's own effect,
/// swung on the ground and in the air.
const HAMMER_MODELS: [u32; 2] = [0x138E, 0x138F];
const HAMMER_GROUND_REQUEST: u16 = 0x498;
const KIRBY_BANK: u8 = 5;
/// efAlt_Spawn 0x494's models: Final Cutter's blade, two on the sword
/// joint and one on the second joint the caller names.
const FINAL_CUTTER_MODELS: [u32; 3] = [0x1388, 0x1389, 0x138A];
/// efAlt 0x494 overwrites each model's lifetime (efalt.c:223).
const BLADE_FRAMES: u16 = 0x41;
/// efLib_Cb_SetRotYAndTransition: with one frame of life left the blade
/// replays from frame 60 for six more (eflib.c:1120-1123); once marked to
/// expire it plays out from frame 65 for eleven (eflib.c:1114-1118).
pub(super) const BLADE_LOOP_FRAMES: u16 = 6;
pub(super) const BLADE_LOOP_START: f32 = 60.0;
pub(super) const BLADE_END_FRAMES: u16 = 0xB;
pub(super) const BLADE_END_START: f32 = 65.0;

/// An f64 +-M_PI_2 by the facing's sign, rounded on the store
/// (eflib.c:1106-1112).
pub(super) fn quarter_turn(facing: f32) -> f32 {
    if facing < 0.0 {
        -std::f64::consts::FRAC_PI_2 as f32
    } else {
        std::f64::consts::FRAC_PI_2 as f32
    }
}

impl Effects {
    /// efAlt_Spawn 0x495 (efalt.c:253-270), from the dash attack's script
    /// (ftCo_8009F834 block_64: efAsync kind 3 with the fighter's facing):
    /// efLib_Create_Attach_Scale(0x138B) on the joint, its root turned to
    /// the facing the script passed (an f64 +-M_PI_2, rounded on the store).
    #[allow(clippy::too_many_arguments)]
    pub(super) fn spawn_kirby_dash_attack<T: InverseTrig>(
        &mut self,
        player: usize,
        bone: usize,
        matrix: Mtx,
        facing: f32,
        fighter: &mut impl EffectOwner,
        common_bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let mut effect = self.acquire(DASH_ATTACK_MODEL, particles);
        effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
        self.next_joint += effect.tree.len();
        effect.owner = Some(ModelOwner::Fighter(player));
        // efAlt's attached character models keep state_flags 0: held
        // through hitlag by efLib_PauseAll.
        effect.hitlag_pause = HitlagPause::Active;
        effect.attachment = Some(player);
        effect.attachment_bone = Some(bone);
        effect.scale_attachment = false;
        // efLib_Create_Attach_Scale: the fighter root's Y scale, uniformly.
        let scale = fighter.effect_scale().y;
        effect
            .tree
            .set_scale(effect.root, &Vec3::new(scale, scale, scale));
        effect.tree.set_rotation_y(
            effect.root,
            if facing < 0.0 {
                -std::f64::consts::FRAC_PI_2 as f32
            } else {
                std::f64::consts::FRAC_PI_2 as f32
            },
        );
        effect.tree.set_translate(
            effect.root,
            &Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
        );
        effect.animate_banks::<T>(
            resources::Banks {
                common: common_bank,
                characters: &self.character_banks,
            },
            particles,
            rng,
            &mut self.draws,
            &mut self.events,
            &mut self.animation_events,
        )?;
        self.instances.push(effect);
        Ok(())
    }

    /// efAlt_Spawn 0x49D (efalt.c:296-319), from the rapid jab's script
    /// (ftCo_8009F834 block_65: efAsync kind 6 with the command's offset,
    /// the facing and an angle): efLib_Create_Attach_Pos(0x1390) at the
    /// point, the fighter root's Y scale uniformly, the child's Z set from
    /// the root's (fsubs), the root turned to the facing (an f64 +-M_PI_2,
    /// rounded) and tilted about Z by the angle, negated facing left;
    /// efLib_Cb_SetJObjOffsetZ is its update callback.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn spawn_kirby_rapid_jab<T: InverseTrig>(
        &mut self,
        player: usize,
        position: Vec3,
        facing: f32,
        angle: f32,
        fighter: &mut impl EffectOwner,
        common_bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let mut effect = self.acquire(RAPID_JAB_MODEL, particles);
        effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
        self.next_joint += effect.tree.len();
        effect.owner = Some(ModelOwner::Fighter(player));
        effect.hitlag_pause = HitlagPause::Active;
        effect.attachment = None;
        effect.tree.set_translate(effect.root, &position);
        let scale = fighter.effect_scale().y;
        effect
            .tree
            .set_scale(effect.root, &Vec3::new(scale, scale, scale));
        let child = effect
            .tree
            .child(effect.root)
            .expect("rapid jab fist joint");
        effect
            .tree
            .set_translate_z(child, position.z - RAPID_JAB_CHILD_START);
        let (turn, tilt) = if facing < 0.0 {
            (-std::f64::consts::FRAC_PI_2 as f32, -angle)
        } else {
            (std::f64::consts::FRAC_PI_2 as f32, angle)
        };
        effect.tree.set_rotation_y(effect.root, turn);
        effect.tree.set_rotation_z(effect.root, tilt);
        effect.child_drift_z = Some(RAPID_JAB_CHILD_STEP);
        effect.animate_banks::<T>(
            resources::Banks {
                common: common_bank,
                characters: &self.character_banks,
            },
            particles,
            rng,
            &mut self.draws,
            &mut self.events,
            &mut self.animation_events,
        )?;
        self.instances.push(effect);
        Ok(())
    }

    /// efAlt_Spawn 0x494 (efalt.c:213-252), from Final Cutter's entries:
    /// efLib_Create_AttachChild(0x1388) and efLib_Create_Attach(0x1389) on
    /// the sword joint and efLib_Create_Attach(0x138A) on the second joint,
    /// each with 0x41 frames of life and efLib_Cb_SetRotYAndTransition as
    /// its update callback. The first has no user data; the other two carry
    /// the fighter, whose facing turns their roots on every update (the
    /// callback reads Fighter +2C through HSD_JObj.scale.x).
    /// TODO(orientation): AttachChild's lb_8000C290 orientation constraint
    /// is not applied (drawn pose only).
    pub(super) fn spawn_kirby_final_cutter<T: InverseTrig>(
        &mut self,
        player: usize,
        bones: [usize; 2],
        fighter: &mut impl EffectOwner,
        common_bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let mut blade = FINAL_CUTTER_MODELS.map(|model| {
            let mut effect = self.acquire(model, particles);
            effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
            self.next_joint += effect.tree.len();
            effect
        });
        let facing = fighter.effect_facing();
        for (index, effect) in blade.iter_mut().enumerate() {
            let bone = bones[usize::from(index == 2)];
            effect.owner = Some(ModelOwner::Fighter(player));
            effect.hitlag_pause = HitlagPause::Active;
            effect.attachment = Some(player);
            effect.attachment_bone = Some(bone);
            effect.scale_attachment = false;
            effect.lifetime = BLADE_FRAMES;
            effect.indefinite = false;
            effect.blade_transition = true;
            if index != 0 {
                effect.facing_rotation = Some(quarter_turn(facing));
            }
            let matrix = fighter.effect_matrix(Some(bone));
            effect.tree.set_translate(
                effect.root,
                &Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
            );
        }
        // efAlt_Spawn drains efLib_AnimQueue newest first (efalt.c:534-542).
        for effect in blade.iter_mut().rev() {
            effect.animate_banks::<T>(
                resources::Banks {
                    common: common_bank,
                    characters: &self.character_banks,
                },
                particles,
                rng,
                &mut self.draws,
                &mut self.events,
                &mut self.animation_events,
            )?;
        }
        for effect in blade {
            self.instances.push(effect);
        }
        Ok(())
    }

    /// efAlt_Spawn 0x498 / 0x499 (efalt.c:281-288), from the hammer
    /// article's creation (itkirbyhammer.c setupHammerParticles: efAsync
    /// kind 0 on the model's attach joint): efLib_Create_AttachChild(0x138E
    /// / 0x138F) with the item as the parent, on that joint.
    /// TODO(orientation): AttachChild's lb_8000C290 orientation constraint
    /// is not applied (drawn pose only).
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_kirby_hammer_model<T: InverseTrig>(
        &mut self,
        id: u16,
        item: u32,
        bone: usize,
        matrix: Mtx,
        common_bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let model = HAMMER_MODELS[usize::from(id != HAMMER_GROUND_REQUEST)];
        let mut effect = self.acquire(model, particles);
        effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
        self.next_joint += effect.tree.len();
        // efLib_Create's parent_gobj: Item_8026A8EC's efLib_DestroyAll
        // takes the model with the hammer.
        effect.owner = Some(ModelOwner::Item(item));
        effect.attachment = None;
        effect.item_bone = Some((item, bone as u8));
        effect.tree.set_translate(
            effect.root,
            &Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
        );
        effect.animate_banks::<T>(
            resources::Banks {
                common: common_bank,
                characters: &self.character_banks,
            },
            particles,
            rng,
            &mut self.draws,
            &mut self.events,
            &mut self.animation_events,
        )?;
        self.instances.push(effect);
        Ok(())
    }

    /// The item bones models are attached to, as (item, bone).
    pub fn item_bone_models(&self) -> impl Iterator<Item = (u32, usize)> + '_ {
        self.instances
            .iter()
            .filter_map(|effect| effect.item_bone)
            .map(|(item, bone)| (item, usize::from(bone)))
    }

    /// lb_8000C1C0's position constraint for a model on an item's bone: the
    /// bone's world translation, read before this update's animation.
    pub fn place_item_bone_models(&mut self, item: u32, bone: usize, matrix: &Mtx) {
        let position = Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]);
        for effect in self
            .instances
            .iter_mut()
            .filter(|effect| effect.item_bone == Some((item, bone as u8)))
        {
            effect.tree.set_translate(effect.root, &position);
        }
    }

    /// efAlt_Spawn 0x49A (efalt.c:289-291), from the Inhale's start rows
    /// (efAsync kind 3 with the facing):
    /// efLib_CreateGenerator_AppSRT_SetFacingDir(0x138B) on the joint, the
    /// generator's own AppSRT turned to the facing (eflib.c:822-835).
    pub(super) fn spawn_kirby_inhale_wind<T: InverseTrig>(
        &mut self,
        player: usize,
        bone: usize,
        matrix: Mtx,
        facing: f32,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let joint_id = FIRST_FIGHTER_JOINT + player * FIGHTER_JOINT_STRIDE + bone;
        let mut spawn = SpawnRequest::new(KIRBY_BANK, INHALE_WIND_GENERATOR, 0);
        spawn.joint = Some((joint_id, matrix));
        spawn.application_transform = Some(hsd_particle::generator::ApplicationTransform {
            rotation: Vec3::new(
                0.0,
                if facing < 0.0 {
                    -std::f32::consts::FRAC_PI_2
                } else {
                    std::f32::consts::FRAC_PI_2
                },
                0.0,
            ),
            ..Default::default()
        });
        self.events.spawn(&spawn, false, false);
        let bank = resources::character_bank(&self.character_banks, KIRBY_BANK.into())?;
        if let Some(id) = spawn_particle::<T>(particles, bank, spawn, rng, &mut self.draws)? {
            let generator = particles.generator_mut(id).unwrap();
            self.events.flags(joint_id, 0x600, 0x800);
            generator.flags = (generator.flags & !0x600) | 0x800;
        }
        self.fighter_joints[player * FIGHTER_JOINT_STRIDE + bone] = true;
        Ok(())
    }
}
