//! efSync_Spawn (8005FDDC), efsync.c:228-292, request 0x4CF.
use super::*;
// efLib_Create, eflib.c:524-528: 32 queued initial animations.
const ANIMATION_QUEUE_CAPACITY: usize = 32;
// efsync.c:228-292 emits twelve shell fragments per burst.
const SHELL_COUNT: usize = 12;
/// efSync_Spawn draws each fragment's model and two angles as it creates
/// it (0x80060848, 0x800608D4, 0x800608E0).
pub(crate) const CREATION_DRAWS: usize = 3 * SHELL_COUNT;
impl Effects {
    /// `owner` is efSync_Spawn's gobj, each fragment's parent
    /// (efLib_Create_Attach_Pos, efsync.c:246-252): efLib_DestroyAll on it
    /// frees the fragments still out, a fighter's at a motion change that
    /// tears its effects down, a thrown egg's when the egg is destroyed.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn spawn_egg_shell<T: InverseTrig>(
        &mut self,
        bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
        matrix: &Mtx,
        scale: f32,
        seed: Option<u32>,
        owner: ModelOwner,
    ) -> Result<()> {
        self.make_room(0x1E, SHELL_COUNT, particles);
        // The creation draws come from where retail made them when the
        // requester reserved them; the animations draw from the stream.
        let mut reserved = HsdRng {
            seed: seed.unwrap_or_default(),
        };
        let mut shells = FixedVec::<_, ANIMATION_QUEUE_CAPACITY>::default();
        for _ in 0..SHELL_COUNT {
            let creation = if seed.is_some() {
                &mut reserved
            } else {
                &mut *rng
            };
            let id = if creation.randf() < 0.5 { 0x1E } else { 0x1F };
            let y = (std::f64::consts::TAU * f64::from(creation.randf())) as f32;
            let x = (std::f64::consts::TAU * f64::from(creation.randf())) as f32;
            let mut effect = self.acquire(id, particles);
            effect.joint_base = FIRST_EFFECT_JOINT + self.next_joint;
            self.next_joint += effect.tree.len();
            effect.lifetime = 50;
            effect.indefinite = false;
            effect.owner = Some(owner);
            // Retail efSync_Spawn: double M_TAU products, then frsp; no FMA.
            effect
                .tree
                .set_scale(effect.root, &Vec3::new(scale, scale, scale));
            effect.tree.set_rotation_y(effect.root, y);
            effect.tree.set_rotation_x(effect.root, x);
            effect.tree.set_translate(
                effect.root,
                &Vec3::new(matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]),
            );
            // efSync_Spawn's separately rounded fmuls; MSL routines are audited.
            use gekko_math::msl::{cosf, sinf};
            effect.velocity = Some(Vec3::new(
                (2.0 * cosf(x)) * sinf(y),
                2.0 * sinf(x),
                (2.0 * cosf(x)) * cosf(y),
            ));
            shells.push(effect);
        }
        // efLib_AnimQueue is drained in reverse creation order by efSync_Spawn.
        for effect in shells.iter_mut().rev() {
            effect.animate::<T>(
                bank,
                particles,
                rng,
                &mut self.draws,
                &mut self.events,
                &mut self.animation_events,
            )?;
        }
        while !shells.is_empty() {
            self.instances.push(shells.remove(0));
        }
        Ok(())
    }
}

/// Requests whose dispatch is efLib_CreateGenerator(generator, &pos): a
/// character-bank generator at a world position.
const ROOT_GENERATORS: [(u16, u32); 5] = [
    // efSync_Spawn 0x4CE (efsync.c:225): Yoshi's egg sparkle.
    (0x4CE, 0x2328),
    // efAlt_Spawn 0x47B (efalt.c:58-60): Mario's fireball bounce.
    (0x47B, 0x3EB),
    // efSync_Spawn 0x508 (efsync.c:581-583): Luigi's fireball bounce.
    (0x508, 0x4652),
    // efAlt_Spawn 0x4A0 (efalt.c:346-348): Kirby's copied fireball bounce.
    (0x4A0, 0x7D02),
    // efAlt_Spawn 0x4B2 (efalt.c:486-488): Kirby's copied green fireball
    // bounce.
    (0x4B2, 0x908A),
];
/// efSync_Spawn 0x4CF (efsync.c:228): the shell burst.
const EGG_SHELL_REQUEST: u16 = 0x4CF;

impl Effects {
    /// efAsync_QueueProcessDeferred, EF_SPAWN_POS (kind 1) and
    /// EF_SPAWN_POS_PARAM (kind 4) on an item's model root: the request's
    /// effect at the root's world translation (lb_8000B1CC(jobj, NULL)),
    /// with kind 4's float parameter. The Egg Throw burst's two requests
    /// (it_802B2C38) and the fireball's bounce (8029B9F4) are ported.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn_item_root<T: InverseTrig>(
        &mut self,
        item: u32,
        id: u16,
        position: Vec3,
        parameter: Option<f32>,
        common_bank: &ParticleBank,
        particles: &mut ParticleSystem,
        rng: &mut HsdRng,
    ) -> Result<()> {
        let generator = ROOT_GENERATORS
            .iter()
            .find(|&&(request, _)| request == id)
            .map(|&(_, generator)| generator);
        match (id, parameter, generator) {
            (_, None, Some(generator)) => {
                let mut request = SpawnRequest::new((generator / 1000) as u8, generator, 0);
                request.position = [position.x, position.y, position.z];
                let bank =
                    resources::character_bank(&self.character_banks, (generator / 1000) as i32)?;
                self.events.spawn(&request, false, false);
                spawn_particle::<T>(particles, bank, request, rng, &mut self.draws)?;
                Ok(())
            }
            (EGG_SHELL_REQUEST, Some(scale), None) => {
                let mut matrix = Mtx::default();
                matrix.0[0][3] = position.x;
                matrix.0[1][3] = position.y;
                matrix.0[2][3] = position.z;
                let owner = ModelOwner::Item(item);
                self.spawn_egg_shell::<T>(common_bank, particles, rng, &matrix, scale, None, owner)
            }
            _ => anyhow::bail!("efAsync item root effect {id:#x} ({parameter:?})"),
        }
    }
}
