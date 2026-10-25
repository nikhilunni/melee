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
//! `CommandState::color_animations` retains request order. Script batches
//! publish slot ownership without stepping the program; direct install/clear
//! paths drain older requests first. Program execution stays in the color proc.
use super::smash::{ChargeOverlay, OverlayCommand};
use super::FighterCore;
use hsd_archive::Archive;

/// Ids below Fighter_804D6538's 0x7B split use the fighter table.
const TABLE_LEN: u32 = 0x7B;
/// ftCo_800C0408: the flashing program installed while a fighter is
/// intangible or invincible (x1990 / x1994 / x2221_b0).
const INVINCIBILITY_FLASH: u8 = 9;

#[derive(Clone, Copy, Debug)]
struct Entry {
    priority: u8,
    /// Fighter_804D653C_t.unk5: nonzero selects the secondary slot (x488).
    secondary: bool,
    /// `ColorOverlayTable::commands[start..end]`; gotos are program-relative.
    start: u32,
    end: u32,
}

/// The decoded color table; ids without a program (id 0) are idle.
///
/// Programs share one command buffer and entries are plain data, so the
/// whole table drops as one vector wherever the fighter's assets are freed.
/// Every program of PlCo.dat's table decodes (docs/COVERAGE_AUDIT.md, wave
/// 13), so one that did not would fail the load.
#[derive(Clone, Debug)]
pub struct ColorOverlayTable {
    entries: [Option<Entry>; TABLE_LEN as usize],
    commands: Vec<OverlayCommand>,
}
impl ColorOverlayTable {
    pub fn read(archive: &Archive, table: u32) -> super::assets::Result<Self> {
        let r = archive.reader();
        let mut overlays = Self {
            entries: [None; TABLE_LEN as usize],
            commands: Vec::new(),
        };
        for id in 0..TABLE_LEN {
            let entry = table + id * 8;
            let Some(script) = archive.link(entry)? else {
                continue;
            };
            let priority = r.u8(entry + 4)?;
            let secondary = r.u8(entry + 5)? != 0;
            let program = super::smash::read_overlay(archive, script)
                .map_err(|error| format!("color animation {id}: {error}"))?;
            let start = overlays.commands.len() as u32;
            overlays.commands.extend(program);
            overlays.entries[id as usize] = Some(Entry {
                priority,
                secondary,
                start,
                end: overlays.commands.len() as u32,
            });
        }
        Ok(overlays)
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
        let entry = self.entry(id);
        &self.commands[entry.start as usize..entry.end as usize]
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
    /// Script batches publish slot ownership immediately, even when the
    /// next ftCo_800C0408 program step is in a later scheduler phase.
    /// This installs only: no timer, graphics, sound or RNG is advanced.
    pub(super) fn install_requested_color_overlays(&mut self, assets: &super::assets::FighterAssets) {
        for request in self.commands.color_animations.take_all() {
            self.install_color_overlay(request.id, request.duration, &assets.color_overlays);
        }
    }

    /// ftCo_800BFFD0 called directly, returning whether `id` was installed.
    /// Requests queued earlier were installed earlier in retail, so they go first.
    pub fn install_color_overlay_now(
        &mut self,
        id: u8,
        assets: &super::assets::FighterAssets,
    ) -> bool {
        self.install_requested_color_overlays(assets);
        self.install_color_overlay(id, 0, &assets.color_overlays)
    }

    /// ftCo_800BFFD0: install `id` in its slot (x408, or x488 for the
    /// table's secondary entries) unless the slot holds a higher priority.
    /// Returns whether the slot took it.
    fn install_color_overlay(&mut self, id: u8, duration: u32, table: &ColorOverlayTable) -> bool {
        let slot = if table.entry(id).secondary {
            &mut self.combat.secondary_color_overlay
        } else {
            &mut self.combat.color_overlay
        };
        let installed = table.priority(slot.id) <= table.priority(id);
        if installed {
            *slot = ColorOverlaySlot {
                id,
                program: ChargeOverlay::default(),
                frames_left: duration,
                flash_expired: slot.flash_expired,
            };
        }
        installed
    }

    /// ftCo_800C0134 (800C0134) on a motion change without Ft_MF_SkipColAnim:
    /// lb_80014498 on the secondary slot and ftData_UnkMotionStates4.
    /// Retail installs script requests as they run, so this motion's queued
    /// secondary requests went too. The Hammer states (ftCo_800C53E4) are
    /// out of scope.
    pub fn clear_secondary_color_overlay(&mut self, table: &ColorOverlayTable) {
        self.empty_secondary_color_overlay(table);
        let queued = &mut self.commands.color_animations;
        let mut i = 0;
        while i < queued.len() {
            if table
                .entry(queued.iter().nth(i).expect("queued request").id)
                .secondary
            {
                queued.remove(i);
            } else {
                i += 1;
            }
        }
    }

    /// ftCo_800BFFAC (800BFFAC): lb_80014498 on the secondary slot (x488)
    /// alone, with no fallback. Retail installs requests as they come, so a
    /// queued request would already own its slot and go with it; no caller
    /// clears with one queued (fails closed otherwise).
    pub fn clear_secondary_color_slot(&mut self) {
        assert!(
            self.commands.color_animations.is_empty(),
            "ftCo_800BFFAC with a color animation queued"
        );
        self.combat.secondary_color_overlay = ColorOverlaySlot::default();
    }

    /// ftCo_800C0408's primary-slot loop: run the program; when it ends or its
    /// frame count runs out, clear the slot (lb_80014498) and install the
    /// invincibility flash if the fighter is still protected, then run that.
    /// Retail's other fallbacks (8, 0x6B) need states the port does not
    /// reach (x18F0, x221D_b6).
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
        // ftCo_800C0408: the secondary slot runs after the primary; with the
        // primary busy, ft_800BFF70 skips its effects and sounds.
        let emit = self.combat.color_overlay.id == 0;
        while self.combat.secondary_color_overlay.id != 0 {
            let slot = &mut self.combat.secondary_color_overlay;
            let ended = slot.program.step(
                emit,
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
            self.empty_secondary_color_overlay(table);
        }
    }

    /// lb_80014498 on the secondary slot (x488), then the kind's
    /// ftData_UnkMotionStates4 (`secondary_color_fallback`).
    fn empty_secondary_color_overlay(&mut self, table: &ColorOverlayTable) {
        self.combat.secondary_color_overlay = ColorOverlaySlot::default();
        if let Some(id) = self.combat.secondary_color_fallback {
            self.install_color_overlay(id, 0, table);
        }
    }

    /// lb_80014498 on the primary slot, then ftCo_800C0408's fallbacks: a
    /// Stamina fighter's KO color (x2224_b2), then the flash while the
    /// fighter is intangible or invincible.
    fn clear_color_overlay(&mut self, table: &ColorOverlayTable) {
        self.combat.color_overlay = ColorOverlaySlot::default();
        if self.stamina_out {
            self.install_color_overlay(super::stamina::STAMINA_OUT_COLOR, 0, table);
        }
        if self.status.revival_invincibility != 0 || self.status.ledge_intangibility != 0 {
            self.install_color_overlay(INVINCIBILITY_FLASH, 0, table);
        }
    }

    /// ftCo_800C0200 (800C0200): a released smash charge clears its color's
    /// slot. The secondary slot empties and takes the kind's
    /// ftData_UnkMotionStates4 (the Hammer check is out of scope); the
    /// primary slot takes ftCo_800C0408's fallbacks. Requests queued before
    /// the release were installed earlier in retail, so they go first.
    pub(super) fn release_charge_color(&mut self, id: u8, assets: &super::assets::FighterAssets) {
        // ftcolanim.c:199-203: retail asserts on a spycloak id.
        if id >= TABLE_LEN as u8 {
            return;
        }
        self.install_requested_color_overlays(assets);
        let table = &assets.color_overlays;
        if table.entry(id).secondary {
            self.empty_secondary_color_overlay(table);
        } else {
            self.clear_color_overlay(table);
        }
    }
}
