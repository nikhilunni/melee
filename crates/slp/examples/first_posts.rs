//! Print each leader's first-frame position and facing as f32 bits.
fn main() -> anyhow::Result<()> {
    for path in std::env::args().skip(1) {
        let replay = slp::Replay::parse(&std::fs::read(&path)?)?;
        let first = replay.frames.values().next().unwrap();
        let posts: Vec<String> = replay
            .leader_ports()
            .map(|p| {
                let post = first.ports[p].leader.post.as_ref().unwrap();
                format!(
                    "port{} ({:08X},{:08X}) facing {}",
                    p + 1,
                    post.position_x.to_bits(),
                    post.position_y.to_bits(),
                    post.facing_direction
                )
            })
            .collect();
        println!(
            "{} stage {} {}",
            path.rsplit('/').next().unwrap(),
            replay.start.stage,
            posts.join(" ")
        );
    }
    Ok(())
}
