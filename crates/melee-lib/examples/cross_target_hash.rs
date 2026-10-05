//! Cross-target determinism probe: seeded random pads over a few matches,
//! printing one FNV-1a hash per tick of `diagnostics::inspect` plus the
//! public observation. `tools/wasm-check.sh` runs it natively and under
//! wasmtime (wasm32-wasip1) and diffs the output; any line that differs is
//! a target-dependent result.
//!
//! ```sh
//! cargo run --release -p melee-lib --example cross_target_hash -- harness/roms/files [ticks]
//! ```
use melee_diff::{Record, Value};
use melee_lib::{
    diagnostics, Character as C, ControllerState, GameAssets, Inputs, Match, MatchConfig,
    MatchStatus, PlayerConfig, Port, Seed, Stage as S,
};

struct Fnv(u64);
impl Fnv {
    fn new() -> Self {
        Fnv(0xcbf2_9ce4_8422_2325)
    }
    fn bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.0 ^= u64::from(b);
            self.0 = self.0.wrapping_mul(0x0100_0000_01b3);
        }
    }
    fn u64(&mut self, v: u64) {
        self.bytes(&v.to_le_bytes())
    }
    fn record(&mut self, record: &Record) {
        self.u64(record.frame);
        self.bytes(record.phase.as_bytes());
        for (key, value) in &record.state {
            self.bytes(key.as_bytes());
            self.bytes(&[0]);
            match value {
                Value::Int(i) => self.u64(*i as u64),
                Value::UInt(u) => self.u64(*u),
                Value::F32 { bits, .. } => self.u64(u64::from(*bits)),
                Value::F64 { bits, .. } => self.u64(*bits),
                Value::Str(s) => self.bytes(s.as_bytes()),
                Value::Null => self.bytes(b"n"),
            }
        }
    }
}

/// xorshift64: the same pad sequence on every target.
struct Rng(u64);
impl Rng {
    fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % n
    }
}

fn random_pad(rng: &mut Rng) -> ControllerState {
    let mut pad = ControllerState::default();
    // Cardinal extremes and neutral, plus arbitrary analog values.
    let axis = |rng: &mut Rng| match rng.below(6) {
        0 => 0,
        1 => 127,
        2 => -128,
        _ => rng.below(256) as u8 as i8,
    };
    let (x, y) = (axis(rng), axis(rng));
    pad.stick = melee_ft::input::pad::normalize_stick(x, y);
    if rng.below(5) == 0 {
        let (x, y) = (axis(rng), axis(rng));
        pad.cstick = melee_ft::input::pad::normalize_stick(x, y);
    }
    // A B X Y Z R L; no Start or d-pad.
    const BUTTONS: [u32; 7] = [0x100, 0x200, 0x400, 0x800, 0x10, 0x20, 0x40];
    let bits = BUTTONS.iter().filter(|_| rng.below(7) == 0).fold(0, |a, b| a | b);
    pad.buttons = melee_lib::Buttons(bits);
    if rng.below(8) == 0 {
        pad.right_trigger = rng.below(141) as u8 as f32 / 140.0;
    }
    pad
}

const CONFIGS: [(&str, S, C, C); 3] = [
    ("fd_fox_marth", S::FinalDestination, C::Fox, C::Marth),
    ("ps_peach_samus", S::PokemonStadium, C::Peach, C::Samus),
    ("ys_iceclimbers_link", S::YoshisStory, C::IceClimbers, C::Link),
];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let files = args.get(1).expect("usage: cross_target_hash <harness/roms/files> [ticks]");
    let ticks: u64 = args.get(2).map_or(2000, |s| s.parse().expect("ticks"));
    for (name, stage, a, b) in CONFIGS {
        let mut seed = Fnv::new();
        seed.bytes(name.as_bytes());
        let config = MatchConfig::versus(stage, [PlayerConfig::new(Port::P1, a), PlayerConfig::new(Port::P2, b)])
            .with_seed(Seed(seed.0 as u32));
        let assets = GameAssets::load(files, &config).expect("assets");
        println!("{name} assets {:016x}", diagnostics::asset_fingerprint(&assets));
        let mut game = Match::new(&assets, config).expect("match");
        let mut rng = Rng(seed.0 | 1);
        let mut held = [(ControllerState::default(), 0u64); 2];
        let mut resets = 0;
        for tick in 0..ticks {
            match game.status() {
                MatchStatus::Faulted => {
                    println!("{name} {tick} fault");
                    break;
                }
                MatchStatus::Finished(_) => {
                    resets += 1;
                    game.reset(Seed((seed.0 as u32).wrapping_add(resets))).expect("reset");
                }
                _ => {}
            }
            let mut inputs = Inputs::default();
            for (p, port) in [Port::P1, Port::P2].into_iter().enumerate() {
                if held[p].1 == 0 {
                    held[p] = (random_pad(&mut rng), 1 + rng.below(18));
                }
                held[p].1 -= 1;
                inputs[port] = held[p].0;
            }
            if let Err(e) = game.step(&inputs) {
                println!("{name} {tick} step error: {e}");
                break;
            }
            println!("{name} {tick} {:016x}", tick_hash(&game));
        }
    }
}

fn tick_hash(game: &Match) -> u64 {
    let mut h = Fnv::new();
    let inspect = diagnostics::inspect(game).expect("inspect");
    h.record(&inspect.fighters);
    h.record(&inspect.items);
    h.record(&inspect.particles);
    for site in inspect.particle_rng_sites.iter().chain(&inspect.effect_rng_sites) {
        h.u64(*site as u64);
    }
    for (writer, count) in &inspect.rng_writers {
        h.bytes(writer.as_bytes());
        h.u64(*count as u64);
    }
    for value in inspect.bones.iter().flatten().flatten() {
        h.u64(*value as u64);
    }
    let view = game.observe().expect("observe");
    for f in view.fighters() {
        let p = f.position();
        for x in [p.x, p.y, p.z, f.percent(), f.shield_health(), f.animation_frame()] {
            h.u64(u64::from(x.to_bits()));
        }
        h.u64(u64::from(f.action().0));
    }
    for item in view.items() {
        let p = item.position();
        h.u64(item.kind().0 as u64);
        for x in [p.x, p.y, p.z] {
            h.u64(u64::from(x.to_bits()));
        }
    }
    for v in view.vertices() {
        h.u64(u64::from(v.x.to_bits()));
        h.u64(u64::from(v.y.to_bits()));
    }
    for s in view.surfaces() {
        h.u64(u64::from(s.id.0));
    }
    h.0
}
