//! The application's flow, shared by every host: which screen is up, what
//! the players picked, what is loading and the match itself. Hosts draw the
//! screens natively and forward user intent here; they hold no game logic.
//!
//! ```text
//! Disc -> Characters -> Stages -> Loading -> Match -> Results
//!  ^         |  ^          |         |         |        |
//!  +---------+  +----------+---------+ (error) |        |
//!               +--------------------- quit ---+--------+ continue
//! ```
use crate::{
    art::{Art, ArtFile, Image, Piece},
    disc::{DiscFiles, FileRequest},
    preview::{self, Previews},
    session::{Action, Session},
};
use std::sync::Arc;
use melee_lib::{
    Character, Costume, MatchConfig, MatchOutcome, MatchStatus, PlayerConfig, Port, Seed, Stage,
};
use std::{collections::VecDeque, time::Duration};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum Screen {
    /// No disc yet: ask for the Melee image.
    Disc = 0,
    Characters = 1,
    Stages = 2,
    /// Fetching the match's files from the disc, then building the match.
    Loading = 3,
    Match = 4,
    Results = 5,
}

/// One player's character select cursor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Slot {
    pub character: Option<Character>,
    pub costume: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LoadProgress {
    pub files_done: u32,
    pub files_total: u32,
    pub bytes_done: u64,
    pub bytes_total: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerHud {
    pub port: Port,
    pub character: Character,
    pub costume: u8,
    pub percent: f32,
    pub stocks: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hud {
    pub tick: u64,
    pub paused: bool,
    pub players: [PlayerHud; 2],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Index into the match's two players.
    Winner(usize),
    Draw,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Results {
    pub outcome: Outcome,
    pub hud: Hud,
}

struct Loading {
    config: MatchConfig,
    pending: VecDeque<FileRequest>,
    progress: LoadProgress,
}

pub const DEFAULT_STOCKS: u8 = 4;

pub struct App {
    screen: Screen,
    disc: Option<DiscFiles>,
    /// Menu art from the open disc's menu archives.
    art: Art,
    /// The stage previews rendered from the open disc.
    previews: Previews,
    /// Whether a host took the preview job for this disc.
    previews_started: bool,
    slots: [Slot; 2],
    stocks: u8,
    loading: Option<Loading>,
    /// The last match's configuration, for a rematch.
    last_config: Option<MatchConfig>,
    session: Option<Session>,
    results: Option<Results>,
    notice: Option<String>,
    /// Counts sessions started, so a host knows when to upload a new scene.
    generation: u64,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

fn wrong_screen(action: &str, screen: Screen) -> String {
    format!("cannot {action} on the {screen:?} screen")
}

impl App {
    pub fn new() -> Self {
        Self {
            screen: Screen::Disc,
            disc: None,
            art: Art::new(),
            previews: Previews::default(),
            previews_started: false,
            slots: [Slot::default(); 2],
            stocks: DEFAULT_STOCKS,
            loading: None,
            last_config: None,
            session: None,
            results: None,
            notice: None,
            generation: 0,
        }
    }
    pub fn screen(&self) -> Screen {
        self.screen
    }
    fn expect(&self, screen: Screen, action: &str) -> Result<(), String> {
        if self.screen == screen {
            Ok(())
        } else {
            Err(wrong_screen(action, self.screen))
        }
    }

    // --- Messages for the host ------------------------------------------

    /// The last error or fault the host should show (load failure, a match
    /// fault with its replay path). Stays until taken.
    pub fn notice(&self) -> Option<&str> {
        self.notice.as_deref()
    }
    pub fn take_notice(&mut self) -> Option<String> {
        self.notice.take()
    }

    // --- Disc -----------------------------------------------------------

    /// Use a validated disc; the menus open on character select. A new disc
    /// replaces the old one, its fetched files and its art; the host then
    /// reads [`App::art_requests`] (natively, [`App::load_art`]).
    pub fn open_disc(&mut self, disc: DiscFiles) -> Result<(), String> {
        if matches!(self.screen, Screen::Loading | Screen::Match) {
            return Err(wrong_screen("change discs", self.screen));
        }
        self.disc = Some(disc);
        self.art = Art::new();
        self.previews = Previews::default();
        self.previews_started = false;
        self.results = None;
        self.screen = Screen::Characters;
        Ok(())
    }
    pub fn disc(&self) -> Option<&DiscFiles> {
        self.disc.as_ref()
    }

    // --- Menu art -----------------------------------------------------------

    /// The menu archives still to read from the open disc, then the files
    /// the stage previews need ([`preview::files`], about 16 MB), on any
    /// screen. Hosts hand them over with [`App::provide_file`]; menus show
    /// placeholders until [`App::art_ready`] (the menu archives) and
    /// [`App::stage_previews_ready`].
    pub fn art_requests(&self) -> Vec<FileRequest> {
        let Some(disc) = &self.disc else {
            return Vec::new();
        };
        let mut requests: Vec<_> = self
            .art
            .missing()
            .into_iter()
            .filter_map(|file| disc.request(file.name()).ok())
            .collect();
        for name in preview::files() {
            if !disc.is_cached(name) && !requests.iter().any(|r| r.name == name) {
                requests.extend(disc.request(name).ok());
            }
        }
        requests
    }
    /// Whether every menu archive is in.
    pub fn art_ready(&self) -> bool {
        self.disc.is_some() && self.art.is_ready()
    }
    /// Menu archives read so far, out of all of them.
    pub fn art_progress(&self) -> LoadProgress {
        let Some(disc) = &self.disc else {
            return LoadProgress::default();
        };
        let mut progress = LoadProgress::default();
        for file in ArtFile::ALL {
            let Ok(request) = disc.request(file.name()) else {
                continue;
            };
            progress.files_total += 1;
            progress.bytes_total += request.len();
            if self.art.has(file) {
                progress.files_done += 1;
                progress.bytes_done += request.len();
            }
        }
        progress
    }
    /// Natively the core reads the menu archives itself, all at once
    /// (about 5 MB). Art is optional: a failure leaves placeholders and is
    /// returned, not raised as a notice.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_art(&mut self) -> Result<(), String> {
        for request in self.art_requests() {
            let disc = self.disc.as_mut().ok_or("no disc is open")?;
            let bytes = disc.read_native(&request)?;
            self.provide_file(request.name, bytes)?;
        }
        Ok(())
    }
    /// One piece of menu art, decoded on first use. Fails (with the reason)
    /// while its archive is still loading or when retail has no such image.
    pub fn art_image(&mut self, piece: Piece) -> Result<Arc<Image>, String> {
        if self.disc.is_none() {
            return Err("no disc is open".into());
        }
        match piece {
            Piece::StagePreview(stage) => self.previews.image(stage),
            _ => self.art.image(piece),
        }
    }

    // --- Stage previews -----------------------------------------------------

    /// Whether every stage preview is settled (rendered, or failed and left
    /// to the host's placeholder).
    pub fn stage_previews_ready(&self) -> bool {
        self.disc.is_some() && self.previews.ready()
    }
    pub fn stage_previews(&self) -> &Previews {
        &self.previews
    }
    /// The previews' work, once per disc, when their files are in. The host
    /// binding runs it on its window's GPU ([`preview::spawn`] natively,
    /// [`preview::run`] on the web).
    pub fn stage_preview_job(&mut self) -> Option<preview::Job> {
        if self.previews_started {
            return None;
        }
        let files = self.disc.as_ref()?.copy_cached(&preview::files())?;
        self.previews_started = true;
        Some(preview::Job {
            files,
            previews: self.previews.clone(),
        })
    }
    /// The art store itself, for its per-piece helpers.
    pub fn art(&mut self) -> &mut Art {
        &mut self.art
    }
    fn provide_art(&mut self, file: ArtFile, bytes: &[u8]) -> Result<(), String> {
        let disc = self.disc.as_ref().ok_or("no disc is open")?;
        disc.check_len(file.name(), bytes.len())?;
        self.art.insert(file, bytes)
    }

    // --- Character select -----------------------------------------------

    pub fn slots(&self) -> [Slot; 2] {
        self.slots
    }
    pub fn stocks(&self) -> u8 {
        self.stocks
    }
    fn taken_by_other(&self, player: usize, character: Character, costume: u8) -> bool {
        let other = self.slots[1 - player];
        other.character == Some(character) && other.costume == costume
    }
    fn check_player(player: usize) -> Result<(), String> {
        if player < 2 {
            Ok(())
        } else {
            Err(format!("player {} does not exist", player + 1))
        }
    }
    /// Pick a character; the costume becomes the first one the other player
    /// is not wearing (retail Versus never shows two identical fighters).
    pub fn choose_character(&mut self, player: usize, character: Character) -> Result<(), String> {
        self.expect(Screen::Characters, "choose a character")?;
        Self::check_player(player)?;
        let costume = (0..character.costume_count())
            .find(|&c| !self.taken_by_other(player, character, c))
            .unwrap_or(0);
        self.slots[player] = Slot {
            character: Some(character),
            costume,
        };
        Ok(())
    }
    /// Clear a player's pick.
    pub fn unchoose_character(&mut self, player: usize) -> Result<(), String> {
        self.expect(Screen::Characters, "clear a character")?;
        Self::check_player(player)?;
        self.slots[player] = Slot::default();
        Ok(())
    }
    pub fn set_costume(&mut self, player: usize, costume: u8) -> Result<(), String> {
        self.expect(Screen::Characters, "change costumes")?;
        Self::check_player(player)?;
        let character = self.slots[player]
            .character
            .ok_or("choose a character before a costume")?;
        if costume >= character.costume_count() {
            return Err(format!(
                "{} has {} costumes",
                character.display_name(),
                character.costume_count()
            ));
        }
        if self.taken_by_other(player, character, costume) {
            return Err("the other player is wearing that costume".into());
        }
        self.slots[player].costume = costume;
        Ok(())
    }
    /// Step through costumes (`step` is +1 or -1), skipping the other
    /// player's.
    pub fn cycle_costume(&mut self, player: usize, step: i32) -> Result<(), String> {
        self.expect(Screen::Characters, "change costumes")?;
        Self::check_player(player)?;
        let character = self.slots[player]
            .character
            .ok_or("choose a character before a costume")?;
        let count = i32::from(character.costume_count());
        let mut costume = i32::from(self.slots[player].costume);
        for _ in 0..count {
            costume = (costume + step.signum()).rem_euclid(count);
            if !self.taken_by_other(player, character, costume as u8) {
                self.slots[player].costume = costume as u8;
                break;
            }
        }
        Ok(())
    }
    pub fn set_stocks(&mut self, stocks: u8) -> Result<(), String> {
        if !(1..=99).contains(&stocks) {
            return Err("stocks must be 1 to 99".into());
        }
        self.stocks = stocks;
        Ok(())
    }
    pub fn characters_ready(&self) -> bool {
        self.slots.iter().all(|slot| slot.character.is_some())
    }
    pub fn confirm_characters(&mut self) -> Result<(), String> {
        self.expect(Screen::Characters, "confirm characters")?;
        if !self.characters_ready() {
            return Err("both players need a character".into());
        }
        self.screen = Screen::Stages;
        Ok(())
    }

    /// The configuration the current picks describe.
    pub fn config(&self, stage: Stage, seed: u32) -> Result<MatchConfig, String> {
        let players: [PlayerConfig; 2] = std::array::from_fn(|i| {
            let slot = self.slots[i];
            let mut player =
                PlayerConfig::new(Port::ALL[i], slot.character.unwrap_or(Character::Fox));
            player.costume = Costume(slot.costume);
            player
        });
        if !self.characters_ready() {
            return Err("both players need a character".into());
        }
        let config = MatchConfig::versus(stage, players)
            .with_stocks(self.stocks)
            .with_seed(Seed(seed));
        melee_lib::GameAssets::files(&config).map_err(|e| e.to_string())?;
        Ok(config)
    }

    // --- Stage select and loading ----------------------------------------

    /// Pick the stage and start loading. `seed` is the host's random number.
    pub fn choose_stage(&mut self, stage: Stage, seed: u32) -> Result<(), String> {
        self.expect(Screen::Stages, "choose a stage")?;
        let config = self.config(stage, seed)?;
        self.begin_loading(config)
    }
    fn begin_loading(&mut self, config: MatchConfig) -> Result<(), String> {
        let disc = self.disc.as_ref().ok_or("no disc is open")?;
        let pending: VecDeque<_> = disc.requests(&config)?.into();
        let progress = LoadProgress {
            files_done: 0,
            files_total: pending.len() as u32,
            bytes_done: 0,
            bytes_total: pending.iter().map(FileRequest::len).sum(),
        };
        self.loading = Some(Loading {
            config,
            pending,
            progress,
        });
        self.notice = None;
        self.screen = Screen::Loading;
        Ok(())
    }
    /// The next file the host should read, or `None` when all are in.
    pub fn next_request(&self) -> Option<&FileRequest> {
        self.loading.as_ref()?.pending.front()
    }
    /// Every file still to read (hosts that fetch in parallel).
    pub fn pending_requests(&self) -> Vec<FileRequest> {
        self.loading
            .as_ref()
            .map(|l| l.pending.iter().cloned().collect())
            .unwrap_or_default()
    }
    pub fn load_progress(&self) -> LoadProgress {
        self.loading
            .as_ref()
            .map(|l| l.progress)
            .unwrap_or_default()
    }
    /// Hand over a file the host read for a pending request: a match file
    /// while loading, or a menu archive from [`App::art_requests`] on any
    /// screen.
    pub fn provide_file(&mut self, name: &str, bytes: Vec<u8>) -> Result<(), String> {
        let art_file = ArtFile::from_name(name).filter(|f| !self.art.has(*f));
        let loading_wants = self
            .loading
            .as_ref()
            .is_some_and(|l| l.pending.iter().any(|r| r.name == name));
        let preview_wants = preview::files().contains(&name)
            && self.disc.as_ref().is_some_and(|d| !d.is_cached(name));
        if let Some(file) = art_file {
            // A match also reads IfAll.usd: one read serves both, and art
            // that fails to parse never fails the match's load.
            let art = self.provide_art(file, &bytes);
            if !loading_wants && !preview_wants {
                return art;
            }
        }
        if preview_wants && !loading_wants {
            // Kept like a match's files: a match on that stage reuses them.
            let disc = self.disc.as_mut().ok_or("no disc is open")?;
            return disc.insert(name, bytes);
        }
        self.expect(Screen::Loading, "provide files")?;
        let (Some(loading), Some(disc)) = (self.loading.as_mut(), self.disc.as_mut()) else {
            return Err("nothing is loading".into());
        };
        let Some(index) = loading.pending.iter().position(|r| r.name == name) else {
            // A host reading art and match files concurrently may hand over
            // a file twice (IfAll.usd); the first copy already counted.
            if disc.is_cached(name) {
                return Ok(());
            }
            return Err(format!("{name} was not requested"));
        };
        let len = bytes.len() as u64;
        disc.insert(name, bytes)?;
        loading.pending.remove(index);
        loading.progress.files_done += 1;
        loading.progress.bytes_done += len;
        Ok(())
    }
    /// Natively the core reads the image itself: one file per call, so the
    /// host can show progress between calls. Returns whether files remain.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_next_file(&mut self) -> Result<bool, String> {
        let Some(request) = self.next_request().cloned() else {
            return Ok(false);
        };
        let disc = self.disc.as_mut().ok_or("no disc is open")?;
        match disc.read_native(&request) {
            Ok(bytes) => self.provide_file(request.name, bytes)?,
            Err(error) => {
                self.fail_loading(error.clone());
                return Err(error);
            }
        }
        Ok(self.next_request().is_some())
    }
    /// The host could not read a file: back to stage select with the error.
    pub fn fail_loading(&mut self, error: String) {
        if self.screen == Screen::Loading {
            self.loading = None;
            self.screen = Screen::Stages;
            self.notice = Some(error);
        }
    }
    /// Build the match from the fetched files. On failure (a character or
    /// stage this build cannot present yet, a fault while starting) the
    /// error becomes the notice and the flow returns to stage select.
    pub fn finish_loading(&mut self) -> Result<(), String> {
        self.expect(Screen::Loading, "finish loading")?;
        if self.next_request().is_some() {
            return Err("files are still loading".into());
        }
        let config = self.loading.take().expect("loading state").config;
        let disc = self.disc.as_ref().ok_or("no disc is open")?;
        let started = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            Session::new(disc, config.clone())
        }))
        .unwrap_or_else(|panic| Err(crate::panic_text(&*panic)));
        match started {
            Ok(session) => {
                self.session = Some(session);
                self.generation += 1;
                self.last_config = Some(config);
                self.results = None;
                self.screen = Screen::Match;
                Ok(())
            }
            Err(error) => {
                let error = format!("Could not start the match: {error}");
                self.screen = Screen::Stages;
                self.notice = Some(error.clone());
                Err(error)
            }
        }
    }

    // --- Match ------------------------------------------------------------

    /// Changes whenever a new match session starts.
    pub fn generation(&self) -> u64 {
        self.generation
    }
    /// The host could not present the new match (its renderer rejected the
    /// scene): drop it and return to stage select with the error.
    pub fn fail_match_start(&mut self, error: String) {
        if self.screen == Screen::Match {
            self.session = None;
            self.screen = Screen::Stages;
            self.notice = Some(format!("Could not start the match: {error}"));
        }
    }
    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }
    pub fn session_mut(&mut self) -> Option<&mut Session> {
        self.session.as_mut()
    }
    pub fn set_action(&mut self, player: usize, action: Action, down: bool) {
        if let Some(session) = &mut self.session {
            session.set_action(player, action, down);
        }
    }
    /// Run the simulation for `elapsed` host time. A fault stops the match
    /// (the notice carries the message and replay location); a finished
    /// match moves to the results screen.
    pub fn advance(&mut self, elapsed: Duration) -> Result<(), String> {
        if self.screen != Screen::Match {
            return Ok(());
        }
        let session = self.session.as_mut().ok_or("no match is running")?;
        if session.failure().is_some() {
            return Ok(());
        }
        if let Err(error) = session.advance(elapsed) {
            self.notice = Some(error.clone());
            return Err(error);
        }
        if let MatchStatus::Finished(outcome) = session.game().status() {
            let hud = self.hud().expect("a finished match is observable");
            let outcome = match outcome {
                MatchOutcome::Winner(port) => {
                    match hud.players.iter().position(|p| p.port == port) {
                        Some(index) => Outcome::Winner(index),
                        None => Outcome::Draw,
                    }
                }
                MatchOutcome::Draw | MatchOutcome::SuddenDeath => Outcome::Draw,
            };
            self.results = Some(Results { outcome, hud });
            self.screen = Screen::Results;
        }
        Ok(())
    }
    pub fn hud(&self) -> Option<Hud> {
        let session = self.session.as_ref()?;
        let view = session.game().observe().ok()?;
        // One entry per configured player. A player may own several fighters
        // (Nana, the sleeping Zelda/Sheik form): show the one that leads.
        let config = session.game().config();
        let player = |index: usize| -> Option<PlayerHud> {
            let pick = &config.players[index];
            let fighter = view
                .fighters()
                .filter(|f| f.port() == pick.port)
                .min_by_key(|f| !f.leads_player())?;
            Some(PlayerHud {
                port: pick.port,
                // The pick, not a transformed Zelda/Sheik's current form.
                character: pick.character,
                costume: pick.costume.0,
                percent: fighter.percent(),
                stocks: fighter.stocks(),
            })
        };
        let players = [player(0)?, player(1)?];
        Some(Hud {
            tick: view.tick.0,
            paused: session.is_paused(),
            players,
        })
    }
    pub fn results(&self) -> Option<Results> {
        self.results
    }
    /// Start the same match over, with fresh state and no recorded inputs.
    pub fn restart(&mut self) -> Result<(), String> {
        self.expect(Screen::Match, "restart")?;
        self.notice = None;
        self.session.as_mut().ok_or("no match is running")?.reset()
    }
    /// Same characters and stage, a new seed; files are already fetched.
    pub fn rematch(&mut self, seed: u32) -> Result<(), String> {
        if !matches!(self.screen, Screen::Results | Screen::Match) {
            return Err(wrong_screen("rematch", self.screen));
        }
        let config = self
            .last_config
            .clone()
            .ok_or("no match to repeat")?
            .with_seed(Seed(seed));
        self.session = None;
        self.results = None;
        self.begin_loading(config)
    }
    /// Leave the match (or results, or a load) for character select.
    pub fn quit_to_menu(&mut self) {
        if matches!(
            self.screen,
            Screen::Loading | Screen::Match | Screen::Results
        ) {
            self.session = None;
            self.loading = None;
            self.results = None;
            self.screen = Screen::Characters;
        }
    }
    /// One step back through the menus.
    pub fn back(&mut self) {
        match self.screen {
            Screen::Characters => self.screen = Screen::Disc,
            Screen::Stages => self.screen = Screen::Characters,
            Screen::Loading => {
                self.loading = None;
                self.screen = Screen::Stages;
            }
            Screen::Results => self.quit_to_menu(),
            Screen::Disc | Screen::Match => {}
        }
    }
    /// From the disc screen, continue with the disc already open.
    pub fn resume_disc(&mut self) -> Result<(), String> {
        self.expect(Screen::Disc, "continue")?;
        if self.disc.is_none() {
            return Err("no disc is open".into());
        }
        self.screen = Screen::Characters;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gc_disc::synthetic::ImageBuilder;

    /// A Melee-shaped synthetic disc holding every file `config` names, with
    /// junk contents: enough for the flow, not for starting a match.
    fn disc_for(configs: &[MatchConfig]) -> DiscFiles {
        let mut builder = ImageBuilder::melee();
        let mut names = std::collections::BTreeSet::new();
        for config in configs {
            names.extend(melee_lib::GameAssets::files(config).unwrap());
        }
        for name in names {
            builder = builder.file(name, name.as_bytes().to_vec());
        }
        let image = builder.build();
        let header = gc_disc::DiscHeader::parse(&image).unwrap();
        let fst = header.fst_range().unwrap();
        DiscFiles::from_parts(
            &image[..gc_disc::HEADER_LEN as usize],
            &image[fst.start as usize..fst.end as usize],
            image.len() as u64,
        )
        .unwrap()
    }
    fn versus(stage: Stage, a: Character, b: Character) -> MatchConfig {
        MatchConfig::versus(
            stage,
            [
                PlayerConfig::new(Port::P1, a),
                PlayerConfig::new(Port::P2, b),
            ],
        )
    }

    #[test]
    fn menus_walk_from_disc_to_loading_and_back() {
        let mut app = App::new();
        assert_eq!(app.screen(), Screen::Disc);
        assert!(app.choose_character(0, Character::Fox).is_err());
        app.open_disc(disc_for(&[versus(
            Stage::Battlefield,
            Character::Fox,
            Character::Marth,
        )]))
        .unwrap();
        assert_eq!(app.screen(), Screen::Characters);
        assert!(app.confirm_characters().is_err(), "needs two picks");
        app.choose_character(0, Character::Fox).unwrap();
        app.choose_character(1, Character::Marth).unwrap();
        app.set_stocks(2).unwrap();
        assert!(app.set_stocks(0).is_err());
        app.confirm_characters().unwrap();
        assert_eq!(app.screen(), Screen::Stages);
        app.back();
        assert_eq!(app.screen(), Screen::Characters);
        app.confirm_characters().unwrap();
        app.choose_stage(Stage::Battlefield, 7).unwrap();
        assert_eq!(app.screen(), Screen::Loading);
        let progress = app.load_progress();
        assert!(progress.files_total > 0);
        assert_eq!(progress.files_done, 0);
        app.back();
        assert_eq!(app.screen(), Screen::Stages);
        assert!(app.next_request().is_none());
    }

    #[test]
    fn same_character_players_never_share_a_costume() {
        let mut app = App::new();
        app.open_disc(disc_for(&[])).unwrap();
        app.choose_character(0, Character::Fox).unwrap();
        app.choose_character(1, Character::Fox).unwrap();
        assert_eq!(app.slots()[0].costume, 0);
        assert_eq!(app.slots()[1].costume, 1);
        assert!(app.set_costume(1, 0).is_err());
        assert!(app.set_costume(1, Character::Fox.costume_count()).is_err());
        app.cycle_costume(1, -1).unwrap();
        assert_eq!(
            app.slots()[1].costume,
            Character::Fox.costume_count() - 1,
            "wraps backwards past player 1's costume 0"
        );
        app.set_costume(0, 2).unwrap();
        app.cycle_costume(1, 1).unwrap();
        assert_eq!(app.slots()[1].costume, 0);
        app.cycle_costume(1, 1).unwrap();
        assert_eq!(app.slots()[1].costume, 1);
        app.cycle_costume(1, 1).unwrap();
        assert_eq!(app.slots()[1].costume, 3, "skips player 1's costume 2");
    }

    #[test]
    fn config_carries_picks_stocks_and_seed() {
        let mut app = App::new();
        app.open_disc(disc_for(&[])).unwrap();
        app.choose_character(0, Character::Peach).unwrap();
        app.choose_character(1, Character::Samus).unwrap();
        app.set_costume(1, 2).unwrap();
        app.set_stocks(3).unwrap();
        let config = app.config(Stage::YoshisStory, 99).unwrap();
        let mut expected = versus(Stage::YoshisStory, Character::Peach, Character::Samus)
            .with_stocks(3)
            .with_seed(Seed(99));
        expected.players[1].costume = Costume(2);
        assert_eq!(config, expected);
    }

    #[test]
    fn fetched_files_stay_cached_for_the_next_match() {
        let fox_marth = versus(Stage::FinalDestination, Character::Fox, Character::Marth);
        let fox_falco = versus(Stage::FinalDestination, Character::Fox, Character::Falco);
        let mut app = App::new();
        app.open_disc(disc_for(&[fox_marth.clone(), fox_falco.clone()]))
            .unwrap();
        app.choose_character(0, Character::Fox).unwrap();
        app.choose_character(1, Character::Marth).unwrap();
        app.confirm_characters().unwrap();
        app.choose_stage(Stage::FinalDestination, 1).unwrap();
        let all = melee_lib::GameAssets::files(&fox_marth).unwrap().len() as u32;
        assert_eq!(app.load_progress().files_total, all);
        assert!(app.provide_file("PlCo.dat", b"wrong".to_vec()).is_err());
        assert!(app.provide_file("NotRequested.dat", vec![]).is_err());
        while let Some(request) = app.next_request().cloned() {
            // The synthetic disc stores each file's name as its contents.
            app.provide_file(request.name, request.name.as_bytes().to_vec())
                .unwrap();
        }
        assert_eq!(app.load_progress().files_done, all);
        // Junk files cannot start a match: the error returns to stage select.
        let error = app.finish_loading().unwrap_err();
        assert!(error.starts_with("Could not start the match"), "{error}");
        assert_eq!(app.screen(), Screen::Stages);
        assert_eq!(app.notice(), Some(error.as_str()));
        // Fox vs Falco needs only Falco's files now.
        app.back();
        app.choose_character(1, Character::Falco).unwrap();
        app.confirm_characters().unwrap();
        app.choose_stage(Stage::FinalDestination, 2).unwrap();
        let needed = app.pending_requests();
        assert!(!needed.is_empty());
        assert!(needed.iter().all(|r| r.name.contains("Fc")), "{needed:?}");
    }

    /// An empty but well-formed HSD archive: a header and nothing else.
    fn empty_archive() -> Vec<u8> {
        let mut bytes = vec![0u8; 0x20];
        bytes[..4].copy_from_slice(&0x20u32.to_be_bytes());
        bytes
    }

    #[test]
    fn menu_art_is_requested_after_the_disc_opens_and_arrives_on_any_screen() {
        let mut builder = ImageBuilder::melee();
        for file in ArtFile::ALL {
            builder = builder.file(file.name(), empty_archive());
        }
        let image = builder.build();
        let header = gc_disc::DiscHeader::parse(&image).unwrap();
        let fst = header.fst_range().unwrap();
        let disc = || {
            DiscFiles::from_parts(
                &image[..gc_disc::HEADER_LEN as usize],
                &image[fst.start as usize..fst.end as usize],
                image.len() as u64,
            )
            .unwrap()
        };
        let mut app = App::new();
        assert!(app.art_requests().is_empty(), "no disc, nothing to read");
        assert!(!app.art_ready());
        app.open_disc(disc()).unwrap();
        let requests = app.art_requests();
        let names: Vec<_> = requests.iter().map(|r| r.name).collect();
        assert_eq!(names, ["MnSlChr.usd", "MnSlMap.usd", "IfAll.usd"]);
        assert!(requests.iter().all(|r| r.len() == 0x20));
        assert_eq!(app.art_progress().files_total, 3);
        assert_eq!(app.art_progress().files_done, 0);
        let pending = app.art_image(Piece::Face(Character::Fox)).unwrap_err();
        assert!(pending.contains("not loaded yet"), "{pending}");

        // Lengths are checked against the file table; any screen accepts art.
        assert!(app.provide_file("MnSlChr.usd", vec![0; 3]).is_err());
        app.choose_character(0, Character::Fox).unwrap();
        for request in requests {
            app.provide_file(request.name, empty_archive()).unwrap();
        }
        assert!(app.art_ready());
        assert!(app.art_requests().is_empty());
        let progress = app.art_progress();
        assert_eq!((progress.files_done, progress.bytes_done), (3, 0x60));
        // An archive without the menu's tables fails per piece, not overall.
        assert!(app.art_image(Piece::Face(Character::Fox)).is_err());
        assert!(app.art_image(Piece::StageIcon(Stage::Battlefield)).is_err());
        // A second copy of a delivered archive is not art and not a match file.
        assert!(app.provide_file("IfAll.usd", empty_archive()).is_err());

        // A new disc starts its art over.
        app.back();
        app.open_disc(disc()).unwrap();
        assert!(!app.art_ready());
        assert_eq!(app.art_requests().len(), 3);
    }

    #[test]
    fn one_read_of_the_interface_archive_serves_the_match_and_the_art() {
        let config = versus(Stage::Battlefield, Character::Fox, Character::Marth);
        let mut builder = ImageBuilder::melee();
        for name in melee_lib::GameAssets::files(&config).unwrap() {
            let contents = if name == "IfAll.usd" {
                empty_archive()
            } else {
                name.as_bytes().to_vec()
            };
            builder = builder.file(name, contents);
        }
        let image = builder.build();
        let header = gc_disc::DiscHeader::parse(&image).unwrap();
        let fst = header.fst_range().unwrap();
        let disc = DiscFiles::from_parts(
            &image[..gc_disc::HEADER_LEN as usize],
            &image[fst.start as usize..fst.end as usize],
            image.len() as u64,
        )
        .unwrap();
        let mut app = App::new();
        app.open_disc(disc).unwrap();
        app.choose_character(0, Character::Fox).unwrap();
        app.choose_character(1, Character::Marth).unwrap();
        app.confirm_characters().unwrap();
        app.choose_stage(Stage::Battlefield, 1).unwrap();
        let pending = app.pending_requests();
        assert!(pending.iter().any(|r| r.name == "IfAll.usd"));
        for request in &pending {
            let bytes = if request.name == "IfAll.usd" {
                empty_archive()
            } else {
                request.name.as_bytes().to_vec()
            };
            app.provide_file(request.name, bytes).unwrap();
        }
        assert!(app.next_request().is_none(), "the match counted IfAll.usd");
        assert!(
            !app.art_requests().iter().any(|r| r.name == "IfAll.usd"),
            "and so did the art"
        );
        // A concurrent art read handing it over again is harmless.
        app.provide_file("IfAll.usd", empty_archive()).unwrap();
        assert_eq!(app.load_progress().files_done as usize, pending.len());
    }

    #[test]
    fn stage_preview_files_follow_the_menu_archives_and_start_one_job() {
        let mut builder = ImageBuilder::melee();
        let mut names: Vec<&str> = ArtFile::ALL.iter().map(|f| f.name()).collect();
        names.extend(preview::files());
        names.sort_unstable();
        names.dedup();
        for &name in &names {
            builder = builder.file(name, empty_archive());
        }
        let image = builder.build();
        let header = gc_disc::DiscHeader::parse(&image).unwrap();
        let fst = header.fst_range().unwrap();
        let disc = DiscFiles::from_parts(
            &image[..gc_disc::HEADER_LEN as usize],
            &image[fst.start as usize..fst.end as usize],
            image.len() as u64,
        )
        .unwrap();
        let mut app = App::new();
        app.open_disc(disc).unwrap();
        let requests = app.art_requests();
        let requested: Vec<_> = requests.iter().map(|r| r.name).collect();
        assert_eq!(requested[..3], ["MnSlChr.usd", "MnSlMap.usd", "IfAll.usd"]);
        assert_eq!(requested.len(), names.len(), "each file once");
        assert!(preview::files().iter().all(|f| requested.contains(f)));
        assert!(app.stage_preview_job().is_none(), "nothing read yet");
        let piece = Piece::StagePreview(Stage::Battlefield);
        for request in requests {
            app.provide_file(request.name, empty_archive()).unwrap();
            assert!(app.art_image(piece).is_err());
        }
        assert!(app.art_ready());
        assert!(app.art_requests().is_empty());
        assert!(!app.stage_previews_ready());
        let job = app.stage_preview_job().expect("every preview file is in");
        assert_eq!(job.files.len(), preview::files().len());
        assert!(app.stage_preview_job().is_none(), "handed out once per disc");
        // A new disc starts over.
        app.back();
        app.open_disc(disc_for(&[])).unwrap();
        assert!(app.stage_preview_job().is_none());
    }

    #[test]
    fn a_disc_missing_the_files_cannot_load() {
        let mut app = App::new();
        app.open_disc(disc_for(&[])).unwrap();
        app.choose_character(0, Character::Fox).unwrap();
        app.choose_character(1, Character::Fox).unwrap();
        app.confirm_characters().unwrap();
        // The empty synthetic disc lacks the files: loading cannot begin.
        let error = app.choose_stage(Stage::Battlefield, 0).unwrap_err();
        assert!(error.contains("is not on this disc"), "{error}");
        assert_eq!(app.screen(), Screen::Stages);
    }
}
