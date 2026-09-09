//! Generator creation, attachment, and emission (`generator.c`).
use crate::rng_sites::{DISC_AZIMUTH, DISC_INITIAL_ANGLE, DISC_RADIUS, INITIAL_EMISSION_COUNT};
use crate::{
    bank::Descriptor,
    particle::Particle,
    rng_sites::{DrawLog, SPHERE_AZIMUTH, SPHERE_LATITUDE},
    Error,
};
use gekko_math::{
    fma::fmadds,
    msl::{cosf, fabs, fabsf, fctiwz, sinf, sqrtf},
    rng::HsdRng,
};
use hsd_anim::mtx::{self, InverseTrig, M_PI, M_PI_2};
use hsd_types::{Mtx, Vec3};
use std::sync::Arc;

/// Static psAppSRT transform shared by a positional generator and its particles.
/// psAddGeneratorAppSRT (psappsrt.c:25) initializes unit scale and status; mutable,
/// attached AppSRT callbacks remain an explicit boundary.
#[derive(Debug, Clone, PartialEq)]
pub struct ApplicationTransform {
    pub translation: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
    pub status: i32,
}

/// Shape-dependent state from `HSD_Generator.aux` (psstructs.h).
#[derive(Debug, Clone)]
pub enum EmissionShape {
    Disc {
        mode: u16,
        minimum_angle: f32,
        maximum_angle: f32,
    },
    Line {
        end: [f32; 3],
    },
    Cone {
        mode: u16,
        minimum_angle: f32,
        maximum_angle: f32,
        height: f32,
    },
    Sphere {
        speed: f32,
        latitude_midpoint: f32,
        latitude_range: f32,
        longitude_midpoint: f32,
        longitude_range: f32,
    },
}

/// `HSD_Generator` (psstructs.h); `hsd_8039F05C` creates this state.
#[derive(Debug, Clone)]
pub struct Generator {
    pub id: usize,
    pub family_id: u16,
    pub bank: u8,
    pub link: u8,
    pub flags: u16,
    pub descriptor: Descriptor,
    pub position: [f32; 3],
    pub count: f32,
    pub emission_rate: f32,
    pub remaining_life: u16,
    pub children: u32,
    /// HSD_Generator.appsrt (+0x54), normalized owned AppSRT index.
    pub appsrt_id: Option<usize>,
    pub application_transform: Option<Arc<ApplicationTransform>>,
    pub shape: EmissionShape,
    /// Caller supplies an already evaluated JObj matrix each animation tick.
    pub joint_matrix: Option<Mtx>,
    /// Caller-owned joint identity, inherited by bytecode-created generators.
    pub attachment_id: Option<usize>,
    pub texture_images: Arc<[bool]>,
}

/// Per-emission-pass values computed before the while(count >= 1) loop.
pub(crate) struct EmissionFrame {
    rotation: Mtx,
    speed: f32,
    angle: f32,
    angle_step: f32,
}

impl Generator {
    /// `hsd_8039F05C` (generator.c, 0x8039F05C). Calls the shared RNG only
    /// for initial count when kind bit 0x100 is clear and rate >= 0.
    pub fn new<T: InverseTrig>(
        descriptor: &Descriptor,
        bank: u8,
        link: u8,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> Result<Self, Error> {
        if link >= 8 {
            return Err(Error::InvalidLink(link));
        }
        if descriptor.kind & 0x20000 != 0 {
            return Err(Error::UnsupportedFeature("generator AppSRT"));
        }
        let shape = EmissionShape::new::<T>(descriptor)?;
        let rate = descriptor.emission_rate;
        let count = if descriptor.kind & 0x100 != 0 {
            if rate < 0.0 {
                if 1.0 + rate > f32::EPSILON {
                    1.0
                } else {
                    0.0
                }
            } else {
                f32::from_bits(0x3f7f_fffe)
            }
        } else if rate < 0.0 {
            0.0
        } else {
            draws.draw(rng, INITIAL_EMISSION_COUNT)
        };
        Ok(Self {
            id: 0,
            family_id: 0,
            bank,
            link,
            flags: descriptor.generator_type,
            descriptor: descriptor.clone(),
            position: [0.0; 3],
            count,
            emission_rate: rate,
            remaining_life: descriptor.generator_life,
            children: 0,
            appsrt_id: None,
            application_transform: None,
            shape,
            joint_matrix: None,
            attachment_id: None,
            texture_images: Arc::from([]),
        })
    }

    /// `hsd_8039EFAC` (0x8039EFAC) attaches a generator to its spawn joint.
    pub fn attach_joint(&mut self, matrix: Mtx) {
        self.joint_matrix = Some(matrix);
        self.flags |= 0x700;
    }

    /// `hsd_8039D214` (0x8039D214), after animation and before emission count.
    pub(crate) fn update_attachment(&mut self) {
        if self.flags & 0x300 == 0x300 {
            if let Some(matrix) = self.joint_matrix {
                self.position = [matrix.0[0][3], matrix.0[1][3], matrix.0[2][3]];
            }
        }
    }

    /// `hsd_8039DAD4` (0x8039DAD4), pre-loop setup. Scalar magnitude sums
    /// are unfused (0x8039DB78..0x8039DB88); sqrt uses the three fnmsub
    /// Newton steps at 0x8039DBA8/DBB8/DBC8, as in gekko_math::msl::sqrtf.
    pub(crate) fn prepare<T: InverseTrig>(
        &self,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> Result<EmissionFrame, Error> {
        if !self.count.is_finite() || self.count >= i32::MAX as f32 {
            return Err(Error::InvalidEmissionCount);
        }
        if self.descriptor.kind & 0x10000 != 0 {
            return Err(Error::UnsupportedFeature("camera-facing generator"));
        }
        let speed = magnitude(self.descriptor.velocity);
        let mut rotation = Mtx::IDENTITY;
        if self.flags & 0x500 == 0x500 && self.descriptor.kind & 0x30000 == 0 {
            if let Some(matrix) = self.joint_matrix {
                for column in 0..3 {
                    let v = normalized([
                        matrix.0[0][column],
                        matrix.0[1][column],
                        matrix.0[2][column],
                    ]);
                    for (row, value) in v.into_iter().enumerate() {
                        rotation.0[row][column] = value;
                    }
                }
            }
        }
        if !matches!(self.shape, EmissionShape::Line { .. }) && speed > f32::EPSILON {
            let velocity_rotation = velocity_rotation::<T>(self.descriptor.velocity);
            let original = rotation;
            // PSMTXConcat 0x80342204: reuse the audited paired-single kernel.
            mtx::mtx_concat(&original, &velocity_rotation, &mut rotation);
        }
        let mut frame = EmissionFrame {
            rotation,
            speed,
            angle: 0.0,
            angle_step: 0.0,
        };
        if self.descriptor.angle < 0.0 {
            let count = fctiwz(self.count) as f32;
            match self.shape {
                EmissionShape::Disc {
                    minimum_angle,
                    maximum_angle,
                    ..
                }
                | EmissionShape::Cone {
                    minimum_angle,
                    maximum_angle,
                    ..
                } => {
                    let cone = matches!(self.shape, EmissionShape::Cone { .. });
                    let random = draws.draw(
                        rng,
                        if cone {
                            0x8039_E0D4
                        } else {
                            DISC_INITIAL_ANGLE
                        },
                    );
                    frame.angle_step = (maximum_angle - minimum_angle) / count;
                    // retail 0x8039E0C8 / 0x8039E114: fmadds
                    frame.angle = fmadds(frame.angle_step, random, minimum_angle);
                }
                _ => {
                    frame.angle = (2.0 * (M_PI * f64::from(draws.draw(rng, 0x8039_E11C)))) as f32;
                    frame.angle_step = (2.0 * M_PI / f64::from(count)) as f32;
                }
            }
        }
        Ok(frame)
    }

    /// One iteration of `hsd_8039DAD4` (0x8039DAD4). System performs the
    /// immediate interpreter before the next iteration, preserving RNG order.
    pub(crate) fn emit<T: InverseTrig>(
        &self,
        frame: &mut EmissionFrame,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> Result<Particle, Error> {
        let (position, velocity) = match self.shape {
            EmissionShape::Sphere {
                speed,
                latitude_range,
                ..
            } => self.sphere(frame, speed, latitude_range, rng, draws),
            EmissionShape::Line { end } => {
                let random = draws.draw(rng, 0x8039_E58C);
                let offset = transform(&frame.rotation, end.map(|v| random * v));
                (
                    add(offset, self.position),
                    transform(&frame.rotation, self.descriptor.velocity),
                )
            }
            _ => self.disc::<T>(frame, rng, draws),
        };
        let mut particle = Particle::new(&self.descriptor, self.bank, self.link)?;
        particle.appsrt_id = self.appsrt_id;
        particle.application_transform = self.application_transform.clone();
        particle.generator_id = Some(self.id);
        particle.family_id = self.family_id;
        particle.position = position;
        particle.velocity = velocity;
        particle.texture_images = Arc::clone(&self.texture_images);
        Ok(particle)
    }

    fn sphere(
        &self,
        frame: &EmissionFrame,
        speed: f32,
        latitude_range: f32,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> ([f32; 3], [f32; 3]) {
        // Inline sqrtf has the same three-step fnmsub sequence at
        // 0x8039EB1C/2C/3C, EB8C/9C/AC, EC08/18/28.
        let latitude = if latitude_range == 0.0
            || fabs(f64::from(latitude_range) - M_PI) < f64::from(0.001_f32)
        {
            let mut latitude = (M_PI_2 * f64::from(sqrtf(draws.draw(rng, 0x8039_EB04)))) as f32;
            if draws.draw(rng, 0x8039_EB5C) < 0.5 {
                latitude = (M_PI - f64::from(latitude)) as f32;
            }
            latitude
        } else {
            latitude_range * sqrtf(draws.draw(rng, SPHERE_LATITUDE))
        };
        // retail 0x8039EBD0/EBDC: two fmul (double), then frsp.
        let azimuth = (2.0 * (M_PI * f64::from(draws.draw(rng, SPHERE_AZIMUTH)))) as f32;
        let radius = if self.descriptor.radius < 0.0 {
            -self.descriptor.radius
        } else {
            self.descriptor.radius * sqrtf(draws.draw(rng, 0x8039_EBF0))
        };
        let direction = transform(
            &frame.rotation,
            [
                sinf(latitude) * cosf(azimuth),
                sinf(latitude) * sinf(azimuth),
                cosf(latitude),
            ],
        );
        let mut velocity = direction.map(|v| v * speed);
        if self.descriptor.radius >= 0.0 && speed < 0.0 {
            let scale = radius / self.descriptor.radius;
            for value in &mut velocity {
                *value *= scale;
            }
        }
        // retail 0x8039ED18/ED28/ED38: fmadds
        let position = std::array::from_fn(|i| fmadds(radius, direction[i], self.position[i]));
        (position, velocity)
    }

    fn disc<T: InverseTrig>(
        &self,
        frame: &mut EmissionFrame,
        rng: &mut HsdRng,
        draws: &mut DrawLog,
    ) -> ([f32; 3], [f32; 3]) {
        let (mode, minimum, maximum, height) = match self.shape {
            EmissionShape::Disc {
                mode,
                minimum_angle,
                maximum_angle,
            } => (mode, minimum_angle, maximum_angle, 0.0),
            EmissionShape::Cone {
                mode,
                minimum_angle,
                maximum_angle,
                height,
            } => (mode, minimum_angle, maximum_angle, height),
            _ => unreachable!("dispatch only calls disc for disc/cone"),
        };
        let fraction = if self.descriptor.radius < 0.0 {
            1.0
        } else {
            let random = draws.draw(rng, DISC_RADIUS);
            if mode == 3 || mode == 4 {
                sqrtf(random)
            } else {
                random
            }
        };
        let radius = if self.descriptor.radius < 0.0 {
            -self.descriptor.radius
        } else {
            fraction * self.descriptor.radius
        };
        let negative_angle = self.descriptor.angle < 0.0;
        if negative_angle {
            frame.angle += frame.angle_step;
        } else {
            let site = match mode {
                6 => 0x8039_E300,
                7 => 0x8039_E394,
                _ => DISC_AZIMUTH,
            };
            // retail 0x8039E314/E3A8/E3E8: fmadds
            frame.angle = fmadds(maximum - minimum, draws.draw(rng, site), minimum);
        }
        let angle = self.descriptor.angle;
        let cone_angle = if mode == 6 {
            if fabsf(radius) < f32::MIN_POSITIVE {
                if height >= 0.0 {
                    if negative_angle {
                        -angle
                    } else {
                        angle
                    }
                } else if negative_angle {
                    (M_PI - f64::from(angle)) as f32
                } else {
                    (M_PI + f64::from(angle)) as f32
                }
            } else if negative_angle {
                (M_PI_2 - f64::from(T::atan2f(height, radius)) - f64::from(angle)) as f32
            } else {
                (f64::from(angle) + (M_PI_2 - f64::from(T::atan2f(height, radius)))) as f32
            }
        } else if mode == 7 {
            (if negative_angle {
                M_PI_2 - f64::from(angle)
            } else {
                M_PI_2 + f64::from(angle)
            }) as f32
        } else {
            fraction * if negative_angle { -angle } else { angle }
        };
        let mut offset = [radius * cosf(frame.angle), radius * sinf(frame.angle), 0.0];
        if mode == 6 || mode == 7 {
            offset[2] = draws.draw(rng, 0x8039_E424);
            if mode == 6 {
                offset[0] *= 1.0 - offset[2];
                offset[1] *= 1.0 - offset[2];
            }
            offset[2] *= height;
        }
        let radial_speed = frame.speed * sinf(cone_angle);
        let mut velocity = [
            radial_speed * cosf(frame.angle),
            radial_speed * sinf(frame.angle),
            frame.speed * cosf(cone_angle),
        ];
        if mode == 3 {
            for value in &mut velocity {
                *value *= fraction;
            }
        }
        (
            add(transform(&frame.rotation, offset), self.position),
            transform(&frame.rotation, velocity),
        )
    }
}

impl EmissionShape {
    fn new<T: InverseTrig>(descriptor: &Descriptor) -> Result<Self, Error> {
        let [first, second, third] = descriptor.parameters;
        let (minimum_angle, maximum_angle) = if first == 0.0 && second == 0.0 {
            (0.0, std::f32::consts::TAU)
        } else {
            (first, second)
        };
        Ok(match descriptor.generator_type & 15 {
            mode @ (0 | 3 | 4) => Self::Disc {
                mode,
                minimum_angle,
                maximum_angle,
            },
            1 => Self::Line {
                end: descriptor.parameters,
            },
            mode @ (6 | 7) => Self::Cone {
                mode,
                minimum_angle,
                maximum_angle,
                height: third,
            },
            8 => {
                let [x, y, z] = descriptor.velocity;
                // retail 0x8039F478..F490, F4E8..F500: unfused sum of squares;
                // sqrt fnmsub at F4AC/BC/CC and F51C/2C/3C.
                let speed = magnitude(descriptor.velocity);
                let horizontal = sqrtf(x * x + z * z);
                Self::Sphere {
                    speed: if first < 0.0 { -speed } else { speed },
                    latitude_midpoint: guarded_atan::<T>(y, horizontal),
                    latitude_range: if first < 0.0 { -first } else { first },
                    longitude_midpoint: guarded_atan::<T>(z, x),
                    longitude_range: second,
                }
            }
            shape => return Err(Error::UnsupportedGenerator { shape }),
        })
    }
}

fn guarded_atan<T: InverseTrig>(y: f32, x: f32) -> f32 {
    if fabsf(x) < f32::MIN_POSITIVE {
        if y >= 0.0 {
            std::f32::consts::FRAC_PI_2
        } else {
            -std::f32::consts::FRAC_PI_2
        }
    } else {
        T::atan2f(y, x)
    }
}
fn magnitude([x, y, z]: [f32; 3]) -> f32 {
    sqrtf(z * z + (x * x + y * y))
}
fn normalized(v: [f32; 3]) -> [f32; 3] {
    let mut result = Vec3::default();
    mtx::vec_normalize(
        &Vec3 {
            x: v[0],
            y: v[1],
            z: v[2],
        },
        &mut result,
    );
    [result.x, result.y, result.z]
}
fn transform(matrix: &Mtx, v: [f32; 3]) -> [f32; 3] {
    let mut result = Vec3::default();
    mtx::mtx_mult_vec(
        matrix,
        &Vec3 {
            x: v[0],
            y: v[1],
            z: v[2],
        },
        &mut result,
    );
    [result.x, result.y, result.z]
}
fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|i| a[i] + b[i])
}

/// Velocity alignment in `hsd_8039DAD4`, 0x8039DDE4..0x8039DF68.
fn velocity_rotation<T: InverseTrig>(velocity: [f32; 3]) -> Mtx {
    let [x, y, z] = normalized(velocity);
    let azimuth = guarded_atan::<T>(y, z);
    let sin_azimuth = sinf(azimuth);
    let cos_azimuth = cosf(azimuth);
    // retail 0x8039DEA0/DEA4: z*cos rounded first, then fmadds(y,sin,...).
    let projected = fmadds(y, sin_azimuth, z * cos_azimuth);
    let elevation = guarded_atan::<T>(x, projected);
    let sin_elevation = sinf(elevation);
    let cos_elevation = cosf(elevation);
    Mtx([
        [cos_elevation, 0.0, sin_elevation, 0.0],
        [
            -sin_azimuth * sin_elevation,
            cos_azimuth,
            sin_azimuth * cos_elevation,
            0.0,
        ],
        [
            -cos_azimuth * sin_elevation,
            -sin_azimuth,
            cos_azimuth * cos_elevation,
            0.0,
        ],
    ])
}
