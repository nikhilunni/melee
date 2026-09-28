//! The jumbotron (map 1, `grStadium_Display` in gr/types.h): what it shows
//! and for how long. Its pictures and text are presentation; its mode timer
//! and the mode choice draw from the shared RNG, and its camera subject
//! pulls the match camera toward the screen while a transformation is
//! announced.
use super::Parameters;
use crate::rand::range;
use gekko_math::HsdRng;

/// `grStadium_Display::xE4`, the mode `grStadium_801D2528` selects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenMode {
    /// Text cleared (`grStadium_801D4194`).
    Blank = 0,
    /// Player names and the clock, or the timer (`grStadium_801D39A0`).
    MatchInfo = 1,
    /// Transformation announcements, one per destination form.
    AnnounceBase = 2,
    AnnounceFire = 3,
    AnnounceGrass = 4,
    AnnounceWater = 5,
    AnnounceRock = 6,
    /// The stage camera feed.
    StageCamera = 7,
    /// A close-up of one player (`xEE`).
    PlayerCamera = 8,
    /// "Player N Defeated" (`grStadium_801D3A0C`, from gm_8016B8D4).
    PlayerDefeated = 9,
    /// Static messages (`grStadium_801D3F40`): the Versus countdown start,
    /// its end, the match end and the Sudden Death countdown start.
    CountdownStart = 10,
    CountdownEnd = 11,
    MatchEnd = 12,
    SuddenDeathStart = 13,
    /// Remaining players in standing order (`grStadium_801D3BBC`).
    Standings = 14,
    /// Picture layers 0x20 and 0x80 (TODO(meaning)).
    Picture15 = 15,
    Picture16 = 16,
    /// Clears both text windows.
    ClearText = 17,
}
impl TryFrom<i16> for ScreenMode {
    type Error = i16;
    fn try_from(value: i16) -> Result<Self, i16> {
        use ScreenMode::*;
        Ok(match value {
            0 => Blank,
            1 => MatchInfo,
            2 => AnnounceBase,
            3 => AnnounceFire,
            4 => AnnounceGrass,
            5 => AnnounceWater,
            6 => AnnounceRock,
            7 => StageCamera,
            8 => PlayerCamera,
            9 => PlayerDefeated,
            10 => CountdownStart,
            11 => CountdownEnd,
            12 => MatchEnd,
            13 => SuddenDeathStart,
            14 => Standings,
            15 => Picture15,
            16 => Picture16,
            17 => ClearText,
            _ => return Err(value),
        })
    }
}

/// `grStadium_Display::xEE` before any player has been shown.
pub const NO_FOCUS: i16 = 99;
/// Player slots `grStadium_801D2528` scans for the close-up.
const PLAYER_SLOTS: i16 = 6;

/// What the screen asks of the players it looks at.
pub trait ScreenPlayers {
    /// `Player_GetEntity(slot) != NULL`.
    fn exists(&self, slot: i16) -> bool;
    /// `Player_8003219C`: the fighter's x221F_b3 (its procs are suppressed).
    fn suppressed(&self, slot: i16) -> bool;
    /// `grStadium_801D32D0`: the close-up box around the player's camera
    /// bone lies inside the viewport.
    fn framed(&mut self, slot: i16) -> bool;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Screen {
    /// xE4.
    pub mode: ScreenMode,
    /// xEA; `None` is retail's -1 before the first change.
    pub previous: Option<ScreenMode>,
    /// xE0: ticks left in a timed mode, post-decremented.
    pub timer: i32,
    /// xEE: the player of the close-up.
    pub focus: i16,
    /// xF2: modes chosen since the last standings.
    pub cycles: i16,
    /// xF4's state: `None` until Ground_801C0FB8 creates the subject
    /// (fn_801D11E4), then whether it is active.
    pub subject_active: Option<bool>,
}

impl Screen {
    /// `grStadium_801D2278` (0x801D2278): mode 1 is written directly, then
    /// `grStadium_801D2528(gobj, 0, 0)` blanks it.
    pub fn new(parameters: &Parameters, rng: &mut HsdRng) -> Self {
        let mut screen = Self {
            mode: ScreenMode::MatchInfo,
            previous: None,
            timer: 0,
            focus: NO_FOCUS,
            cycles: 0,
            subject_active: None,
        };
        screen.set_mode(ScreenMode::Blank, 0, parameters, rng, &mut NoPlayers);
        screen
    }

    /// `grStadium_801D2528` (0x801D2528) outside Training mode: the timer,
    /// focus and camera-subject effects of entering `mode`. A nonzero
    /// `timer` overrides the mode's own.
    pub fn set_mode(
        &mut self,
        mode: ScreenMode,
        timer: i32,
        parameters: &Parameters,
        rng: &mut HsdRng,
        players: &mut impl ScreenPlayers,
    ) {
        use ScreenMode::*;
        self.previous = Some(self.mode);
        self.mode = mode;
        let mut subject = Some(false);
        match mode {
            Blank | CountdownStart | CountdownEnd | MatchEnd | SuddenDeathStart => {}
            MatchInfo | Picture15 | Picture16 => self.timer = parameters.info_frames,
            PlayerDefeated => self.timer = parameters.defeat_frames,
            Standings => self.timer = parameters.standings_frames,
            StageCamera => self.timer = range(rng, parameters.stage_camera_frames),
            PlayerCamera => {
                self.timer = range(rng, parameters.player_camera_frames);
                self.next_focus(players);
            }
            // grpstadium.c:1079-1086 leaves the subject as it was.
            ClearText => subject = None,
            AnnounceBase | AnnounceFire | AnnounceGrass | AnnounceWater | AnnounceRock => {
                self.timer = parameters.announce_frames;
                subject = Some(true);
            }
        }
        if let (Some(active), Some(state)) = (subject, self.subject_active.as_mut()) {
            *state = active;
        }
        if timer != 0 {
            self.timer = timer;
        }
    }

    /// grpstadium.c:1059-1072: the next existing player after the current
    /// focus, wrapping at six slots; after two wraps the timer expires.
    fn next_focus(&mut self, players: &impl ScreenPlayers) {
        let start = self.focus;
        self.focus = self.focus.wrapping_add(1);
        let mut wraps = 0;
        while self.focus != start {
            if self.focus >= PLAYER_SLOTS {
                self.focus = 0;
                wraps += 1;
                if wraps > 2 {
                    self.focus = 0;
                    self.timer = -1;
                    break;
                }
            }
            if players.exists(self.focus) {
                break;
            }
            self.focus += 1;
        }
    }

    /// Post-decrement test shared by the timed modes: a timer of zero still
    /// shows one more tick.
    fn expire(&mut self) -> bool {
        let old = self.timer;
        self.timer -= 1;
        old < 0
    }

    /// `grStadium_801D2344` (0x801D2344), the screen's per-tick proc after
    /// the audience flash. Text and picture updates are presentation.
    pub fn tick(
        &mut self,
        parameters: &Parameters,
        rng: &mut HsdRng,
        players: &mut impl ScreenPlayers,
    ) {
        use ScreenMode::*;
        match self.mode {
            Blank | CountdownStart | CountdownEnd | MatchEnd | SuddenDeathStart | ClearText => {}
            MatchInfo | PlayerDefeated | Standings | AnnounceBase | AnnounceFire
            | AnnounceGrass | AnnounceWater | AnnounceRock | Picture15 | Picture16
            | StageCamera => {
                if self.expire() {
                    self.advance(parameters, rng, players);
                }
            }
            PlayerCamera => {
                // grpstadium.c:898-912: the timer, then the player, then the
                // framing; each failure moves on (the second checks repeat).
                let lost = self.expire()
                    || !players.exists(self.focus)
                    || players.suppressed(self.focus)
                    || !players.framed(self.focus);
                if lost {
                    self.advance(parameters, rng, players);
                }
            }
        }
    }

    /// `grStadium_801D2A60` (0x801D2A60): after `standings_interval` choices
    /// show the standings, else draw a weighted mode different from the
    /// current and previous ones. No multiply-add: 801D2B04 fmuls,
    /// 801D2B08 fsubs, each weight converted from int then fsubs.
    fn advance(
        &mut self,
        parameters: &Parameters,
        rng: &mut HsdRng,
        players: &mut impl ScreenPlayers,
    ) {
        let w = &parameters.mode_weights;
        let mode = if self.cycles >= parameters.standings_interval {
            self.cycles = 0;
            ScreenMode::Standings
        } else {
            loop {
                let total = i32::from(w.player_camera)
                    + (i32::from(w.match_info) + i32::from(w.stage_camera) + i32::from(w.picture));
                let mut value = rng.randf() * total as f32 - f32::from(w.player_camera);
                let mode = if value < 0.0 {
                    ScreenMode::PlayerCamera
                } else {
                    value -= f32::from(w.stage_camera);
                    if value < 0.0 {
                        ScreenMode::StageCamera
                    } else {
                        value -= f32::from(w.match_info);
                        if value < 0.0 {
                            ScreenMode::MatchInfo
                        } else if value - f32::from(w.picture) < 0.0 {
                            ScreenMode::Picture15
                        } else {
                            ScreenMode::MatchInfo
                        }
                    }
                };
                if mode != self.mode && Some(mode) != self.previous {
                    break mode;
                }
            }
        };
        self.cycles += 1;
        self.set_mode(mode, 0, parameters, rng, players);
    }
}

/// Before any fighter exists (Ground creation).
struct NoPlayers;
impl ScreenPlayers for NoPlayers {
    fn exists(&self, _: i16) -> bool {
        false
    }
    fn suppressed(&self, _: i16) -> bool {
        false
    }
    fn framed(&mut self, _: i16) -> bool {
        false
    }
}
