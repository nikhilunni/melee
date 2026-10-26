//! The side platforms (map 6, grCastle_801CDA0C / 801CDC44): each rises
//! while fighters stand on it and sinks once they leave.
use super::Parameters;
use gekko_math::fma::fmadds;

/// Map 6's platform bones (Ground_801C3FA4(gobj, 1 / 4)) and their
/// collision joints (grCs_803E0FE8, the archive's own mappings).
pub const BONES: [usize; 2] = [1, 4];
pub const JOINTS: [i32; 2] = [4, 5];

/// One platform (`grCastle_Platform`, map 6 +0xC4, stride 0x10).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Platform {
    /// +0x4: its height (the bone's translate y).
    pub position: f32,
    /// +0x8: 0 loaded, 1 empty, 2 rising, 3 sinking.
    pub state: i16,
    /// +0xA: ticks in the loaded or empty wait.
    pub timer: i16,
    /// +0xC: this tick's weight, `coll->x50` of each fighter on it
    /// (grCastle_801CF750).
    pub weight: f32,
}

impl Platform {
    /// grCastle_801CDA0C (0x801CDA0C): at the bottom when x18 is set.
    pub fn new(p: &Parameters) -> Self {
        Self {
            position: if p.platform_starts_low != 0.0 {
                p.platform_bottom
            } else {
                p.platform_top
            },
            state: 0,
            // (s16) x38: fctiwz.
            timer: gekko_math::msl::fctiwz(p.rise_delay) as i16,
            weight: 0.0,
        }
    }

    /// grCastle_801CF750 (0x801CF750): a fighter's floor contact on the
    /// platform's joint.
    pub fn stand(&mut self, collision_type: u8, x50: i32) {
        if collision_type == 1 {
            self.weight += x50 as f32;
        }
    }

    /// grCastle_801CDC44 (0x801CDC44) for one platform: its new height
    /// when it moved (HSD_JObjSetTranslateY), then the weight cleared.
    pub fn step(&mut self, p: &Parameters) -> Option<f32> {
        let mut moved = None;
        let loaded = self.weight > 0.0;
        match self.state {
            0 => {
                if loaded {
                    let count = self.timer;
                    self.timer = count.wrapping_add(1);
                    if f32::from(count) > p.rise_delay {
                        self.state = 2;
                    }
                } else {
                    self.timer = 0;
                    self.state = 1;
                }
            }
            1 => {
                if loaded {
                    self.timer = 0;
                    self.state = 0;
                } else {
                    let count = self.timer;
                    self.timer = count.wrapping_add(1);
                    if f32::from(count) > p.sink_delay {
                        self.state = 3;
                    }
                }
            }
            2 => {
                if loaded {
                    // retail 0x801CDD74: fmadds.
                    let mut speed = fmadds(self.weight, p.rise[0], p.rise[1]);
                    if speed > p.rise[2] {
                        speed = p.rise[2];
                    }
                    let top = p.platform_top;
                    if top - self.position < speed {
                        self.position = top;
                    } else {
                        self.position += speed;
                    }
                    moved = Some(self.position);
                } else {
                    self.timer = 0;
                    self.state = 1;
                }
            }
            3 => {
                if loaded {
                    self.timer = 0;
                    self.state = 0;
                } else {
                    // retail 0x801CDE68: fmadds.
                    let mut speed = fmadds(self.weight, p.sink[0], p.sink[1]);
                    if speed > p.sink[2] {
                        speed = p.sink[2];
                    }
                    let bottom = p.platform_bottom;
                    if self.position - bottom < speed {
                        self.position = bottom;
                    } else {
                        self.position -= speed;
                    }
                    moved = Some(self.position);
                }
            }
            _ => {}
        }
        self.weight = 0.0;
        moved
    }
}
