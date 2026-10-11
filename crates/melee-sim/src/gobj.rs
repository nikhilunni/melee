//! GObj address identity against retail (`melee-sim gobj-diff`).
//!
//! Retail takes a new GObj's address from the allocator's free list
//! (`hsd_gobj::GObjAddresses`), and a hitbox's victim list keeps addresses
//! past their objects' lives, so which objects share an address is game
//! state. The port keeps identities, not addresses: it agrees with retail
//! when one renaming turns its identities into retail's addresses.
//!
//! Two captures show retail's side. Every tick trace lists each item's
//! GObj; a GObj ledger (`harness/gobj_ledger_record.py`) lists every
//! creation and destruction in order.
use crate::scenario::Scenario;
use anyhow::{Context, Result};
use std::collections::BTreeMap;
use std::io::BufRead;
use std::path::Path;

/// HSD_ObjAlloc (0x8037ABC8, 0x158 bytes) and HSD_ObjFree (0x8037AD20,
/// 0x28): the functions whose free-list stores the ledger records.
/// HSD_ObjAllocAddFree's store (new memory) is not an event of its own.
const ALLOC: std::ops::Range<u64> = 0x8037_ABC8..0x8037_AD20;
const FREE: std::ops::Range<u64> = 0x8037_AD20..0x8037_AD48;

pub struct Report {
    pub differences: Vec<String>,
    pub summary: String,
}

/// The renaming between the port's identities and retail's addresses,
/// built as objects appear; a pair that contradicts it is a difference.
#[derive(Default)]
struct Renaming {
    address_of: BTreeMap<u32, u32>,
    identity_of: BTreeMap<u32, u32>,
}
impl Renaming {
    /// Whether `identity` may stand for `address`. A contradiction replaces
    /// the earlier pairs, so one fault is reported once.
    fn agree(&mut self, identity: u32, address: u32) -> bool {
        let known_address = self.address_of.get(&identity).copied();
        let known_identity = self.identity_of.get(&address).copied();
        let agrees = known_address.is_none_or(|a| a == address)
            && known_identity.is_none_or(|i| i == identity);
        if !agrees {
            if let Some(a) = known_address {
                self.identity_of.remove(&a);
            }
            if let Some(i) = known_identity {
                self.address_of.remove(&i);
            }
        }
        self.address_of.insert(identity, address);
        self.identity_of.insert(address, identity);
        agrees
    }
}

/// The `"key": [...]` array of one trace line, parsed alone (a tick
/// record's fighter images are megabytes the comparison does not need).
fn array_field(line: &str, key: &str) -> Result<Vec<serde_json::Value>> {
    let Some(at) = line.find(&format!("\"{key}\": [")) else {
        return Ok(Vec::new());
    };
    let start = at + key.len() + 4;
    let mut values = serde_json::Deserializer::from_str(&line[start..]).into_iter();
    let value: serde_json::Value = values
        .next()
        .with_context(|| format!("empty {key}"))?
        .with_context(|| format!("parsing {key}"))?;
    Ok(value.as_array().cloned().unwrap_or_default())
}

fn address(value: &serde_json::Value) -> Result<u32> {
    let text = value.as_str().context("GObj address")?;
    Ok(u32::from_str_radix(text.trim_start_matches("0x"), 16)?)
}

/// Each tick's items against the tick trace's item GObjs: the same kinds in
/// the same order, and one renaming of identities to addresses over the
/// whole scenario.
pub fn item_diff(scenario: &Scenario) -> Result<Report> {
    let path = scenario.trace_path("tick.raw.jsonl");
    let mut simulation = crate::trace::simulation_displayed_as(scenario, &path)?;
    let mut renaming = Renaming::default();
    let mut differences = Vec::new();
    let mut live: Vec<u32> = Vec::new();
    let (mut items, mut reused) = (0usize, 0usize);
    let mut seen = std::collections::BTreeSet::new();
    for (tick, line) in melee_trace_io::open(&path)?.lines().enumerate() {
        let line = line?;
        simulation.tick_without_snapshot()?;
        let retail: Vec<(u32, i64)> = array_field(&line, "items")?
            .iter()
            .map(|item| {
                Ok((
                    address(&item["gobj"])?,
                    item["kind"].as_i64().context("kind")?,
                ))
            })
            .collect::<Result<_>>()?;
        let port = simulation.item_gobjs();
        if port.len() != retail.len() {
            differences.push(format!(
                "tick {tick}: {} items, retail has {}",
                port.len(),
                retail.len()
            ));
            break;
        }
        for (index, (&(identity, _), &(address, _))) in port.iter().zip(&retail).enumerate() {
            if !live.contains(&address) {
                items += 1;
                if !seen.insert(address) {
                    reused += 1;
                }
            }
            if !renaming.agree(identity, address) {
                differences.push(format!(
                    "tick {tick}: item {index} at 0x{address:08X} is not the port's object {identity}"
                ));
            }
        }
        live = retail.iter().map(|&(address, _)| address).collect();
    }
    Ok(Report {
        summary: format!(
            "{items} items, {reused} at an address used before, {} differences",
            differences.len()
        ),
        differences,
    })
}

/// One retail event: created or destroyed, the object's classifier (zero
/// when the ledger could not read it) and its address.
fn retail_events(
    line: &str,
    classifiers: &mut BTreeMap<u32, u16>,
) -> Result<Vec<(bool, u16, u32)>> {
    let mut events = Vec::new();
    for event in array_field(line, "gobj")? {
        let pc = event["pc"].as_u64().context("pc")?;
        if ALLOC.contains(&pc) {
            let address = event["old"].as_u64().context("old")? as u32;
            // The object's first word: classifier, p_link, gx_link. An
            // object freed before the ledger read it shows the free list's
            // link there instead (a MEM1 pointer): unknown.
            let word = event["word"].as_u64().unwrap_or(0);
            let classifier = if word >> 31 == 0 {
                (word >> 16) as u16
            } else {
                0
            };
            classifiers.insert(address, classifier);
            events.push((true, classifier, address));
        } else if FREE.contains(&pc) {
            let address = event["head"].as_u64().context("head")? as u32;
            let classifier = classifiers.remove(&address).unwrap_or(0);
            events.push((false, classifier, address));
        }
    }
    drop_hud_rebuilds(&mut events, classifiers);
    Ok(events)
}

/// HSD_GOBJ_CLASS_UI.
const HUD: u16 = 0xE;

/// A HUD model rebuilt in place (a player's stock icons when it loses a
/// stock or returns): the old GObj is destroyed, a scratch GObj is created
/// and destroyed, and the new GObj is created, four consecutive events on
/// one address. They leave the free list and every other object as they
/// were, and the port models no HUD GObj, so they are not compared.
fn drop_hud_rebuilds(events: &mut Vec<(bool, u16, u32)>, classifiers: &mut BTreeMap<u32, u16>) {
    let mut at = 0;
    while at + 4 <= events.len() {
        let run = &events[at..at + 4];
        let address = run[0].2;
        let rebuilt = run.iter().all(|event| event.2 == address)
            && [false, true, false, true]
                .iter()
                .zip(run)
                .all(|(created, event)| event.0 == *created)
            && run[0].1 == HUD
            && run[3].1 == HUD;
        if rebuilt {
            events.drain(at..at + 4);
            // The rebuilt model lives on at the address.
            classifiers.insert(address, HUD);
        } else {
            at += 1;
        }
    }
}

/// Every creation and destruction of a GObj ledger against the port's, tick
/// by tick: the same events in the same order under one renaming. An
/// object the match's first tick creates and never frees is outside the
/// comparison (the port starts with the free list those leave empty).
pub fn ledger_diff(scenario: &Scenario, ledger: &Path) -> Result<Report> {
    let mut simulation =
        crate::trace::simulation_displayed_as(scenario, &scenario.trace_path("tick.raw.jsonl"))?;
    let mut renaming = Renaming::default();
    let mut classifiers = BTreeMap::new();
    let mut differences = Vec::new();
    let mut events = 0usize;
    let show = |list: &[(bool, u16, u32)], hex: bool| {
        list.iter()
            .map(|&(created, classifier, object)| {
                let op = if created { '+' } else { '-' };
                if hex {
                    format!("{op}{classifier:X}@{:05X}", object & 0xF_FFFF)
                } else {
                    format!("{op}{classifier:X}#{object}")
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    for (tick, line) in melee_trace_io::open(ledger)?.lines().enumerate() {
        let line = line?;
        simulation.tick_without_snapshot()?;
        let retail = retail_events(&line, &mut classifiers)?;
        let port = simulation.take_gobj_events();
        if tick == 0 {
            // Scene set-up before the first tick: nothing of the match.
            continue;
        }
        events += retail.len();
        let same_shape = port.len() == retail.len()
            && port
                .iter()
                .zip(&retail)
                .all(|(p, r)| p.0 == r.0 && (r.1 == 0 || p.1 == r.1));
        let mut same_objects = same_shape;
        if same_shape {
            // Every pair is entered, also after the first disagreement.
            for (p, r) in port.iter().zip(&retail) {
                same_objects &= renaming.agree(p.2, r.2);
            }
        }
        if !same_objects {
            differences.push(format!(
                "tick {tick}: port [{}] retail [{}]",
                show(&port, false),
                show(&retail, true)
            ));
            if !same_shape {
                // Keep following retail's objects where the port has them.
                for (p, r) in port.iter().zip(&retail) {
                    if p.0 == r.0 {
                        renaming.agree(p.2, r.2);
                    }
                }
            }
        }
    }
    Ok(Report {
        summary: format!("{events} events, {} differing ticks", differences.len()),
        differences,
    })
}
