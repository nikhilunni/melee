//! Map 7's flight (grVenom_8020454C, 0x8020454C): its animation's frame
//! (the first AObj, grAnime_801C8318(gobj, 0, 7)) crossing set frames
//! raises and lowers the flags `u.venom2.xE0_state` b0..b6. b0..b2 (frames
//! 3800..6100) keep every Arwing away; b3..b5 (6810..9200) allow only the
//! near passes 1..4. At 4400 the tunnel's floor (collision joint 2) comes
//! and at 6100 it goes. b6 only colours the lights.

/// The tunnel floor's collision joint (map 9's root).
pub const TUNNEL_JOINT: i32 = 2;

/// `u.venom2.xE0_state` and the frame the proc last read (`u.venom.xE4`).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Sequence {
    pub flags: [bool; 7],
    pub frame: f32,
}

/// What a frame step does to the collision.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JointChange {
    /// mpJointListAdd(2).
    On,
    /// mpLib_80057BC0(2).
    Off,
}

/// The flags' groups grVenom_8020362C reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Groups {
    /// b0 | b1 | b2.
    pub tunnel: bool,
    /// b3 | b4 | b5.
    pub near_only: bool,
}

impl Sequence {
    pub fn groups(&self) -> Groups {
        let f = &self.flags;
        Groups {
            tunnel: f[0] | f[1] | f[2],
            near_only: f[3] | f[4] | f[5],
        }
    }

    /// One proc step at the animation's frame `frame`: each comparison is
    /// `xE4 < mark && mark <= frame` (the integer marks compare as floats),
    /// in the proc's order. The changes come in the proc's order too.
    pub fn step(&mut self, frame: f32) -> [Option<JointChange>; 2] {
        let last = self.frame;
        let crossed = |mark: f32| last < mark && mark <= frame;
        let any = |marks: &[f32]| marks.iter().any(|&m| crossed(m));
        let mut changes = [None; 2];
        if any(&[2460.0, 6440.0, 9330.0]) {
            self.flags[6] = true;
        }
        if any(&[4070.0, 7220.0]) || last > frame {
            self.flags[6] = false;
        }
        if crossed(4200.0) {
            self.flags[0] = true;
        }
        if crossed(3800.0) {
            self.flags[1] = true;
        }
        if crossed(4400.0) {
            self.flags[2] = true;
            changes[0] = Some(JointChange::On);
        }
        if crossed(6100.0) {
            self.flags[0] = false;
            self.flags[1] = false;
            self.flags[2] = false;
            changes[1] = Some(JointChange::Off);
        }
        if crossed(7210.0) {
            self.flags[3] = true;
        }
        if crossed(6810.0) {
            self.flags[4] = true;
        }
        if crossed(7410.0) {
            self.flags[5] = true;
        }
        if crossed(9200.0) {
            self.flags[3] = false;
            self.flags[4] = false;
            self.flags[5] = false;
        }
        self.frame = frame;
        changes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tunnel_floor_comes_at_4400_and_goes_at_6100() {
        let mut sequence = Sequence::default();
        let mut changes = Vec::new();
        for frame in 0..7000 {
            for change in sequence.step(frame as f32).into_iter().flatten() {
                changes.push((frame, change));
            }
            if frame == 4300 {
                assert!(sequence.groups().tunnel);
            }
        }
        assert_eq!(changes, [(4400, JointChange::On), (6100, JointChange::Off)]);
        assert!(!sequence.groups().tunnel);
    }

    #[test]
    fn only_near_passes_between_6810_and_9200() {
        let mut sequence = Sequence::default();
        for frame in 0..=9300 {
            sequence.step(frame as f32);
            let near_only = sequence.groups().near_only;
            assert_eq!(near_only, (6810..9200).contains(&frame), "frame {frame}");
        }
    }
}
