//! ftAction_80073240 / lbcommand.c: timed script execution without owner callbacks.
use crate::Command;
use melee_types::fixed::FixedVec;

/// Port bound on nested control flow, checked before writing. The decomp's
/// event_return[3] is explicitly a guessed size (lb/types.h:1015).
const CONTROL_DEPTH: usize = 16;
#[derive(Clone, Debug)]
pub struct CommandLoop {
    pub start: usize,
    pub remaining: u32,
}
#[derive(Clone, Debug, Default)]
pub struct ScriptState {
    pub instruction: Option<usize>,
    pub timer: f32,
    pub frame: f32,
    pub return_stack: FixedVec<usize, CONTROL_DEPTH>,
    pub loops: FixedVec<CommandLoop, CONTROL_DEPTH>,
    wait_for_next_step: bool,
}
impl ScriptState {
    pub fn restart(&mut self, instruction: usize) {
        self.instruction = Some(instruction);
        self.timer = 0.0;
        self.return_stack.clear();
        self.loops.clear();
        self.wait_for_next_step = false;
    }
    /// ftaction.c:1318-1348; no fused arithmetic. Frame is sampled even for End.
    pub fn begin_frame(&mut self, frame: f32, speed: f32) {
        self.frame = frame;
        self.wait_for_next_step = false;
        if self.instruction.is_some() && self.timer != f32::MAX {
            self.timer -= speed;
        }
    }
    /// Emit one owner command at a time, so application finishes before the
    /// next instruction. The returned reference borrows only immutable code.
    pub fn next<'a>(&mut self, script: &'a [Command], speed: f32) -> Option<&'a Command> {
        if self.wait_for_next_step {
            return None;
        }
        while let Some(pc) = self.instruction {
            if self.timer == f32::MAX {
                if self.frame >= speed {
                    return None;
                }
                self.timer = -self.frame;
            } else if self.timer > 0.0 {
                return None;
            }
            self.instruction = Some(pc + 1);
            match &script[pc] {
                Command::BeginLoop(count) => self.loops.push(CommandLoop {
                    start: pc + 1,
                    remaining: *count,
                }),
                Command::EndLoop => {
                    let state = self
                        .loops
                        .last_mut()
                        .expect("Command_04 without Command_03");
                    state.remaining = state.remaining.wrapping_sub(1);
                    if state.remaining != 0 {
                        self.instruction = Some(state.start);
                    } else {
                        self.loops.pop();
                    }
                }
                Command::End => self.instruction = None,
                Command::Goto(target) => self.instruction = Some(*target),
                Command::WaitAnimationLoop => {
                    self.timer = f32::MAX;
                    self.wait_for_next_step = true;
                    return None;
                }
                Command::Wait(frames) => self.timer += frames,
                Command::AtFrame(frame) => self.timer = frame - self.frame,
                Command::Call {
                    target,
                    continuation,
                } => {
                    self.return_stack.push(*continuation);
                    self.instruction = Some(*target);
                }
                Command::Return => {
                    self.instruction = Some(
                        self.return_stack
                            .pop()
                            .expect("command return without call"),
                    )
                }
                Command::Unported(opcode) => {
                    unimplemented!("ftAction_80073240 (ftaction.c): subaction opcode {opcode}")
                }
                command => return Some(command),
            }
        }
        None
    }
}
