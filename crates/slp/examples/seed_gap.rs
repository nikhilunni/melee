//! For each replay: HSD LCG steps from the Game Start seed to the first
//! Frame Start seed and to each leader's first Pre Frame seed.
use slp::Replay;

fn steps(from: u32, to: u32, limit: u32) -> Option<u32> {
    let mut s = from;
    for n in 0..=limit {
        if s == to {
            return Some(n);
        }
        s = s.wrapping_mul(214013).wrapping_add(2531011);
    }
    None
}

fn main() -> anyhow::Result<()> {
    for path in std::env::args().skip(1) {
        let replay = Replay::parse(&std::fs::read(&path)?)?;
        let gs = replay.start.random_seed;
        let first = replay.frames.values().next().unwrap();
        let fs = first.scheduler_start_seed().map(|s| steps(gs, s, 100_000));
        let pre: Vec<_> = replay
            .leader_ports()
            .map(|p| {
                first.ports[p]
                    .leader
                    .pre
                    .as_ref()
                    .map(|pre| steps(gs, pre.random_seed, 100_000))
            })
            .collect();
        let fixes: Vec<_> = replay
            .leader_ports()
            .map(|p| {
                (
                    replay.start.players[p].dashback_fix,
                    replay.start.players[p].shield_drop_fix,
                )
            })
            .collect();
        println!(
            "fixes {fixes:?} items {} {} v{:?} stage {} online {} gs {gs:08X} frame_start {fs:?} pre {pre:?}",
            replay.start.item_spawn_behavior,
            path.rsplit('/').next().unwrap(),
            replay.version(),
            replay.start.stage,
            replay.start.is_online()
        );
    }
    Ok(())
}
