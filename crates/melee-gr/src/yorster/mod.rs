//! Yoshi's Island (`gr/gryorster.c`, GrYt.dat, StKind 0x10). Map 1 carries
//! the whole stage and its nine flipping blocks (bones 10..18, one collision
//! joint each). A block flips when a hit on its stage material item
//! (grMaterial_801C8CFC) or a head bump from below (the joint's ceiling
//! callback) passes its threshold: it lets fighters through while it spins
//! and returns once nothing stands where it rests.
pub mod procs;
use gekko_math::msl::fctiwz;
use hsd_types::Vec3;

/// `YorsterParams` (gryorster.c:57-66, GrYt.dat `yakumono_param`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Parameters {
    /// x00: the upward speed (`pos_delta.y`) of a bump that flips a block,
    /// and the bump total a flip consumes.
    pub bump_speed: f32,
    /// x04: a bump past this flips the block with no extra spin.
    pub bump_full: f32,
    /// x08: the damage that flips a block.
    pub damage_threshold: f32,
    /// x0C: damage past this flips the block with no extra spin.
    pub damage_full: f32,
    /// x10: the freeze a bump puts on the fighter (ftLib_80086A4C).
    pub bump_freeze: i32,
    /// x14: the freeze a hit puts on its attacker.
    pub hit_freeze: i32,
    /// x18: the ticks between the trigger and the spin.
    pub flip_delay: i32,
    /// x1C: the spin's last start frame.
    pub spin_end_frame: i32,
}

/// The nine blocks' bones (grYorster_802022A4's `joints`).
pub const BLOCK_BONES: [usize; 9] = [10, 11, 12, 13, 14, 15, 16, 17, 18];
/// grAnime_801C7FF8 animation indices on a block's subtree.
pub mod block_animation {
    /// The spin, started at the block's resume frame.
    pub const SPIN: usize = 0;
    /// The rest pose (rate 0 at creation, 1 once back).
    pub const REST: usize = 1;
    /// The wobble while something stands where the block returns.
    pub const BLOCKED: usize = 2;
}
/// grYorster_8020266C: a spinning block's clock runs to 0x143.
const SPIN_TICKS: i32 = 0x143;
/// grYorster_8020266C state 4: the wobble ends after 0x19 ticks at most.
const BLOCKED_TICKS: i32 = 0x19;
/// grAnime_801C7FF8(..., 2, 0.0, 0.3): the wobble's rate.
const BLOCKED_RATE: f32 = 0.3;

/// `elements[i].x01`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockPhase {
    /// 0: as created.
    Created,
    /// 1: back in place.
    Rest,
    /// 2: triggered, waiting `flip_delay`.
    Triggered,
    /// 3: spinning, intangible.
    Spinning,
    /// 4: wobbling while blocked.
    Blocked,
}
impl BlockPhase {
    pub fn from_saved(value: i8) -> Option<Self> {
        Some(match value {
            0 => Self::Created,
            1 => Self::Rest,
            2 => Self::Triggered,
            3 => Self::Spinning,
            4 => Self::Blocked,
            _ => return None,
        })
    }
}

/// One block (`GroundVars_yorster.elements[i]`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    /// x01.
    pub phase: BlockPhase,
    /// x04: damage taken (the material item's hits).
    pub damage: f32,
    /// x08: this tick's bumps (cleared after every step).
    pub bump: f32,
    /// x0C: the phase's clock.
    pub clock: i32,
    /// x10: the spin's start frame, then its ticks.
    pub resume_frame: i32,
    /// x14: the block's bone.
    pub bone: usize,
}
impl Block {
    pub fn new(bone: usize) -> Self {
        Self {
            phase: BlockPhase::Created,
            damage: 0.0,
            bump: 0.0,
            clock: 0,
            resume_frame: 0,
            bone,
        }
    }
}

/// What a block's step does to the stage, in retail order.
pub trait BlockEngine {
    /// grMaterial_801C8E28: the block's material item stops taking hits.
    fn disable_hurt(&mut self, block: usize);
    /// grMaterial_801C8E08: it takes hits again.
    fn enable_hurt(&mut self, block: usize);
    /// lbGetJObjEndFrame(block jobj).
    fn end_frame(&mut self, block: usize) -> f32;
    /// grAnime_801C7FF8(gobj, bone, 7, index, frame, rate).
    fn play(&mut self, block: usize, index: usize, frame: f32, rate: f32);
    /// grAnime_801C84A4(gobj, bone, 7): the subtree's first AObj rewound.
    fn rewound(&mut self, block: usize) -> bool;
    /// mpLib_80057BC0 / mpJointListAdd on the block's collision joint.
    fn set_solid(&mut self, block: usize, solid: bool);
    /// grLib_801C9EE8(HSD_JObjGetTranslation(block jobj), offset).
    fn occupied(&mut self, block: usize, offset: f32) -> bool;
    /// Ground_801C53EC(0xE): the bump's sound.
    fn bump_sound(&mut self) {}
}

/// The `(f32) frames_left * (end_frame / span)` spin start of a flip
/// (0x802027C8 fdivs, 0x802027D4 fmuls, fctiwz; 0x802028C0..D0 for a bump).
fn resume_frame(frames_left: i32, end_frame: f32, span: f32) -> i32 {
    fctiwz(frames_left as f32 * (end_frame / span))
}

/// `min((f32) x1C, lbGetJObjEndFrame)`: 0x80202798's `>=` keeps the JObj's.
fn spin_end<E: BlockEngine>(parameters: &Parameters, engine: &mut E, block: usize) -> f32 {
    let end_frame = parameters.spin_end_frame as f32;
    let jobj_end = engine.end_frame(block);
    if end_frame >= jobj_end {
        jobj_end
    } else {
        end_frame
    }
}

impl Block {
    /// grYorster_8020266C (0x8020266C) for one block. `map_scale` is
    /// Ground_801C0498.
    pub fn step<E: BlockEngine>(
        &mut self,
        index: usize,
        parameters: &Parameters,
        map_scale: f32,
        engine: &mut E,
    ) {
        match self.phase {
            BlockPhase::Created | BlockPhase::Rest => self.check_trigger(index, parameters, engine),
            BlockPhase::Triggered => {
                if self.clock <= 0 {
                    engine.play(index, block_animation::SPIN, self.resume_frame as f32, 1.0);
                    engine.set_solid(index, false);
                    self.phase = BlockPhase::Spinning;
                    self.clock = self.resume_frame;
                } else {
                    self.clock -= 1;
                }
            }
            BlockPhase::Spinning => {
                if self.clock >= SPIN_TICKS {
                    // 0x802029B4: fmsubs 10.0 * scale - 2.0.
                    let offset = gekko_math::fma::fmsubs(10.0, map_scale, 2.0);
                    if engine.occupied(index, offset) {
                        engine.play(index, block_animation::BLOCKED, 0.0, BLOCKED_RATE);
                        self.clock = 0;
                        self.phase = BlockPhase::Blocked;
                    } else {
                        self.come_back(index, engine);
                    }
                } else {
                    self.clock += 1;
                }
            }
            BlockPhase::Blocked => {
                if engine.rewound(index) || self.clock >= BLOCKED_TICKS {
                    self.clock = 0;
                    // 0x80202AB8: two fmuls, (10.0 * scale) * 0.5.
                    let offset = (10.0 * map_scale) * 0.5;
                    if !engine.occupied(index, offset) {
                        self.come_back(index, engine);
                    }
                } else {
                    self.clock += 1;
                }
            }
        }
        self.bump = 0.0;
    }

    /// The block's rest animation from frame zero, solid and hittable again.
    fn come_back<E: BlockEngine>(&mut self, index: usize, engine: &mut E) {
        engine.play(index, block_animation::REST, 0.0, 1.0);
        engine.set_solid(index, true);
        engine.enable_hurt(index);
        self.damage = 0.0;
        self.bump = 0.0;
        self.phase = BlockPhase::Rest;
    }

    /// States 0 and 1: damage first, then this tick's bumps.
    fn check_trigger<E: BlockEngine>(&mut self, index: usize, p: &Parameters, engine: &mut E) {
        let triggered = if self.damage >= p.damage_threshold {
            engine.disable_hurt(index);
            self.damage -= p.damage_threshold;
            let frames_left = if self.damage >= p.damage_full {
                0
            } else {
                fctiwz(p.damage_full - self.damage)
            };
            let end_frame = spin_end(p, engine, index);
            self.resume_frame = resume_frame(frames_left, end_frame, p.damage_full);
            true
        } else if self.bump >= p.bump_speed {
            engine.disable_hurt(index);
            self.bump -= p.bump_speed;
            if self.bump <= p.bump_speed {
                self.bump = p.bump_speed;
            }
            let frames_left = if self.bump >= p.bump_full {
                0
            } else {
                // 0x8020285C: fmuls 10.0 * (x04 - x08).
                fctiwz(10.0 * (p.bump_full - self.bump))
            };
            let end_frame = spin_end(p, engine, index);
            // 0x802028C0: fmuls 10.0 * x04, then the fdivs.
            self.resume_frame = resume_frame(frames_left, end_frame, 10.0 * p.bump_full);
            self.bump = 0.0;
            engine.bump_sound();
            true
        } else {
            false
        };
        if triggered {
            self.clock = p.flip_delay;
            self.phase = BlockPhase::Triggered;
        }
    }
}

/// grLib_801C9EE8's PointInsideColl (grlib.c:308): `point` lies within
/// `offset` of the ECB's box around `position`. 0x801C9F38..0x801C9F84:
/// the sums and differences separate, the halves fused (fmadds).
pub fn point_inside_ecb(
    position: Vec3,
    ecb: &melee_types::mp::FtEcb,
    point: Vec3,
    offset: f32,
) -> bool {
    use gekko_math::fma::fmadds;
    let close = |centre: f32, sum: f32, span: f32, at: f32| {
        let mut distance = fmadds(0.5, sum, centre) - at;
        if distance < 0.0 {
            distance = -distance;
        }
        // fcmpo; ble: a NaN distance is outside.
        distance <= fmadds(0.5, span, offset)
    };
    close(
        position.y,
        ecb.top.y + ecb.bottom.y,
        ecb.top.y - ecb.bottom.y,
        point.y,
    ) && close(
        position.x,
        ecb.left.x + ecb.right.x,
        ecb.right.x - ecb.left.x,
        point.x,
    )
}
