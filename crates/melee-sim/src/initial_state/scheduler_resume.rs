//! Saved HSD scheduler cursor and instruction-level continuations.
use super::{word, SavedPose};
use anyhow::{ensure, Context, Result};

/// A callback's owner within the retail p_link list. Ground uses map_id;
/// fighters use player_id. Neither is inferred from the current player's slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ProcKey {
    pub p_link: u8,
    pub object: u8,
    pub callback: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Continuation {
    Invoke,
    Complete,
    Animation { restart: bool, configuring: bool },
    Collision { in_sweep: bool },
    GroundCollision,
    ParticleEmission,
}

pub(crate) struct SchedulerResume {
    pub s_link: u8,
    pub current: Option<(ProcKey, Continuation)>,
    /// The actual proc-list prefix, including disabled procs. Read only on tick zero.
    completed: Vec<ProcKey>,
}
impl SchedulerResume {
    pub fn between_ticks(match_start: bool) -> Self {
        Self {
            s_link: if match_start { 24 } else { 0 },
            current: None,
            completed: Vec::new(),
        }
    }
    pub fn action(&self, s_link: u8, key: ProcKey) -> Continuation {
        if s_link < self.s_link || (s_link == self.s_link && self.completed.contains(&key)) {
            return Continuation::Complete;
        }
        if s_link == self.s_link {
            if let Some((current, action)) = self.current {
                if current == key {
                    return action;
                }
            }
        }
        Continuation::Invoke
    }
    pub(super) fn restore(saved: &SavedPose, match_start: bool) -> Result<Self> {
        // gobj.c:19-21,101-115: link, current proc, next proc. The list-head
        // array is INDIRECT through 804D7840 (gobj.c:17,103), not 804D7834.
        let current = word(saved.bytes(0x804D_7838, 4), 0);
        let link = word(saved.bytes(0x804D_7834, 4), 0);
        if current == 0 && link == 24 {
            return Ok(Self::between_ticks(match_start));
        }
        ensure!(
            current != 0 && link < 24 && !match_start,
            "unsupported scheduler boundary"
        );
        let raw = saved.bytes(current, 0x18);
        ensure!(u32::from(raw[12]) == link, "saved scheduler link mismatch");
        // gobjproc.h:9-18; gobjproc.c:71-89,158-162: child +0, next +4,
        // prev +8, s_link +C, flags +D, owner +10, callback +14.
        ensure!(
            word(raw, 4) == word(saved.bytes(0x804D_7830, 4), 0),
            "saved next proc mismatch"
        );
        ensure!(
            word(raw, 0x10) == word(saved.bytes(0x804D_781C, 4), 0),
            "saved current GObj mismatch"
        );
        let epoch = word(saved.bytes(0x804D_783C, 4), 0);
        ensure!(
            epoch <= 2 && u32::from((raw[13] >> 4) & 3) == epoch && raw[13] & 0xC0 == 0,
            "saved current proc flags mismatch"
        );
        ensure!(
            word(saved.bytes(0x804C_E3E4, 4), 0) == 0,
            "pending scheduler mutation unsupported"
        );
        let completed = saved_prefix(saved, link, current)?;
        let owner = word(raw, 0x10);
        let key = key(saved, raw)?;
        let action = continuation(saved, link, key, owner)?;
        Ok(Self {
            s_link: link as u8,
            current: Some((key, action)),
            completed,
        })
    }
}
fn saved_prefix(saved: &SavedPose, link: u32, current: u32) -> Result<Vec<ProcKey>> {
    let heads = word(saved.bytes(0x804D_7840, 4), 0);
    let mut pointer = word(saved.bytes(heads + link * 4, 4), 0);
    let mut previous = 0;
    let mut completed = Vec::new();
    let mut found = false;
    let mut count = 0;
    while pointer != 0 {
        count += 1;
        ensure!(count <= 512, "cyclic saved proc list");
        let proc = saved.bytes(pointer, 0x18);
        ensure!(
            u32::from(proc[12]) == link && word(proc, 8) == previous,
            "inconsistent saved proc list"
        );
        if pointer == current {
            found = true;
        } else if !found {
            completed.push(key(saved, proc)?);
        }
        previous = pointer;
        pointer = word(proc, 4);
    }
    ensure!(found, "current proc absent from saved list");
    Ok(completed)
}

fn continuation(saved: &SavedPose, link: u32, key: ProcKey, owner: u32) -> Result<Continuation> {
    let (registers, pc) = saved.cpu_general_registers()?;
    // gobj.c:112-115, retail 80390DF0..E10. At callback entry or return
    // the scheduler registers still name the cursor. This applies to every
    // s_link and owner; no callback-specific resume case is needed.
    if let Some(action) = callback_boundary(pc, key.callback) {
        ensure!(
            registers[27] == word(saved.bytes(0x804D_7838, 4), 0)
                && registers[28] == link
                && registers[24] == owner,
            "saved CPU and scheduler cursor disagree"
        );
        return Ok(action);
    }
    let stack = saved.stack_returns(registers[1])?;
    let has = |address| stack.iter().any(|&(_, caller)| caller == address);
    ensure!(has(0x8039_0E00), "callback is not inside HSD_GObj_80390CFC");
    let fighter = || saved.bytes(word(saved.bytes(owner + 0x2C, 4), 0), 0x23EC);
    let action = match (link, key.callback) {
        (1, 0x8006_A360) => {
            let restart = has(0x8006_AB78) && has(0x800D_5D64);
            let configuring = has(0x8006_9E78);
            ensure!(
                (has(0x8006_EB18) && (has(0x8006_EBBC) || (restart && has(0x8006_9E88))))
                    || (restart && configuring && has(0x8006_ED84)),
                "unsupported suspended animation PC {pc:08X}"
            );
            let raw = fighter();
            ensure!(
                word(raw, 0x10) == 14 && word(raw, 0x14) != u32::MAX,
                "suspended animation requires Wait"
            );
            Continuation::Animation {
                restart,
                configuring,
            }
        }
        (4, 0x8006_B82C) => {
            let raw = fighter();
            ensure!(
                pc == 0x8006_BF28 && registers[31] == word(saved.bytes(owner + 0x2C, 4), 0),
                "unsupported Fighter_procUpdate instruction/owner"
            );
            ensure!(word(raw, 0x18A4) == 0, "pending knockback magnitude clear");
            Continuation::Complete
        }
        (6, 0x8006_C27C) => {
            let raw = fighter();
            ensure!(
                has(0x8006_C3A8) && has(0x8008_A688) && has(0x8008_4394),
                "unsupported suspended collision PC {pc:08X}"
            );
            ensure!(
                word(raw, 0x10) == 14
                    && word(raw, 0x88C) == 0
                    && raw[0xB0..0xBC] == raw[0x6F4..0x700]
                    && raw[0x6F4..0x700] == raw[0x70C..0x718],
                "collision continuation requires stationary unlocked Wait"
            );
            let in_sweep = has(0x8004_39B4);
            if in_sweep {
                let index = stack.iter().position(|&(_, lr)| lr == 0x8004_39B4).unwrap();
                let frame = stack[index
                    .checked_sub(1)
                    .context("missing ground collision frame")?]
                .0;
                // mpColl_8004ACE4 saves r30/r31 (step count/index) at +38/+3C.
                // Only the final, contact-free step may restart its pure probes.
                ensure!(
                    word(saved.bytes(frame + 0x38, 4), 0) == 1
                        && word(saved.bytes(frame + 0x3C, 4), 0) == 0
                        && word(raw, 0x824) == 0
                        && raw[0x724] & 6 == 0
                        && ((has(0x8004_ADC0) && pc == 0x8004_DD08)
                            || (has(0x8004_ADF8) && pc == 0x8005_0F68)),
                    "unsupported partial collision sweep"
                );
            } else {
                ensure!(has(0x8004_B500), "unsupported collision loader");
            }
            Continuation::Collision { in_sweep }
        }
        (4, 0x8021_AAB0) => {
            ensure!(
                key.p_link == 5 && key.object == 3 && has(0x8021_AB14) && has(0x801C_3068),
                "unsupported FD controller instruction/owner"
            );
            ensure!(
                word(saved.bytes(0x804D_63B0, 4), 0) == 0,
                "pending dynamic quake list"
            );
            Continuation::GroundCollision
        }
        (14, 0x8006_D1EC) => Continuation::Invoke,
        (15, 0x8005_C9A4) => Continuation::ParticleEmission,
        (17, 0x802F_9410) => {
            let user = word(saved.bytes(owner + 0x2C, 4), 0);
            let hud = saved.bytes(user, 12);
            ensure!(hud[0] < 6 && hud[1] <= 1, "unsupported stock HUD mode");
            if hud[1] == 0 {
                let stocks = word(
                    saved.bytes(0x804A_1378 + 0x54 + u32::from(hud[0]) * 0x50, 4),
                    0,
                ) as usize;
                ensure!(
                    (1..=5).contains(&stocks)
                        && (hud[2] == 0 || hud[5 + stocks..10].iter().all(|&v| v != 0))
                        && hud[10..12] == [0, 0],
                    "pending stock HUD effects: {hud:?}, stocks {stocks}"
                );
            }
            Continuation::Complete
        }
        _ => anyhow::bail!(
            "unsupported scheduler resume boundary: link {link}, callback {:08X}, PC {pc:08X}",
            key.callback
        ),
    };
    Ok(action)
}

fn callback_boundary(pc: u32, callback: u32) -> Option<Continuation> {
    if pc == callback || (0x8039_0DF0..=0x8039_0DFC).contains(&pc) {
        Some(Continuation::Invoke)
    } else if (0x8039_0E00..=0x8039_0E10).contains(&pc) {
        Some(Continuation::Complete)
    } else {
        None
    }
}

fn key(saved: &SavedPose, proc: &[u8]) -> Result<ProcKey> {
    let owner = saved.bytes(word(proc, 0x10), 0x30);
    let p_link = owner[2];
    let user = word(owner, 0x2C);
    let object = match p_link {
        5 if user != 0 => u8::try_from(word(saved.bytes(user + 0x14, 4), 0)).context("map id")?,
        8 => saved.bytes(user + 12, 1)[0],
        15 if word(proc, 0x14) == 0x802F_9410 => saved.bytes(user, 1)[0],
        _ => u8::MAX,
    };
    Ok(ProcKey {
        p_link,
        object,
        callback: word(proc, 0x14),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fighter(player: u8) -> ProcKey {
        ProcKey {
            p_link: 8,
            object: player,
            callback: 0x8006_A360,
        }
    }
    #[test]
    fn current_owner_controls_the_partial_tick() {
        for player in 0..2 {
            let current = fighter(player);
            let resume = SchedulerResume {
                s_link: 1,
                completed: (0..player).map(fighter).collect(),
                current: Some((
                    current,
                    Continuation::Animation {
                        restart: false,
                        configuring: false,
                    },
                )),
            };
            assert_eq!(resume.action(0, current), Continuation::Complete);
            for earlier in 0..player {
                assert_eq!(resume.action(1, fighter(earlier)), Continuation::Complete);
            }
            assert!(matches!(
                resume.action(1, current),
                Continuation::Animation { .. }
            ));
            for later in player + 1..2 {
                assert_eq!(resume.action(1, fighter(later)), Continuation::Invoke);
            }
            assert_eq!(resume.action(2, current), Continuation::Invoke);
        }
    }
    #[test]
    fn callbacks_on_the_same_ground_object_have_distinct_positions() {
        let wrapper = ProcKey {
            p_link: 5,
            object: 3,
            callback: 0x801C_1D38,
        };
        let controller = ProcKey {
            callback: 0x8021_AAB0,
            ..wrapper
        };
        let resume = SchedulerResume {
            s_link: 4,
            completed: vec![wrapper],
            current: Some((controller, Continuation::GroundCollision)),
        };
        assert_eq!(resume.action(4, wrapper), Continuation::Complete);
        assert_eq!(resume.action(4, controller), Continuation::GroundCollision);
        assert_eq!(
            resume.action(
                4,
                ProcKey {
                    object: 4,
                    ..wrapper
                }
            ),
            Continuation::Invoke
        );
        assert_eq!(resume.action(4, fighter(0)), Continuation::Invoke);
    }
    #[test]
    fn scheduler_call_and_return_boundaries_do_not_depend_on_callback() {
        for callback in [0x8006_A360, 0x8006_AD10, 0x8006_C27C, 0x8021_AAB0] {
            assert_eq!(
                callback_boundary(callback, callback),
                Some(Continuation::Invoke)
            );
            assert_eq!(
                callback_boundary(0x8039_0DFC, callback),
                Some(Continuation::Invoke)
            );
            assert_eq!(
                callback_boundary(0x8039_0E00, callback),
                Some(Continuation::Complete)
            );
            assert_eq!(callback_boundary(callback + 4, callback), None);
        }
    }
    fn memory(next: u32, previous: u32) -> SavedPose {
        let mut memory = vec![0; 0x4D_7844];
        let mut put = |address: u32, value: u32| {
            let start = (address - 0x8000_0000) as usize;
            memory[start..start + 4].copy_from_slice(&value.to_be_bytes());
        };
        put(0x804D_7840, 0x8000_0100);
        put(0x8000_0104, 0x8000_0200);
        put(0x8000_0204, next);
        put(0x8000_020C, 0x0100_0000);
        put(0x8000_0210, 0x8000_0400);
        put(0x8000_0214, 0x8006_A360);
        put(0x8000_0308, previous);
        put(0x8000_030C, 0x0100_0000);
        put(0x8000_0310, 0x8000_0400);
        put(0x8000_0314, 0x8006_A360);
        put(0x8000_0400, 0x0000_0800);
        put(0x8000_042C, 0x8000_0500);
        SavedPose::test_memory(memory)
    }
    #[test]
    fn saved_list_checks_both_directions_and_current_membership() {
        let valid = memory(0x8000_0300, 0x8000_0200);
        assert_eq!(saved_prefix(&valid, 1, 0x8000_0300).unwrap(), [fighter(0)]);
        assert!(saved_prefix(&valid, 1, 0x8000_0600)
            .unwrap_err()
            .to_string()
            .contains("absent"));
        let broken = memory(0x8000_0300, 0);
        assert!(saved_prefix(&broken, 1, 0x8000_0300)
            .unwrap_err()
            .to_string()
            .contains("inconsistent"));
        let cycle = memory(0x8000_0200, 0);
        assert!(saved_prefix(&cycle, 1, 0x8000_0200).is_err());
    }
}
