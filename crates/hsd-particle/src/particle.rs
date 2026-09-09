//! Particle creation and bytecode simulation (`particle.c`).
use crate::{
    bank::Descriptor,
    color::ColorTrack,
    program::Cursor,
    rng_sites::{DrawLog, PRIMARY_COLOR},
    Error,
};
use gekko_math::{
    fma::{fmadds, fmsubs},
    msl::fctiwz,
    rng::HsdRng,
};
use std::sync::Arc;

/// Kind flags used by simulation; all other retail bits are retained.
pub const PAUSED: u32 = 0x800;
pub const GRAVITY: u32 = 1;
pub const FRICTION: u32 = 2;
pub const TORNADO: u32 = 4;
pub const TEXTURED: u32 = 0x400;

/// Owned `HSD_Particle` state (psstructs.h, retail allocation 0x80398C04).
#[derive(Debug, Clone)]
pub struct Particle {
    pub generator_id: Option<usize>,
    pub family_id: u16,
    pub bank: u8,
    pub link: u8,
    pub kind: u32,
    pub texture_group: u8,
    pub pose: u8,
    pub palette: u8,
    pub program: Arc<[u8]>,
    pub pc: u16,
    pub mark: u16,
    pub loop_start: u16,
    pub loop_count: u8,
    pub wait: u16,
    pub life: u16,
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub gravity: f32,
    pub friction: f32,
    pub size: f32,
    pub size_target: f32,
    pub size_timer: u16,
    pub rotation: f32,
    pub rotation_target: f32,
    pub rotation_timer: u16,
    pub primary: ColorTrack,
    pub environment: ColorTrack,
    pub trail: f32,
    /// HSD_Particle.aCmpCount/Remain/Param1/Param2/targets (+0x54,78,57,58,7A).
    pub alpha_compare: BytePairTrack,
    /// HSD_Particle.aCmpMode (+0x56).
    pub alpha_compare_mode: u8,
    /// HSD_Particle.pJObjOfs (+0x59).
    pub point_joint_offset: u8,
    /// HSD_Particle.matColCount/Remain/RGB/A/targets (+0x5A,74,7C,7D,80).
    pub material: BytePairTrack,
    /// HSD_Particle.ambColCount/Remain/RGB/A/targets (+0x5C,76,7E,7F,82).
    pub ambient: BytePairTrack,
    /// HSD_Particle.appsrt (+0x8C), normalized owned AppSRT index.
    pub appsrt_id: Option<usize>,
    /// Availability from the owning texture group; no image bytes needed.
    pub texture_images: Arc<[bool]>,
}
impl Particle {
    /// `psGenerateParticle0` (particle.c, 0x80398C04). The caller inserts
    /// at the list head *before* the optional immediate interpretation.
    pub fn new(descriptor: &Descriptor, bank: u8, link: u8) -> Result<Self, Error> {
        if link >= 16 {
            return Err(Error::InvalidLink(link));
        }
        Ok(Self {
            generator_id: None,
            family_id: 0,
            bank,
            link,
            kind: descriptor.kind,
            texture_group: descriptor.texture_group as u8,
            pose: 0,
            palette: 255,
            program: descriptor.program.clone().into(),
            pc: 0,
            mark: 0,
            loop_start: 0,
            loop_count: 0,
            wait: u16::from(!descriptor.program.is_empty()),
            life: descriptor.particle_life.wrapping_add(1),
            position: [0.0; 3],
            velocity: descriptor.velocity,
            gravity: descriptor.gravity,
            friction: descriptor.friction,
            size: descriptor.size,
            size_target: 0.0,
            size_timer: 0,
            rotation: 0.0,
            rotation_target: 0.0,
            rotation_timer: 0,
            primary: ColorTrack::new([255; 4]),
            environment: ColorTrack::new([0; 4]),
            trail: 1.0,
            // particle.c:461-483; retail 0x80398E18..0x80398E7C: stb/sth.
            alpha_compare: BytePairTrack::new([1, 255]),
            alpha_compare_mode: 0x33,
            point_joint_offset: 0,
            material: BytePairTrack::new([255; 2]),
            ambient: BytePairTrack::new([255; 2]),
            appsrt_id: None,
            texture_images: Arc::from([]),
        })
    }

    /// `hsd_8039930C` (particle.c, 0x8039930C). Returns false on deletion.
    /// An error terminates the simulation; callers must not continue a partial
    /// tick after unsupported bytecode or corrupt data.
    pub fn update(&mut self, rng: &mut HsdRng, draws: &mut DrawLog) -> Result<bool, Error> {
        if self.kind & PAUSED != 0 {
            return Ok(true);
        }
        if self.kind & TORNADO != 0 {
            return Err(Error::UnsupportedFeature("tornado particle physics"));
        }
        if self.kind & 0x8000 != 0 {
            return Err(Error::UnsupportedFeature("particle JObj attachment"));
        }
        self.interpolate();
        if self.wait != 0 {
            self.wait -= 1;
            if self.wait == 0 {
                self.interpret(rng, draws)?;
            }
        }
        self.life = self.life.wrapping_sub(1);
        if self.life == 0 {
            return Ok(false);
        }
        self.integrate();
        Ok(true)
    }

    fn interpolate(&mut self) {
        // retail 0x803993B0..0x803993BC and 0x80399544..0x80399550:
        // fsubs/fdivs/fadds, no contraction.
        if self.size_timer != 0 {
            self.size += (self.size_target - self.size) / f32::from(self.size_timer);
            self.size_timer -= 1;
        }
        self.primary.tick();
        self.environment.tick();
        self.material.tick();
        self.ambient.tick();
        self.alpha_compare.tick();
        if self.rotation_timer != 0 {
            self.rotation +=
                (self.rotation_target - self.rotation) / f32::from(self.rotation_timer);
            self.rotation_timer -= 1;
        }
    }

    fn integrate(&mut self) {
        // retail 0x8039CB90..0x8039CC1C: separate subtract, multiply, add.
        if self.kind & GRAVITY != 0 {
            self.velocity[1] -= self.gravity;
        }
        if self.kind & FRICTION != 0 {
            for value in &mut self.velocity {
                *value *= self.friction;
            }
        }
        for (position, velocity) in self.position.iter_mut().zip(self.velocity) {
            *position += velocity;
        }
    }

    fn interpret(&mut self, rng: &mut HsdRng, draws: &mut DrawLog) -> Result<(), Error> {
        let program = Arc::clone(&self.program);
        let mut cursor = Cursor {
            bytes: &program,
            pc: self.pc,
        };
        // Valid programs yield at a wait or end. Bound malformed zero-time
        // loops explicitly instead of hanging the host or silently yielding.
        for _ in 0..65_536 {
            let opcode_pc = cursor.pc;
            let opcode = cursor.byte()?;
            if opcode < 0x80 {
                let mut wait = u16::from(opcode & 0x1f);
                if opcode & 0x20 != 0 {
                    wait = (wait << 8) | u16::from(cursor.byte()?);
                }
                if opcode & 0x40 != 0 {
                    self.pose = cursor.byte()?;
                    self.enable_texture();
                }
                if wait != 0 {
                    self.pc = cursor.pc;
                    self.wait = wait;
                    return Ok(());
                }
            } else if opcode < 0xa0 {
                self.vector_command(opcode, &mut cursor)?;
            } else if opcode & 0xf0 == 0xc0 || opcode & 0xf0 == 0xd0 {
                let track = if opcode & 0xf0 == 0xc0 {
                    &mut self.primary
                } else {
                    &mut self.environment
                };
                track.setup(opcode & 15, &mut cursor)?;
            } else {
                match opcode {
                    0xfa => {
                        self.loop_count = cursor.byte()?;
                        self.loop_start = cursor.pc;
                    }
                    0xfb => {
                        self.loop_count = self.loop_count.wrapping_sub(1);
                        if self.loop_count != 0 {
                            cursor.pc = self.loop_start;
                        }
                    }
                    0xfc => self.mark = cursor.pc,
                    0xfd => cursor.pc = self.mark,
                    0xfe | 0xff => {
                        self.life = 1;
                        self.pc = cursor.pc;
                        self.wait = 0;
                        return Ok(());
                    }
                    0xa7 => {
                        let threshold = cursor.byte()?;
                        if i32::from(threshold) >= fctiwz(100.0 * draws.draw(rng, 0x8039_A430)) {
                            self.life = 1;
                            self.pc = cursor.pc;
                            self.wait = 0;
                            return Ok(());
                        }
                    }
                    _ => self.command(opcode, opcode_pc, &mut cursor, rng, draws)?,
                }
            }
        }
        Err(Error::InstructionLimit { pc: cursor.pc })
    }

    fn enable_texture(&mut self) {
        if self
            .texture_images
            .get(usize::from(self.pose))
            .copied()
            .unwrap_or(false)
        {
            self.kind |= TEXTURED;
        }
    }

    fn vector_command(&mut self, opcode: u8, pc: &mut Cursor<'_>) -> Result<(), Error> {
        let vector = if opcode < 0x90 {
            &mut self.position
        } else {
            &mut self.velocity
        };
        for (axis, value) in vector.iter_mut().enumerate() {
            if opcode & (1 << axis) != 0 {
                let operand = pc.float()?;
                if opcode & 8 != 0 {
                    *value += operand;
                } else {
                    *value = operand;
                }
            }
        }
        Ok(())
    }

    fn command(
        &mut self,
        opcode: u8,
        opcode_pc: u16,
        pc: &mut Cursor<'_>,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> Result<(), Error> {
        match opcode {
            0xa0 | 0xac => {
                self.size_timer = pc.timer()?;
                self.size_target = pc.float()?;
                if opcode == 0xac {
                    let range = pc.float()?;
                    // retail 0x8039A818: fmadds
                    self.size_target =
                        fmadds(range, draws.draw(rng, 0x8039_A810), self.size_target);
                }
                if self.size_timer == 0 {
                    self.size = self.size_target;
                }
            }
            0xa1 => self.kind &= !TEXTURED,
            0xa2 => {
                self.gravity = pc.float()?;
                self.set_flag(GRAVITY, self.gravity != 0.0);
            }
            0xa3 => {
                self.friction = pc.float()?;
                self.set_flag(FRICTION, self.friction != 1.0);
            }
            0xa6 => {
                let base = pc.short()?;
                let range = pc.short()?;
                self.life = base
                    .wrapping_add(fctiwz(f32::from(range) * draws.draw(rng, 0x8039_A3F4)) as u16);
            }
            0xa8 => {
                for (axis, site) in [0x8039_A490, 0x8039_A4D4, 0x8039_A51C]
                    .into_iter()
                    .enumerate()
                {
                    let range = pc.float()?;
                    // retail 0x8039A4A4, 0x8039A4EC, 0x8039A528: fmsubs;
                    // subsequent position addition remains separate.
                    self.position[axis] += fmsubs(2.0 * range, draws.draw(rng, site), range);
                }
            }
            0xab => {
                let scale = pc.float()?;
                for value in &mut self.velocity {
                    *value *= scale;
                }
            }
            0xad => self.kind |= 0x80,
            0xae => self.kind &= !0x60,
            0xaf => self.kind = (self.kind & !0x40) | 0x20,
            0xb0 => self.kind = (self.kind & !0x20) | 0x40,
            0xb1 => self.kind |= 0x60,
            0xba => self.primary.random_delta(pc, rng, draws, PRIMARY_COLOR)?,
            0xbb => self.environment.random_delta(
                pc,
                rng,
                draws,
                [0x8039_B2F0, 0x8039_B360, 0x8039_B3D4, 0x8039_B448],
            )?,
            0xbc => {
                let base = pc.byte()?;
                let range = pc.byte()?;
                // retail 0x8039B514: fmadds before fctiwz/stb
                self.pose = fctiwz(fmadds(
                    f32::from(range),
                    draws.draw(rng, 0x8039_B4FC),
                    f32::from(base),
                )) as u8;
                self.enable_texture();
            }
            0xbe => {
                for value in &mut self.velocity {
                    *value *= pc.float()?;
                }
            }
            0xe3 => self.palette = pc.byte()?,
            0xe4 | 0xe5 => self.texture_flip(opcode, pc.byte()?, rng, draws),
            0xe6 => self.kind |= 1 << 21,
            0xe7 => self.kind &= !(1 << 21),
            0xe8 => {
                let trail = pc.float()?;
                if trail < 0.0 {
                    self.kind &= !(1 << 20);
                } else {
                    self.kind |= 1 << 20;
                    self.trail = trail;
                }
            }
            0xed => self.random_rotation(pc, rng, draws)?,
            _ => {
                return Err(Error::UnsupportedOpcode {
                    opcode,
                    pc: opcode_pc,
                })
            }
        }
        Ok(())
    }

    fn set_flag(&mut self, flag: u32, enabled: bool) {
        if enabled {
            self.kind |= flag;
        } else {
            self.kind &= !flag;
        }
    }

    fn texture_flip(&mut self, opcode: u8, mode: u8, rng: &mut HsdRng, draws: &mut DrawLog) {
        let flag = if opcode == 0xe4 { 1 << 18 } else { 1 << 19 };
        match mode & 3 {
            0 => self.kind &= !flag,
            1 => self.kind |= flag,
            2 => self.kind ^= flag,
            _ => {
                let site = if opcode == 0xe4 {
                    0x8039_C4F8
                } else {
                    0x8039_C58C
                };
                self.set_flag(flag, draws.draw(rng, site) >= 0.5);
            }
        }
    }

    fn random_rotation(
        &mut self,
        pc: &mut Cursor<'_>,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> Result<(), Error> {
        let base = pc.float()?;
        let range = pc.float()?;
        let divisions = pc.byte()?;
        let delta = if divisions != 0 {
            let index = fctiwz(f32::from(u16::from(divisions) + 1) * draws.draw(rng, 0x8039_C870));
            // retail 0x8039C8C4..0x8039C8CC: fmuls/fdivs/fadds.
            base + (range * index as f32) / f32::from(divisions)
        } else {
            // retail 0x8039C8D8: fmadds
            fmadds(range, draws.draw(rng, 0x8039_C8D4), base)
        };
        self.rotation_target += delta;
        self.rotation += delta;
        Ok(())
    }
}

/// Two-byte countdown tracks in HSD_Particle (psstructs.h:154-183).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytePairTrack {
    pub current: [u8; 2],
    pub target: [u8; 2],
    pub duration: u16,
    pub remaining: u16,
}
impl BytePairTrack {
    fn new(current: [u8; 2]) -> Self {
        Self {
            current,
            target: [0; 2],
            duration: 0,
            remaining: 0,
        }
    }
    fn tick(&mut self) {
        // particle.c:732-766 (hsd_8039930C): integer countdown, then copy.
        // Retail 0x80399474/78/7C (material), 0x803994B0/B4/B8
        // (ambient), 0x803994EC/F0/F4 (alpha): lhz/subi/sth; zero
        // branches copy target bytes at 0x80399494, 0x803994D0, 0x8039950C.
        if self.duration != 0 {
            self.remaining = self.remaining.wrapping_sub(1);
            if self.remaining == 0 {
                self.duration = 0;
                self.current = self.target;
            }
        }
    }
}
