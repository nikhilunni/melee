# melee-lib

A deterministic, headless match library. Applications own input acquisition,
rendering, clocks, episode limits, policies, rewards, and worker scheduling.
Subsystem crates retain their own implementations; this crate owns composition
and the supported application API.

```rust,no_run
use melee_lib::*;
let config = MatchConfig::versus(Stage::FinalDestination, [
    PlayerConfig::new(Port::P1, Character::Fox),
    PlayerConfig::new(Port::P2, Character::Marth),
]).with_seed(Seed(42));
let assets = GameAssets::load("harness/roms/files", &config)?;
let mut game = Match::new(&assets, config)?;
game.step(&Inputs::default())?;
let branch = game.clone();
let observation = game.observe()?;
assert_eq!(observation.tick, Tick(1));
# Ok::<(), Box<dyn std::error::Error>>(())
```

`GameAssets` loads files once and shares immutable resources across compatible
matches and threads. Compatibility currently means the same stage and ordered
pair of characters; costumes, occupied ports, stocks, unlock state, and seeds
may differ. Players must occupy two distinct ascending physical ports. The
initial composition supports singles stock matches, no random items, normal
damage, and no time limit. Registered characters and stages have varying move
coverage; registration does not imply a complete port.

Each successful `step` advances one complete scheduler tick, including the
startup countdown. There is no automatic repetition or jump to playable state.
`Tick(0)` is the state after construction. `Seed` preserves the existing retail
post-creation, pre-music boundary: construction reproduces setup and consumes
music-selection draws before the first public tick. The unstable oracle adapter
retains its original partial-boundary tick zero and trace indexing explicitly.

`Inputs` contains four normalized controller samples, indexed by `Port`, and
starts neutral. Stick axes must be finite and within -1..=1, triggers within
0..=1, and button bits must be recognized HSD bits. The raw adapter
`ControllerState::from_origin_adjusted` uses the existing retail normalization
helpers on SDK origin-calibrated signed sticks and unsigned trigger samples.
Keyboard mapping and device calibration belong in applications.

Invalid inputs are rejected before mutation. The tick that eliminates the last
opponent succeeds; further steps reject the finished match. Simulation errors
or unwinding panics fault the match; observation and stepping then reject it.
A reset or assignment from a healthy clone recovers it. As elsewhere in Rust,
an aborting panic or process allocation failure cannot be recovered by this API.
Reset retains assets and rules, installs the new seed, and returns to tick zero;
failed reset leaves the original match intact.

Observations borrow read-only facts without allocating or computing presentation
poses. Fighter, combat, item, and world-space stage views do not prescribe a
training tensor. Immutable borrowing prevents advancement while a view is live.
`Match: Send` and `GameAssets: Send + Sync`; there is no global mutable match
cache or internal thread pool. Advancing requires exclusive ownership.

Cloning copies scheduler order and slot generations, input state, character
payloads, animations, RNG, pending work, items, particles, and terminal/fault
state. Prepared capacities survive cloning so subsequent stepping remains
allocation-free. `clone_from` has ordinary Rust semantics, replacing the entire
configuration and resource handle even across different match configurations.
It currently performs a full replacement; it does not promise allocation reuse.
Creation, reset, and cloning may allocate.

Particle and effect computations remain part of headless simulation because
they affect ordered RNG draws. They must not be disabled as a rendering
optimization. The explicitly unstable `diagnostics` module supplies allocating
snapshots and captured-boundary import for oracle tools. It is not a supported
application state-editing API.

Run the public consumers with local, untracked game files:

```
cargo run --release -p melee-lib --example headless -- harness/roms/files
cargo run --release -p melee-lib --example training -- harness/roms/files
cargo bench -p melee-lib --bench lifecycle
```

The lifecycle benchmark measures creation, reset, clone, and clone_from
separately from disk loading and stepping, and reports incremental retained
heap for 1, 8, and 32 matches sharing assets. Creation/clone timings include drop;
retained heap excludes allocator overhead and the shared asset set.

The graphical consumer will be a separate `melee-platform` library with session
and presentation behavior, a C ABI for Swift, and wgpu rendering. It is outside
this milestone.
