//! The HUD's score popups of a Time match (if_2F72.c, lbl_804A1340): the
//! "+1", "-1" and "-2" models a fall shows over the players' HUD places.
//!
//! Nothing here renders; the popups are GObjs (GObj_Create(14, 15, 0)), so
//! their creation and destruction decide the addresses later objects take
//! (`hsd_gobj::GObjAddresses`). A popup lives until its animation ends
//! (lb_8000B09C); the scene steps them at s_link 17 with the animation's
//! timeline (`PopupTimeline`), which it reads from IfAll.
use hsd_gobj::{GObjAddress, GObjClass, GObjAddresses};

/// Player slots the table holds (lbl_804A1340: one pair per slot, six).
const SLOTS: usize = 6;
/// fn_802F7994: past this frame of a "-1" for a fall to its own hit, the
/// second "-1" appears.
const SECOND_POPUP_FRAME: f32 = 12.0;

/// The popup model's animation (gm_8016895C's index into
/// ScInfStc_scene_models[0]): what the number reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PopupKind {
    /// Animation 0: "-2".
    MinusTwo = 0,
    /// Animation 1: "-1".
    MinusOne = 1,
    /// Animation 2: "+1".
    PlusOne = 2,
}

/// A popup's proc (HSD_GObj_SetupProc(gobj, fn, 0x11)).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PopupProc {
    /// fn_802F75D4 (a slot's first popup) or fn_802F7670 (its second):
    /// animate until done, then destroy.
    Fade,
    /// fn_802F7994: as `Fade`, and past frame 12 a second "-1" in the
    /// slot's other entry while that is empty.
    FadeThenSecond,
}

#[derive(Clone, Copy, Debug)]
struct Popup {
    gobj: GObjAddress,
    kind: PopupKind,
    proc_kind: PopupProc,
    /// Procs this popup has run.
    steps: u16,
    /// Creation order, which is its proc's order in the s_link 17 pass.
    order: u32,
}

/// One popup animation's course from its creation (fn_802F77F8:
/// HSD_JObjReqAnimAll(0), HSD_JObjAnimAll): before proc `k`, the first
/// AObj's current frame (lbGetJObjCurrFrame); the animation is done
/// (lb_8000B09C false) at the last entry's proc, which destroys it.
#[derive(Clone, Debug, Default)]
pub struct PopupTimeline {
    pub frames: Vec<f32>,
}

/// A player's own fighter's fall, as ftCo_800D34E0 reports it to the HUD
/// (Player_UpdateKOsBySlot(by, 0, fallen) with a source,
/// Player_IncSuicideCount without).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fall {
    pub fallen: u8,
    /// dmg.x18C4: the player credited; `None` for retail's 6.
    pub by: Option<u8>,
}

/// What the HUD makes of a fall.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScoreEvent {
    /// Another player's KO (ifStatus_802F6AF8 -> if_802F7AF8): "-1" over
    /// the fallen, "+1" over the player its fighter names as its source
    /// (Player_80036428).
    KnockedOut { fallen: u8, by: u8 },
    /// A teammate's KO (ifStatus_802F69C0): the self-destruct popup over
    /// the teammate (if_802F7C30), then "-1" over the fallen (if_802F7BB4).
    KnockedOutByTeammate { fallen: u8, by: u8 },
    /// A fall to the player's own hit (ifStatus_802F6D10 -> if_802F7D08).
    OwnKnockOut { fallen: u8 },
    /// A fall nobody is credited with (Player_IncSuicideCount ->
    /// ifStatus_802F6C04 -> if_802F7C30).
    SelfDestruct { fallen: u8 },
}

/// lbl_804A1340's popup entries: each slot's first (`[slot][0]`, base[2s+1])
/// and second (`[slot][1]`, base[2s+2]) popup.
#[derive(Clone, Debug)]
pub struct ScorePopups {
    /// StartMeleeRules match_kind != Stock && x2_5: the HUD shows scores
    /// (ifStatus_TriggerStockLoss, ifStatus_802F69C0).
    pub enabled: bool,
    /// StartMeleeRules xC (gm_8016AEC8): what a self-destruct scores.
    pub self_destruct_penalty: i8,
    /// Each slot's team in a team battle (Player_GetTeam, gm_8016B168).
    pub teams: [Option<u8>; SLOTS],
    entries: [[Option<Popup>; 2]; SLOTS],
    next_order: u32,
}

impl Default for ScorePopups {
    fn default() -> Self {
        Self {
            enabled: false,
            self_destruct_penalty: -1,
            teams: [None; SLOTS],
            entries: [[None; 2]; SLOTS],
            next_order: 0,
        }
    }
}

impl ScorePopups {
    /// Whether any popup lives.
    pub fn is_empty(&self) -> bool {
        self.entries.iter().flatten().all(Option::is_none)
    }

    /// The HUD's response to a fall (ifStatus_TriggerStockLoss's callback,
    /// and ifStatus_802F69C0's own if_802F7C30 first).
    pub fn score(&mut self, fall: Fall, gobjs: &mut GObjAddresses) {
        if !self.enabled {
            return;
        }
        let Fall { fallen, by } = fall;
        // Player_UpdateKOsBySlot (player.c:1367-1387).
        let event = match by {
            None => ScoreEvent::SelfDestruct { fallen },
            Some(by) if by == fallen => ScoreEvent::OwnKnockOut { fallen },
            Some(by)
                if self.teams[usize::from(by)].is_some()
                    && self.teams[usize::from(by)] == self.teams[usize::from(fallen)] =>
            {
                ScoreEvent::KnockedOutByTeammate { fallen, by }
            }
            Some(by) => ScoreEvent::KnockedOut { fallen, by },
        };
        match event {
            ScoreEvent::KnockedOut { fallen, by } => {
                // if_802F7AF8.
                self.show(fallen, 0, PopupKind::MinusOne, PopupProc::Fade, gobjs);
                self.show(by, 0, PopupKind::PlusOne, PopupProc::Fade, gobjs);
            }
            ScoreEvent::KnockedOutByTeammate { fallen, by } => {
                self.self_destruct(by, gobjs);
                // if_802F7BB4.
                self.show(fallen, 0, PopupKind::MinusOne, PopupProc::Fade, gobjs);
            }
            ScoreEvent::OwnKnockOut { fallen } => {
                // if_802F7D08.
                let (kind, proc_kind) = match self.self_destruct_penalty {
                    -2 => (PopupKind::MinusTwo, PopupProc::FadeThenSecond),
                    -1 => (PopupKind::MinusOne, PopupProc::FadeThenSecond),
                    _ => (PopupKind::MinusOne, PopupProc::Fade),
                };
                self.show(fallen, 0, kind, proc_kind, gobjs);
            }
            ScoreEvent::SelfDestruct { fallen } => self.self_destruct(fallen, gobjs),
        }
    }

    /// if_802F7C30: "-2" or "-1" by the self-destruct rule; nothing when a
    /// self-destruct costs nothing.
    fn self_destruct(&mut self, slot: u8, gobjs: &mut GObjAddresses) {
        let kind = match self.self_destruct_penalty {
            -2 => PopupKind::MinusTwo,
            -1 => PopupKind::MinusOne,
            _ => return,
        };
        self.show(slot, 0, kind, PopupProc::Fade, gobjs);
    }

    /// fn_802F77F8 into an entry: the entry's popup goes first
    /// (HSD_GObjPLink_80390228), then the new one's GObj.
    fn show(
        &mut self,
        slot: u8,
        entry: usize,
        kind: PopupKind,
        proc_kind: PopupProc,
        gobjs: &mut GObjAddresses,
    ) {
        let slot = usize::from(slot);
        assert!(slot < SLOTS, "if_802F7AF8: a popup for player slot {slot}");
        if let Some(old) = self.entries[slot][entry].take() {
            gobjs.destroy(GObjClass::Hud, old.gobj);
        }
        let gobj = gobjs.create(GObjClass::Hud);
        self.entries[slot][entry] = Some(Popup {
            gobj,
            kind,
            proc_kind,
            steps: 0,
            order: self.next_order,
        });
        self.next_order += 1;
    }

    /// Every popup's proc this tick (s_link 17), in creation order. A
    /// second popup created here runs its first proc in the same pass.
    /// `timelines` are indexed by `PopupKind`.
    pub fn tick(&mut self, timelines: &[PopupTimeline; 3], gobjs: &mut GObjAddresses) {
        let mut ran = 0u32;
        loop {
            // The next popup in creation order whose proc has not run.
            let next = self
                .entries
                .iter()
                .enumerate()
                .flat_map(|(slot, pair)| {
                    pair.iter()
                        .enumerate()
                        .filter_map(move |(entry, p)| p.map(|p| (slot, entry, p)))
                })
                .filter(|(_, _, p)| p.order >= ran)
                .min_by_key(|(_, _, p)| p.order);
            let Some((slot, entry, popup)) = next else {
                break;
            };
            ran = popup.order + 1;
            let timeline = &timelines[popup.kind as usize];
            let step = usize::from(popup.steps);
            let last = timeline.frames.len().saturating_sub(1);
            let frame = timeline.frames.get(step.min(last)).copied().unwrap_or(0.0);
            let active = step < last;
            if popup.proc_kind == PopupProc::FadeThenSecond
                && frame > SECOND_POPUP_FRAME
                && self.entries[slot][1].is_none()
            {
                self.show(slot as u8, 1, PopupKind::MinusOne, PopupProc::Fade, gobjs);
            }
            if active {
                if let Some(p) = &mut self.entries[slot][entry] {
                    p.steps += 1;
                }
            } else {
                self.entries[slot][entry] = None;
                gobjs.destroy(GObjClass::Hud, popup.gobj);
            }
        }
    }

    /// The GObjs a splice renamed (`GObjAddresses::replay`).
    pub fn rename_gobjs(&mut self, renamed: &hsd_gobj::Renamed) {
        for popup in self.entries.iter_mut().flatten().flatten() {
            popup.gobj = renamed.apply(popup.gobj);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timelines(lengths: [usize; 3]) -> [PopupTimeline; 3] {
        lengths.map(|n| PopupTimeline {
            frames: (0..n).map(|k| (k + 1) as f32).collect(),
        })
    }

    #[test]
    fn a_ko_shows_minus_one_then_plus_one_and_both_expire() {
        let mut gobjs = GObjAddresses::default();
        let mut popups = ScorePopups {
            enabled: true,
            ..Default::default()
        };
        popups.score(Fall { fallen: 1, by: Some(0) }, &mut gobjs);
        let t = timelines([5, 4, 4]);
        for _ in 0..3 {
            popups.tick(&t, &mut gobjs);
            assert!(!popups.is_empty());
        }
        popups.tick(&t, &mut gobjs);
        assert!(popups.is_empty());
    }

    #[test]
    fn a_stock_match_shows_nothing() {
        let mut gobjs = GObjAddresses::default();
        let mut popups = ScorePopups::default();
        popups.score(Fall { fallen: 0, by: None }, &mut gobjs);
        assert!(popups.is_empty());
    }

    #[test]
    fn an_own_ko_adds_a_second_minus_one_past_frame_twelve() {
        let mut gobjs = GObjAddresses::default();
        let mut popups = ScorePopups {
            enabled: true,
            ..Default::default()
        };
        popups.score(Fall { fallen: 2, by: Some(2) }, &mut gobjs);
        let t = timelines([20, 20, 20]);
        for _ in 0..12 {
            popups.tick(&t, &mut gobjs);
        }
        assert!(popups.entries[2][1].is_none());
        popups.tick(&t, &mut gobjs);
        assert!(popups.entries[2][1].is_some());
    }
}
