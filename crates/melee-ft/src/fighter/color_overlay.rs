//! Fighter color animations, ftcolanim.c / lb_013B.c.
//!
//! PlCo's color table (Fighter_804D653C) gives every color animation id a
//! priority, a slot and a small program. ftCo_800BFFD0 installs an id into
//! its slot when the slot's current id has no higher priority; each frame
//! ftCo_800C0408 runs the programs. Most programs only tint the model, but
//! some also spawn effects or play sounds (the burning and electric damage
//! programs, the powershield flash), so which program owns a slot is
//! gameplay: it decides which effects draw from the shared RNG.
//!
//! Requests from scripts, damage and shields go through
//! `CommandState::color_animations` in call order and are installed right
//! before the next program step, which preserves retail's install order.
use super::smash::{ChargeOverlay, OverlayCommand};
use super::FighterCore;
use hsd_archive::Archive;

/// Ids below Fighter_804D6538's 0x7B split use the fighter table.
const TABLE_LEN: u32 = 0x7B;
/// ftCo_800C0408: the flashing program installed while a fighter is
/// intangible or invincible (x1990 / x1994 / x2221_b0).
const INVINCIBILITY_FLASH: u8 = 9;
/// ftCo_800DF0D0: a smash charge with color 0x7B installs no program.
const NO_CHARGE_COLOR: u8 = 0x7B;

#[derive(Clone, Debug)]
struct Entry {
    priority: u8,
    /// Fighter_804D653C_t.unk5: nonzero selects the secondary slot (x488).
    secondary: bool,
    program: Result<Vec<OverlayCommand>, String>,
}

/// The decoded color table; ids without a program (id 0) are idle.
#[derive(Clone, Debug)]
pub struct ColorOverlayTable {
    entries: Vec<Option<Entry>>,
}
impl ColorOverlayTable {
    pub fn read(archive: &Archive, table: u32) -> super::assets::Result<Self> {
        let r = archive.reader();
        let mut entries = Vec::with_capacity(TABLE_LEN as usize);
        for id in 0..TABLE_LEN {
            let entry = table + id * 8;
            entries.push(match archive.link(entry)? {
                None => None,
                Some(script) => Some(Entry {
                    priority: r.u8(entry + 4)?,
                    secondary: r.u8(entry + 5)? != 0,
                    program: super::smash::read_overlay(archive, script).map_err(|e| e.to_string()),
                }),
            });
        }
        Ok(Self { entries })
    }
    fn entry(&self, id: u8) -> &Entry {
        self.entries[usize::from(id)]
            .as_ref()
            .unwrap_or_else(|| panic!("color animation {id} has no program"))
    }
    /// Priority of the id a slot holds; an idle slot (id 0) has priority 0.
    fn priority(&self, id: u8) -> u8 {
        self.entries[usize::from(id)]
            .as_ref()
            .map_or(0, |e| e.priority)
    }
    pub fn program(&self, id: u8) -> &[OverlayCommand] {
        match &self.entry(id).program {
            Ok(program) => program,
            Err(error) => unimplemented!("color animation {id}: {error}"),
        }
    }
    /// Whether a program affects gameplay (effects draw RNG, sounds queue).
    fn has_gameplay_commands(&self, id: u8) -> bool {
        self.program(id)
            .iter()
            .any(|c| matches!(c, OverlayCommand::Graphics(_) | OverlayCommand::Sound(_)))
    }
}

/// Fighter.x408, the primary ColorOverlay slot.
#[derive(Clone, Debug, Default)]
pub struct ColorOverlaySlot {
    /// x28_colanim.i: 0 when idle.
    pub id: u8,
    pub program: ChargeOverlay,
    /// x4_pri: frames until the program is stopped; 0 runs until it ends.
    frames_left: u32,
    /// A protection timer ran out this frame (Fighter_8006A360); the flash
    /// is cleared before the next step if it still owns the slot.
    pub flash_expired: bool,
}

impl FighterCore {
    /// ftCo_800BFFD0 -> lb_800144C8 for queued requests, in request order.
    fn install_requested_color_overlays(&mut self, assets: &super::assets::FighterAssets) {
        let requests = std::mem::take(&mut self.commands.color_animations);
        for request in requests.iter() {
            self.install_color_overlay(request.id, request.duration, &assets.color_overlays);
        }
    }

    /// ftCo_800BFFD0: install `id` unless the slot holds a higher priority.
    fn install_color_overlay(&mut self, id: u8, duration: u32, table: &ColorOverlayTable) {
        if table.entry(id).secondary {
            // The secondary slot (x488) is not modelled; its programs are
            // tint-only in the supported roster, which keeps this RNG-neutral.
            if table.has_gameplay_commands(id) {
                unimplemented!("ftCo_800BFFD0: secondary color slot program {id} with effects");
            }
            return;
        }
        let slot = &mut self.combat.color_overlay;
        if table.priority(slot.id) <= table.priority(id) {
            *slot = ColorOverlaySlot {
                id,
                program: ChargeOverlay::default(),
                frames_left: duration,
                flash_expired: slot.flash_expired,
            };
        }
    }

    /// ftCo_800C0408's primary-slot loop: run the program; when it ends or its
    /// frame count runs out, clear the slot (lb_80014498) and install the
    /// invincibility flash if the fighter is still protected, then run that.
    /// Retail's other fallbacks (0x7A, 8, 0x6B) need states the port does not
    /// reach (CPU input takeover, x18F0, x221D_b6).
    pub(super) fn advance_color_overlay(&mut self, assets: &super::assets::FighterAssets) {
        let table = &assets.color_overlays;
        // fighter.c:1470/1481: ftCo_800C0694(fp) == 9 -> ftCo_800C0200(fp, 9),
        // which clears the slot and reinstalls the flash if still protected.
        if std::mem::take(&mut self.combat.color_overlay.flash_expired)
            && self.combat.color_overlay.id == INVINCIBILITY_FLASH
        {
            self.clear_color_overlay(table);
        }
        self.install_requested_color_overlays(assets);
        while self.combat.color_overlay.id != 0 {
            let slot = &mut self.combat.color_overlay;
            let ended = slot.program.step(
                true,
                table.program(slot.id),
                &mut self.commands.graphics,
                &mut self.commands.footstep_sounds,
            );
            let expired = !ended && slot.frames_left != 0 && {
                slot.frames_left -= 1;
                slot.frames_left == 0
            };
            if !ended && !expired {
                break;
            }
            self.clear_color_overlay(table);
        }
        self.advance_charge_color(assets);
    }

    /// lb_80014498 on the primary slot, then ftCo_800C0408's fallback: the
    /// flash returns while the fighter is intangible or invincible.
    fn clear_color_overlay(&mut self, table: &ColorOverlayTable) {
        self.combat.color_overlay = ColorOverlaySlot::default();
        if self.status.revival_invincibility != 0 || self.status.ledge_intangibility != 0 {
            self.install_color_overlay(INVINCIBILITY_FLASH, 0, table);
        }
    }

    /// ftCo_800C0408's secondary slot (x488), which holds the smash-charge
    /// color while charging. With the primary slot busy it runs through
    /// ft_800BFF70, which skips the program's effects and sounds.
    fn advance_charge_color(&mut self, assets: &super::assets::FighterAssets) {
        let Some(charge) = &self.commands.smash_charge else {
            return;
        };
        if !matches!(charge.phase, melee_cmd::ChargePhase::Charging)
            || charge.color_animation == NO_CHARGE_COLOR
        {
            return;
        }
        let emit = self.combat.color_overlay.id == 0;
        self.combat.charge_overlay.step(
            emit,
            assets.color_overlays.program(charge.color_animation),
            &mut self.commands.graphics,
            &mut self.commands.footstep_sounds,
        );
    }
}
