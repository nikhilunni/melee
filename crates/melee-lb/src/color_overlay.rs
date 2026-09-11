//! lb_80014014 / lb_800140F8 / lb_80014258 color overlay playback.
use hsd_archive::desc::color_animation::ColorCommand;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorOverlay {
    pub color: [u8; 4],
    pub enabled: bool,
    pub complete: bool,
    cursor: usize,
    timer: u32,
    channels: [f32; 4],
    increments: [f32; 4],
}
impl Default for ColorOverlay {
    fn default() -> Self {
        Self {
            color: [0; 4],
            enabled: false,
            complete: true,
            cursor: 0,
            timer: 0,
            channels: [0.0; 4],
            increments: [0.0; 4],
        }
    }
}
impl ColorOverlay {
    /// grMaterial_801C9604 immediately interprets the first script frame.
    pub fn start(&mut self, program: &[ColorCommand]) {
        self.cursor = 0;
        self.timer = 0;
        self.enabled = false;
        self.complete = false;
        self.tick(program);
    }
    pub fn tick(&mut self, program: &[ColorCommand]) {
        if self.complete {
            return;
        }
        self.timer = self.timer.saturating_sub(1);
        while self.timer == 0 {
            let command = program[self.cursor];
            self.cursor += 1;
            match command {
                ColorCommand::Complete => {
                    self.complete = true;
                    return;
                }
                ColorCommand::Disable => self.enabled = false,
                ColorCommand::Wait(frames) => self.timer += frames,
                ColorCommand::Set(color) => {
                    self.enabled = true;
                    self.color = color;
                    self.channels = color.map(f32::from);
                    self.increments = [0.0; 4];
                }
                ColorCommand::Blend { color, frames } => {
                    // retail lb_800140F8 0x800140F8: fadds, fsubs, fdivs;
                    // no fused instructions. Half-unit bias precedes subtraction.
                    self.increments = std::array::from_fn(|i| {
                        ((0.5 + f32::from(color[i])) - f32::from(self.color[i])) / frames as f32
                    });
                }
            }
        }
        if self.enabled {
            for i in 0..4 {
                // retail lb_80014258 0x80014258: fadds, fctiwz, byte store.
                self.channels[i] += self.increments[i];
                self.color[i] = (self.channels[i] as i32) as u8;
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fade_updates_on_request_and_completes_before_an_extra_color_step() {
        let program = [
            ColorCommand::Set([0; 4]),
            ColorCommand::Blend {
                color: [200, 200, 200, 255],
                frames: 120,
            },
            ColorCommand::Wait(120),
            ColorCommand::Complete,
        ];
        let mut overlay = ColorOverlay::default();
        overlay.start(&program);
        assert_eq!(overlay.color, [1, 1, 1, 2]);
        for _ in 1..120 {
            overlay.tick(&program);
        }
        assert_eq!(overlay.color, [200, 200, 200, 255]);
        assert!(!overlay.complete);
        let mut branch = overlay;
        overlay.tick(&program);
        branch.tick(&program);
        assert_eq!(overlay, branch);
        assert!(overlay.complete);
        assert_eq!(overlay.color, [200, 200, 200, 255]);
    }
}
