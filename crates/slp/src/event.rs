//! Event payload layouts from SPEC.md. Every `parse` takes the whole event
//! including its command byte, so the offsets below are the spec's offsets
//! verbatim. Fields that a later Slippi version appended are `Option` and
//! are `None` when the payload is too short to hold them.

use anyhow::{bail, Result};

pub const CMD_MESSAGE_SPLITTER: u8 = 0x10;
pub const CMD_EVENT_PAYLOADS: u8 = 0x35;
pub const CMD_GAME_START: u8 = 0x36;
pub const CMD_PRE_FRAME: u8 = 0x37;
pub const CMD_POST_FRAME: u8 = 0x38;
pub const CMD_GAME_END: u8 = 0x39;
pub const CMD_FRAME_START: u8 = 0x3A;
pub const CMD_ITEM_UPDATE: u8 = 0x3B;
pub const CMD_FRAME_BOOKEND: u8 = 0x3C;
pub const CMD_GECKO_LIST: u8 = 0x3D;

/// Big-endian field reader over one event. Reads past the end yield `None`.
struct Payload<'a>(&'a [u8]);

impl Payload<'_> {
    fn bytes<const N: usize>(&self, off: usize) -> Option<[u8; N]> {
        self.0.get(off..off + N)?.try_into().ok()
    }
    fn u8(&self, off: usize) -> Option<u8> {
        self.0.get(off).copied()
    }
    fn i8(&self, off: usize) -> Option<i8> {
        self.u8(off).map(|b| b as i8)
    }
    fn bool(&self, off: usize) -> Option<bool> {
        self.u8(off).map(|b| b != 0)
    }
    fn u16(&self, off: usize) -> Option<u16> {
        self.bytes(off).map(u16::from_be_bytes)
    }
    fn u32(&self, off: usize) -> Option<u32> {
        self.bytes(off).map(u32::from_be_bytes)
    }
    fn i32(&self, off: usize) -> Option<i32> {
        self.bytes(off).map(i32::from_be_bytes)
    }
    fn f32(&self, off: usize) -> Option<f32> {
        self.bytes(off).map(f32::from_be_bytes)
    }
}

/// Fields required since 0.1.0 are read with this; a short payload is an error.
macro_rules! req {
    ($p:expr, $name:expr, $m:ident, $off:expr) => {
        match $p.$m($off) {
            Some(v) => v,
            None => bail!("{}: payload too short for field at 0x{:X}", $name, $off),
        }
    };
}

/// Slippi extraction-code version from Game Start (`major.minor.build`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Version {
    pub major: u8,
    pub minor: u8,
    pub build: u8,
}

impl Version {
    pub const fn new(major: u8, minor: u8, build: u8) -> Self {
        Version {
            major,
            minor,
            build,
        }
    }
    pub fn at_least(self, major: u8, minor: u8, build: u8) -> bool {
        self >= Version::new(major, minor, build)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.build)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlayerType {
    Human,
    Cpu,
    Demo,
    Empty,
}

impl PlayerType {
    fn from_u8(v: u8) -> Result<Self> {
        Ok(match v {
            0 => PlayerType::Human,
            1 => PlayerType::Cpu,
            2 => PlayerType::Demo,
            3 => PlayerType::Empty,
            _ => bail!("Game Start: unknown player type {v}"),
        })
    }
}

/// Per-port settings from the Game Info Block (`0x60 + 0x24*i`).
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerStart {
    /// 0-based port index (port number minus one).
    pub port: u8,
    /// External character id; see `ids::external_character_name`.
    pub character: u8,
    pub player_type: PlayerType,
    pub stock_start_count: u8,
    /// Costume index ("character colour"). Not a colour per se; the mapping
    /// to an actual palette is character-specific.
    pub costume: u8,
    /// StartMeleeData PlayerInitData.x5: -1 uses the slot's stage marker.
    pub spawn_point: i8,
    pub team_shade: u8,
    pub handicap: u8,
    pub team_id: u8,
    pub player_bitfield: u8,
    pub cpu_level: u8,
    pub damage_start: u16,
    pub damage_spawn: u16,
    pub offense_ratio: f32,
    pub defense_ratio: f32,
    pub model_scale: f32,
    /// Controller fixes (1.0.0+): 0 = off, 1 = UCF, 2 = Dween.
    pub dashback_fix: Option<u32>,
    pub shield_drop_fix: Option<u32>,
    /// Raw Shift-JIS nametag bytes (1.3.0+), 16 bytes, not decoded.
    pub nametag: Option<[u8; 16]>,
}

impl PlayerStart {
    pub fn is_present(&self) -> bool {
        self.player_type != PlayerType::Empty
    }
}

/// Game Start (0x36). Offsets below are into the whole event; Game Info
/// Block offsets are `0x5 + block offset`.
#[derive(Debug, Clone, PartialEq)]
pub struct GameStart {
    pub version: Version,
    pub game_bitfields: [u8; 4],
    pub bomb_rain: u8,
    pub is_teams: bool,
    pub item_spawn_behavior: i8,
    pub self_destruct_score_value: i8,
    pub stage: u16,
    pub game_timer: u32,
    pub item_spawn_bitfields: [u8; 5],
    pub damage_ratio: f32,
    pub game_speed: f32,
    pub players: [PlayerStart; 4],
    /// "The random seed before the game start" (0x13D).
    pub random_seed: u32,
    pub pal: Option<bool>,
    pub frozen_ps: Option<bool>,
    pub minor_scene: Option<u8>,
    /// 0x2 for VS mode, 0x8 for Slippi Online (rollback) games.
    pub major_scene: Option<u8>,
    pub language: Option<u8>,
}

impl GameStart {
    /// Timer behaviour from Game Bitfield 1 bits 1-2: 0 none, 2 down, 3 up.
    pub fn timer_type(&self) -> u8 {
        self.game_bitfields[0] & 0x03
    }
    /// Game mode from Game Bitfield 1 bits 6-8: 0 time, 1 stock, 2 coin, 3 bonus.
    pub fn game_mode(&self) -> u8 {
        (self.game_bitfields[0] & 0xE0) >> 5
    }
    pub fn is_online(&self) -> bool {
        self.major_scene == Some(0x8)
    }

    pub fn parse(ev: &[u8]) -> Result<GameStart> {
        let p = Payload(ev);
        const N: &str = "Game Start";
        let version = Version {
            major: req!(p, N, u8, 0x1),
            minor: req!(p, N, u8, 0x2),
            build: req!(p, N, u8, 0x3),
        };
        const B: usize = 0x5; // Game Info Block base
        let mut players = Vec::with_capacity(4);
        for i in 0..4usize {
            let o = B + 0x60 + 0x24 * i;
            players.push(PlayerStart {
                port: i as u8,
                character: req!(p, N, u8, o),
                player_type: PlayerType::from_u8(req!(p, N, u8, o + 1))?,
                stock_start_count: req!(p, N, u8, o + 2),
                costume: req!(p, N, u8, o + 3),
                spawn_point: req!(p, N, i8, o + 5),
                team_shade: req!(p, N, u8, o + 7),
                handicap: req!(p, N, u8, o + 8),
                team_id: req!(p, N, u8, o + 9),
                player_bitfield: req!(p, N, u8, o + 0xC),
                cpu_level: req!(p, N, u8, o + 0xF),
                damage_start: req!(p, N, u16, o + 0x10),
                damage_spawn: req!(p, N, u16, o + 0x12),
                offense_ratio: req!(p, N, f32, o + 0x18),
                defense_ratio: req!(p, N, f32, o + 0x1C),
                model_scale: req!(p, N, f32, o + 0x20),
                dashback_fix: p.u32(0x141 + 0x8 * i),
                shield_drop_fix: p.u32(0x145 + 0x8 * i),
                nametag: p.bytes::<16>(0x161 + 0x10 * i),
            });
        }
        let players: [PlayerStart; 4] = players.try_into().expect("four players");
        Ok(GameStart {
            version,
            game_bitfields: req!(p, N, bytes, B),
            bomb_rain: req!(p, N, u8, B + 0x6),
            is_teams: req!(p, N, bool, B + 0x8),
            item_spawn_behavior: req!(p, N, i8, B + 0xB),
            self_destruct_score_value: req!(p, N, i8, B + 0xC),
            stage: req!(p, N, u16, B + 0xE),
            game_timer: req!(p, N, u32, B + 0x10),
            item_spawn_bitfields: req!(p, N, bytes, B + 0x23),
            damage_ratio: req!(p, N, f32, B + 0x30),
            game_speed: req!(p, N, f32, B + 0x34),
            players,
            random_seed: req!(p, N, u32, 0x13D),
            pal: p.bool(0x1A1),
            frozen_ps: p.bool(0x1A2),
            minor_scene: p.u8(0x1A3),
            major_scene: p.u8(0x1A4),
            language: p.u8(0x2BD),
        })
    }
}

/// Pre Frame Update (0x37): controller state right before it is applied.
#[derive(Debug, Clone, PartialEq)]
pub struct PreFrame {
    pub frame: i32,
    pub player_index: u8,
    pub is_follower: bool,
    /// The RNG seed "at this point", i.e. before this port's input is processed.
    pub random_seed: u32,
    pub action_state: u16,
    pub position_x: f32,
    pub position_y: f32,
    pub facing_direction: f32,
    /// Processed (dead-zoned, scaled) main stick, [-1, 1].
    pub joystick_x: f32,
    pub joystick_y: f32,
    pub cstick_x: f32,
    pub cstick_y: f32,
    /// Processed trigger, [0, 1].
    pub trigger: f32,
    /// Fighter.input held word (+0x65C), including HSD directions and the
    /// fighter's Z->A+shield and digital-shoulder macros. Low bits therefore
    /// need not equal buttons_physical (MasterStatus, a different pad phase).
    pub buttons_processed: u32,
    pub buttons_physical: u16,
    pub physical_l_trigger: f32,
    pub physical_r_trigger: f32,
    /// Raw stick X (1.2.0+), used by UCF.
    pub raw_joystick_x: Option<i8>,
    pub percent: Option<f32>,
    /// 3.15.0+
    pub raw_joystick_y: Option<i8>,
    /// 3.17.0+
    pub raw_cstick_x: Option<i8>,
    pub raw_cstick_y: Option<i8>,
}

impl PreFrame {
    pub fn parse(ev: &[u8]) -> Result<PreFrame> {
        let p = Payload(ev);
        const N: &str = "Pre Frame Update";
        Ok(PreFrame {
            frame: req!(p, N, i32, 0x1),
            player_index: req!(p, N, u8, 0x5),
            is_follower: req!(p, N, bool, 0x6),
            random_seed: req!(p, N, u32, 0x7),
            action_state: req!(p, N, u16, 0xB),
            position_x: req!(p, N, f32, 0xD),
            position_y: req!(p, N, f32, 0x11),
            facing_direction: req!(p, N, f32, 0x15),
            joystick_x: req!(p, N, f32, 0x19),
            joystick_y: req!(p, N, f32, 0x1D),
            cstick_x: req!(p, N, f32, 0x21),
            cstick_y: req!(p, N, f32, 0x25),
            trigger: req!(p, N, f32, 0x29),
            buttons_processed: req!(p, N, u32, 0x2D),
            buttons_physical: req!(p, N, u16, 0x31),
            physical_l_trigger: req!(p, N, f32, 0x33),
            physical_r_trigger: req!(p, N, f32, 0x37),
            raw_joystick_x: p.i8(0x3B),
            percent: p.f32(0x3C),
            raw_joystick_y: p.i8(0x40),
            raw_cstick_x: p.i8(0x41),
            raw_cstick_y: p.i8(0x42),
        })
    }
}

/// Post Frame Update (0x38): fighter state after collision, end of frame.
#[derive(Debug, Clone, PartialEq)]
pub struct PostFrame {
    pub frame: i32,
    pub player_index: u8,
    pub is_follower: bool,
    /// Internal character id (`Fighter.kind`); see `ids::internal_character_name`.
    pub internal_character: u8,
    /// Action state / motion id (`Fighter.motion_id`).
    pub action_state: u16,
    pub position_x: f32,
    pub position_y: f32,
    pub facing_direction: f32,
    pub percent: f32,
    pub shield_size: f32,
    pub last_hitting_attack: u8,
    pub combo_count: u8,
    pub last_hit_by: u8,
    pub stocks: u8,
    /// 0.2.0+. Frames the action state has been active; may be fractional.
    pub action_state_frame: Option<f32>,
    /// 2.0.0+
    pub state_flags: Option<[u8; 5]>,
    /// 2.0.0+. Hitstun frames remaining while in hitstun; otherwise reused.
    pub misc_action_state: Option<f32>,
    /// 2.0.0+. false = grounded, true = airborne.
    pub airborne: Option<bool>,
    pub last_ground_id: Option<u16>,
    pub jumps_remaining: Option<u8>,
    /// 2.0.0+. 0 none, 1 successful, 2 unsuccessful.
    pub l_cancel_status: Option<u8>,
    /// 2.1.0+. 0 vulnerable, 1 invulnerable, 2 intangible.
    pub hurtbox_state: Option<u8>,
    /// 3.5.0+
    pub self_air_speed_x: Option<f32>,
    pub self_speed_y: Option<f32>,
    pub attack_speed_x: Option<f32>,
    pub attack_speed_y: Option<f32>,
    pub self_ground_speed_x: Option<f32>,
    /// 3.8.0+
    pub hitlag_remaining: Option<f32>,
    /// 3.11.0+
    pub animation_index: Option<u32>,
    /// 3.16.0+
    pub instance_hit_by: Option<u16>,
    pub instance_id: Option<u16>,
}

impl PostFrame {
    pub fn parse(ev: &[u8]) -> Result<PostFrame> {
        let p = Payload(ev);
        const N: &str = "Post Frame Update";
        Ok(PostFrame {
            frame: req!(p, N, i32, 0x1),
            player_index: req!(p, N, u8, 0x5),
            is_follower: req!(p, N, bool, 0x6),
            internal_character: req!(p, N, u8, 0x7),
            action_state: req!(p, N, u16, 0x8),
            position_x: req!(p, N, f32, 0xA),
            position_y: req!(p, N, f32, 0xE),
            facing_direction: req!(p, N, f32, 0x12),
            percent: req!(p, N, f32, 0x16),
            shield_size: req!(p, N, f32, 0x1A),
            last_hitting_attack: req!(p, N, u8, 0x1E),
            combo_count: req!(p, N, u8, 0x1F),
            last_hit_by: req!(p, N, u8, 0x20),
            stocks: req!(p, N, u8, 0x21),
            action_state_frame: p.f32(0x22),
            state_flags: p.bytes::<5>(0x26),
            misc_action_state: p.f32(0x2B),
            airborne: p.bool(0x2F),
            last_ground_id: p.u16(0x30),
            jumps_remaining: p.u8(0x32),
            l_cancel_status: p.u8(0x33),
            hurtbox_state: p.u8(0x34),
            self_air_speed_x: p.f32(0x35),
            self_speed_y: p.f32(0x39),
            attack_speed_x: p.f32(0x3D),
            attack_speed_y: p.f32(0x41),
            self_ground_speed_x: p.f32(0x45),
            hitlag_remaining: p.f32(0x49),
            animation_index: p.u32(0x4D),
            instance_hit_by: p.u16(0x51),
            instance_id: p.u16(0x53),
        })
    }

    /// State Bit Flags 4 bit 2: in hitstun.
    pub fn in_hitstun(&self) -> Option<bool> {
        self.state_flags.map(|f| f[3] & 0x02 != 0)
    }
    /// Hitstun frames remaining, only meaningful while [`Self::in_hitstun`].
    pub fn hitstun_remaining(&self) -> Option<f32> {
        match self.in_hitstun() {
            Some(true) => self.misc_action_state,
            _ => None,
        }
    }
}

/// Frame Start (0x3A), 2.2.0+.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameStart {
    pub frame: i32,
    /// The RNG seed at the very beginning of the frame's processing.
    pub random_seed: u32,
    /// 3.10.0+. Counts even while paused.
    pub scene_frame_counter: Option<u32>,
}

impl FrameStart {
    pub fn parse(ev: &[u8]) -> Result<FrameStart> {
        let p = Payload(ev);
        const N: &str = "Frame Start";
        Ok(FrameStart {
            frame: req!(p, N, i32, 0x1),
            random_seed: req!(p, N, u32, 0x5),
            scene_frame_counter: p.u32(0x9),
        })
    }
}

/// Frame Bookend (0x3C), 3.0.0+.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameBookend {
    pub frame: i32,
    /// 3.7.0+. For rollback games, the newest frame guaranteed not to be
    /// re-simulated.
    pub latest_finalized_frame: Option<i32>,
}

impl FrameBookend {
    pub fn parse(ev: &[u8]) -> Result<FrameBookend> {
        let p = Payload(ev);
        const N: &str = "Frame Bookend";
        Ok(FrameBookend {
            frame: req!(p, N, i32, 0x1),
            latest_finalized_frame: p.i32(0x5),
        })
    }
}

/// Item Update (0x3B), 3.0.0+. Includes projectiles.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemUpdate {
    pub frame: i32,
    pub type_id: u16,
    pub state: u8,
    pub facing_direction: f32,
    pub velocity_x: f32,
    pub velocity_y: f32,
    pub position_x: f32,
    pub position_y: f32,
    pub damage_taken: u16,
    pub expiration_timer: f32,
    pub spawn_id: u32,
    /// 3.2.0+
    pub misc: Option<[u8; 4]>,
    /// 3.6.0+. 0-3, or -1 when unowned.
    pub owner: Option<i8>,
    /// 3.16.0+
    pub instance_id: Option<u16>,
}

impl ItemUpdate {
    pub fn parse(ev: &[u8]) -> Result<ItemUpdate> {
        let p = Payload(ev);
        const N: &str = "Item Update";
        Ok(ItemUpdate {
            frame: req!(p, N, i32, 0x1),
            type_id: req!(p, N, u16, 0x5),
            state: req!(p, N, u8, 0x7),
            facing_direction: req!(p, N, f32, 0x8),
            velocity_x: req!(p, N, f32, 0xC),
            velocity_y: req!(p, N, f32, 0x10),
            position_x: req!(p, N, f32, 0x14),
            position_y: req!(p, N, f32, 0x18),
            damage_taken: req!(p, N, u16, 0x1C),
            expiration_timer: req!(p, N, f32, 0x1E),
            spawn_id: req!(p, N, u32, 0x22),
            misc: p.bytes::<4>(0x26),
            owner: p.i8(0x2A),
            instance_id: p.u16(0x2B),
        })
    }
}

/// Game End (0x39).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameEnd {
    /// Pre-2.0.0: 0 unresolved, 3 resolved. 2.0.0+: 1 TIME!, 2 GAME!, 7 No Contest.
    pub method: u8,
    /// 2.0.0+. Port index of the player who L+R+A+Started, or -1.
    pub lras_initiator: Option<i8>,
    /// 3.13.0+. 0-based placements per port, -1 if not in game.
    pub placements: Option<[i8; 4]>,
}

impl GameEnd {
    pub fn parse(ev: &[u8]) -> Result<GameEnd> {
        let p = Payload(ev);
        const N: &str = "Game End";
        Ok(GameEnd {
            method: req!(p, N, u8, 0x1),
            lras_initiator: p.i8(0x2),
            placements: p.bytes::<4>(0x3).map(|b| b.map(|x| x as i8)),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_start_without_scene_counter() {
        let mut ev = vec![CMD_FRAME_START];
        ev.extend((-123i32).to_be_bytes());
        ev.extend(0xDEADBEEFu32.to_be_bytes());
        let fs = FrameStart::parse(&ev).unwrap();
        assert_eq!(fs.frame, -123);
        assert_eq!(fs.random_seed, 0xDEADBEEF);
        assert_eq!(fs.scene_frame_counter, None);
    }

    #[test]
    fn short_required_field_is_an_error() {
        let ev = [CMD_FRAME_START, 0, 0];
        assert!(FrameStart::parse(&ev).is_err());
    }

    #[test]
    fn version_ordering() {
        assert!(Version::new(3, 12, 0).at_least(2, 2, 0));
        assert!(!Version::new(0, 1, 0).at_least(0, 2, 0));
        assert_eq!(Version::new(3, 12, 0).to_string(), "3.12.0");
    }
}
