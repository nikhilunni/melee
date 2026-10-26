//! The switches (maps 18..20, grCastle_801CF0F4 / 801CF308), the roof's
//! choice of the next one (grCastle_801CF7B0 / 801CF868) and the block sets
//! they fill in (maps 5, 7 and 17, grCastle_801CEACC / 801CEF04).
use super::{rand_range, Parameters};
use gekko_math::HsdRng;

/// grCastle_801CF7B0's creation order: the switches' maps, which index the
/// roof's weights (`xC4[0..3]`).
pub const SWITCH_MAPS: [u8; 3] = [20, 19, 18];
/// grCastle_801CF0F4: each switch's block set (by map id: 0x14 Onett,
/// 0x13 Big Blue and 0x12 Mute City name maps 17, 7 and 5).
pub const BLOCK_MAPS: [u8; 3] = [17, 7, 5];
/// grCastle_801CF0F4: the switch model's scale, times the map scale.
pub const SWITCH_SCALE: f32 = 1.5;
/// grCastle_801CF308's hurt capsule (grMaterial_801C8DE0).
pub const SWITCH_CAPSULE: ([f32; 3], [f32; 3], f32) = ([0.0, -1.0, 0.0], [0.0, 1.0, 0.0], 4.0);
/// The material item's state while a switch waits (grMaterial_801C8CFC(0, 1)).
pub const SWITCH_ITEM_STATE: u16 = 1;
/// grCs_803B7EFC: where a switch may appear, (map, bone).
pub const PLACES: [(u8, usize); 11] = [
    (3, 16),
    (3, 17),
    (3, 14),
    (3, 15),
    (3, 18),
    (3, 19),
    (3, 21),
    (3, 22),
    (3, 20),
    (6, 2),
    (6, 5),
];
/// grCs_803B7EF0.
const WEIGHTS: [i32; 3] = [100, 100, 100];
/// grCs_803B7EC8, read a byte at a time: the appearing switch's blink
/// delays (halved), -1 ending it.
const BLINK: [i16; 19] = [
    0x201, 0x201, 0x201, 0x201, 0x201, 0x201, 0x101, 0x101, 0x101, 0x101, 0x101, 0x101, 0x101,
    0x101, 0x101, 0x102, 0x102, 0x102, 0x1ff,
];

fn blink_byte(index: i32) -> i8 {
    let half = BLINK[(index / 2) as usize].to_be_bytes();
    half[(index % 2) as usize] as i8
}

/// Map 3's switch countdown (`castle2`/`castle12`: xD0, xD2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwitchSelector {
    /// xD0: the switch last chosen (an index into [`SWITCH_MAPS`]), -1 none.
    pub current: i16,
    /// xD2: ticks to the next switch.
    pub timer: i16,
}

/// A switch the selector placed: on `PLACES[place]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub switch: usize,
    pub place: usize,
}

impl SwitchSelector {
    /// grCastle_801CF7B0 (0x801CF7B0), after it created the switches.
    pub fn new(p: &Parameters, rng: &mut HsdRng) -> Self {
        Self {
            current: -1,
            timer: wait(p, rng) as i16,
        }
    }

    /// grCastle_801CF868 (0x801CF868): while no switch is out, count down,
    /// then choose one by weight (the last one's weight divided) and a
    /// place for it. `idle[i]` is switch `i`'s state 0.
    pub fn step(&mut self, p: &Parameters, idle: [bool; 3], rng: &mut HsdRng) -> Option<Placement> {
        if self.current != -1 && !idle[self.current as usize] {
            return None;
        }
        self.timer = self.timer.wrapping_sub(1);
        if self.timer >= 0 {
            return None;
        }
        self.timer = wait(p, rng) as i16;
        let mut weights = WEIGHTS;
        if self.current != -1 {
            weights[self.current as usize] /= p.switch_repeat_divisor;
        }
        let total = weights[0] + weights[1] + weights[2];
        let mut draw = super::randi_or_zero(total, rng);
        let mut switch = 0;
        while switch < 3 {
            draw -= weights[switch];
            if draw < 0 {
                break;
            }
            switch += 1;
        }
        if switch == 3 {
            switch = 0;
        }
        self.current = switch as i16;
        let place = rng.randi(PLACES.len() as i32) as usize;
        Some(Placement { switch, place })
    }
}

fn wait(p: &Parameters, rng: &mut HsdRng) -> i32 {
    rand_range(p.switch_wait[1], p.switch_wait[0], rng)
}

/// A switch's phase (`castle5.xC4`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwitchState {
    Hidden = 0,
    Appearing = 1,
    Blinking = 2,
    Waiting = 3,
    Pressed = 4,
    Retiring = 5,
}

/// One switch (maps 18..20).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Switch {
    pub state: SwitchState,
    /// xC8: the blink byte reached.
    pub blink: i32,
    /// xCA: ticks to the next blink.
    pub delay: i16,
    /// +0xCC: ticks it lingers once its animation ended.
    pub linger: i16,
    /// xD4: the bone it sits on.
    pub place: usize,
}

/// What a switch step asks of the stage, in retail order.
pub trait SwitchEngine {
    /// HSD_JObjClearFlagsAll / SetFlagsAll(HIDDEN) on the switch.
    fn set_hidden(&mut self, hidden: bool);
    /// HSD_JObjGetFlags(switch) & HIDDEN.
    fn hidden(&self) -> bool;
    /// grAnime_801C86D4(map, gobj, 0): the model root back to its archive pose.
    fn reset_root(&mut self);
    /// The material item (state 1, capsule, hurt on), made when the blink ends.
    fn create_item(&mut self);
    /// grMaterial_801C8CDC on the material item.
    fn destroy_item(&mut self);
    /// The switch's block set to state 1 (grCastle_801CEF04 starts).
    fn start_blocks(&mut self);
    /// The block set is back to state 0.
    fn blocks_idle(&self) -> bool;
    /// grAnime_801C8138(gobj, map, 0): the press, Small quake.
    fn press(&mut self);
    /// grAnime_801C83D0(gobj, 0, 7).
    fn animation_ended(&self) -> bool;
    /// Follow the place's bone (lb_8000B1CC, HSD_JObjGetRotation).
    fn follow(&mut self, place: usize);
}

impl Switch {
    /// grCastle_801CF0F4 (0x801CF0F4).
    pub fn new() -> Self {
        Self {
            state: SwitchState::Hidden,
            blink: 0,
            delay: 0,
            linger: 0,
            place: 0,
        }
    }

    /// grCastle_801CF868's choice: the switch appears on `place`.
    pub fn place(&mut self, place: usize) {
        self.place = place;
        self.state = SwitchState::Appearing;
    }

    /// fn_801CFAFC / fn_801CFB68 (the material item's hit or touch).
    pub fn hit(&mut self) {
        self.state = SwitchState::Pressed;
    }

    /// grCastle_801CF308 (0x801CF308).
    pub fn step<E: SwitchEngine>(&mut self, p: &Parameters, engine: &mut E) {
        let follows = match self.state {
            SwitchState::Hidden => false,
            SwitchState::Appearing | SwitchState::Blinking => {
                if self.state == SwitchState::Appearing {
                    self.blink = -1;
                    self.delay = 0;
                    engine.set_hidden(false);
                    engine.reset_root();
                    self.state = SwitchState::Blinking;
                }
                if self.delay > 0 {
                    self.delay -= 1;
                } else {
                    self.blink += 1;
                    let hidden = engine.hidden();
                    engine.set_hidden(!hidden);
                    let value = blink_byte(self.blink);
                    if value != -1 {
                        // (s16)(2.0 * val): in double, exact.
                        self.delay = (2.0 * f64::from(value)) as i16;
                    } else {
                        self.state = SwitchState::Waiting;
                        engine.create_item();
                        self.linger = p.switch_linger as i16;
                    }
                }
                true
            }
            SwitchState::Waiting => true,
            SwitchState::Pressed | SwitchState::Retiring => {
                if self.state == SwitchState::Pressed {
                    engine.destroy_item();
                    engine.start_blocks();
                    self.state = SwitchState::Retiring;
                    engine.press();
                }
                if engine.animation_ended() {
                    let linger = self.linger;
                    self.linger = linger.wrapping_sub(1);
                    if linger < 0 {
                        engine.set_hidden(true);
                    }
                }
                if engine.blocks_idle() {
                    self.state = SwitchState::Hidden;
                    engine.set_hidden(true);
                }
                true
            }
        };
        if follows {
            engine.follow(self.place);
        }
    }
}

impl Default for Switch {
    fn default() -> Self {
        Self::new()
    }
}

/// A block set's phase (`castle6.xC4`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlocksState {
    Idle = 0,
    Growing = 1,
    Standing = 2,
    Blinking = 3,
    Shrinking = 4,
}

/// One block of a set (`castle10`'s per-block arrays).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    pub bone: usize,
    /// The `?` block's two looks (effect_a shown with items on, effect_b
    /// the used block).
    pub looks: Option<(usize, usize)>,
    /// x120: its collision joint.
    pub joint: i32,
}

/// grCastle_801CEACC's blocks per map.
pub fn blocks(map: u8) -> &'static [Block] {
    const fn b(bone: usize, looks: Option<(usize, usize)>, joint: i32) -> Block {
        Block { bone, looks, joint }
    }
    // Map 7's third block (bone 7, joint 8) is hidden and disabled for good.
    const MAP17: [Block; 4] = [
        b(1, Some((2, 3)), 0xB),
        b(4, Some((5, 6)), 0xC),
        b(7, None, 0xD),
        b(8, None, 0xE),
    ];
    const MAP7: [Block; 4] = [
        b(1, Some((2, 3)), 6),
        b(4, Some((5, 6)), 7),
        b(0xA, None, 9),
        b(0xB, None, 0xA),
    ];
    const MAP5: [Block; 3] = [b(1, Some((2, 3)), 0), b(4, None, 1), b(5, None, 2)];
    match map {
        17 => &MAP17,
        7 => &MAP7,
        5 => &MAP5,
        _ => unreachable!("castle block map {map}"),
    }
}

/// What a block set step asks of the stage.
pub trait BlockEngine {
    /// HSD_JObjGetFlags(bone) & HIDDEN.
    fn hidden(&self, bone: usize) -> bool;
    /// HSD_JObjClearFlags / SetFlags(bone, HIDDEN) (that joint only).
    fn set_hidden(&mut self, bone: usize, hidden: bool);
    fn scale_y(&self, bone: usize) -> f32;
    fn set_scale_y(&mut self, bone: usize, y: f32);
    /// grCastle_801CDF54: a Bullet Bill flies within 40 of the bone.
    fn bill_near(&mut self, bone: usize) -> bool;
    /// gm_8016AE80() != -1 && gm_8016B238() == 0: items are on.
    fn items_on(&self) -> bool;
    fn enable_joint(&mut self, joint: i32);
    fn disable_joint(&mut self, joint: i32);
}

/// One block set (maps 5, 7, 17).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockSet {
    pub map: u8,
    pub state: BlocksState,
    /// xC8: drawn on even ticks while blinking (grCastle_801D0520).
    pub blink: bool,
    /// xCC: its standing time, then its blink count.
    pub timer: i32,
}


/// grCastle_801CFBD4's floor of a block's scale.
const SMALL: f32 = 0.0001;

impl BlockSet {
    /// grCastle_801CEACC (0x801CEACC): hidden, its joints disabled
    /// (grCastle_801D0298(gobj, 0) on hidden blocks only disables).
    pub fn new<E: BlockEngine>(map: u8, p: &Parameters, engine: &mut E) -> Self {
        let set = Self {
            map,
            state: BlocksState::Idle,
            blink: false,
            timer: 0,
        };
        set.resize(false, p, engine);
        set
    }

    /// The switch was pressed (grCastle_801CF308 state 4).
    pub fn start(&mut self) {
        self.state = BlocksState::Growing;
    }

    /// grCastle_801CEF04 (0x801CEF04), before its Ground_801C2FE0. The
    /// bounce (grCastle_801D02B8) needs a `?` block's hit, items on only.
    pub fn step<E: BlockEngine>(&mut self, p: &Parameters, rng: &mut HsdRng, engine: &mut E) {
        match self.state {
            BlocksState::Idle => {}
            BlocksState::Growing => {
                self.blink = false;
                self.timer = rand_range(p.blocks_time[1], p.blocks_time[0], rng);
                if self.resize(true, p, engine) {
                    self.state = BlocksState::Standing;
                }
            }
            BlocksState::Standing => {
                let timer = self.timer;
                self.timer -= 1;
                if timer < 0 {
                    // No material item is live with items off.
                    self.state = BlocksState::Blinking;
                    self.timer = 0;
                }
            }
            BlocksState::Blinking => {
                let timer = self.timer;
                self.timer += 1;
                if timer > p.blocks_blink {
                    self.state = BlocksState::Shrinking;
                } else {
                    self.blink = !self.blink;
                }
            }
            BlocksState::Shrinking => {
                self.blink = !self.blink;
                if self.resize(false, p, engine) {
                    self.blink = false;
                    self.state = BlocksState::Idle;
                }
            }
        }
    }

    /// grCastle_801CFBD4 (0x801CFBD4): grow (`grow`) or shrink every block
    /// by one step; true once all are done.
    fn resize<E: BlockEngine>(&self, grow: bool, p: &Parameters, engine: &mut E) -> bool {
        let mut done = true;
        for block in blocks(self.map) {
            // The scale lives on the shown look of a `?` block.
            let target = match block.looks {
                Some((a, b)) if engine.hidden(a) => b,
                Some((a, _)) => a,
                None => block.bone,
            };
            if grow {
                if engine.hidden(block.bone) {
                    if !engine.bill_near(block.bone) {
                        engine.set_hidden(block.bone, false);
                        match block.looks {
                            Some((a, b)) => {
                                let items = engine.items_on();
                                engine.set_hidden(a, !items);
                                engine.set_scale_y(a, if items { SMALL } else { 1.0 });
                                engine.set_hidden(b, items);
                                engine.set_scale_y(b, if items { 1.0 } else { SMALL });
                            }
                            None => engine.set_scale_y(block.bone, SMALL),
                        }
                    }
                    done = false;
                    continue;
                }
                let mut scale = engine.scale_y(target);
                if scale < 1.0 {
                    scale += p.block_grow;
                    if scale >= 1.0 {
                        scale = 1.0;
                        engine.enable_joint(block.joint);
                        if let Some((a, _)) = block.looks {
                            if !engine.hidden(a) {
                                unimplemented!(
                                    "grcastle.c:1608: a `?` block's material item (items on)"
                                );
                            }
                        }
                    }
                    engine.set_scale_y(target, scale);
                    done = false;
                }
            } else {
                if !engine.hidden(block.bone) {
                    let mut scale = engine.scale_y(target);
                    if scale > SMALL {
                        scale -= p.block_shrink;
                        if scale <= SMALL {
                            scale = SMALL;
                            engine.set_hidden(block.bone, true);
                            if let Some((a, b)) = block.looks {
                                engine.set_hidden(a, true);
                                engine.set_hidden(b, true);
                            }
                        }
                        engine.set_scale_y(target, scale);
                        done = false;
                    }
                }
                engine.disable_joint(block.joint);
            }
        }
        done
    }
}
