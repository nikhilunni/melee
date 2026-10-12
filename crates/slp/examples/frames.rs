//! A replay as JSON lines for `harness/replay_vs_trace.py`: a header with the
//! Slippi version and Game Start seed, then one line per frame holding
//! the harness frame (0 is Slippi's -123) and, per recorded fighter in port
//! order (leader, then follower), Pre Frame's seed and Post Frame's internal
//! kind, action, position bits and percent bits, and Pre Frame's inputs
//! (`pad`): the fighter's dead-zoned sticks and trigger as float bits
//! (Fighter +0x620, +0x624, +0x638, +0x63C, +0x650), its held word (+0x65C),
//! the pad's physical buttons and analog triggers, and the raw stick bytes
//! the Slippi version records.
//!
//!   cargo run -q --release -p slp --example frames -- <game.slp> > frames.jsonl
use serde_json::json;

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).expect("usage: frames <game.slp>");
    let replay = slp::Replay::parse(&std::fs::read(&path)?)?;
    let ports: Vec<usize> = replay.leader_ports().collect();
    let version = replay.version();
    println!(
        "{}",
        json!({
            "version": [version.major, version.minor, version.build],
            "game_start_seed": replay.start.random_seed,
        })
    );
    for frame in replay.frames.values() {
        let mut fighters = Vec::new();
        for &port in &ports {
            let both = &frame.ports[port];
            for (follower, player) in [(false, &both.leader), (true, &both.follower)] {
                let (Some(pre), Some(post)) = (&player.pre, &player.post) else {
                    continue;
                };
                fighters.push(json!({
                    "slot": port,
                    "follower": follower,
                    "seed": pre.random_seed,
                    "kind": post.internal_character,
                    "action": post.action_state,
                    "x": post.position_x.to_bits(),
                    "y": post.position_y.to_bits(),
                    "percent": post.percent.to_bits(),
                    "pad": {
                        "stick": [pre.joystick_x.to_bits(), pre.joystick_y.to_bits()],
                        "cstick": [pre.cstick_x.to_bits(), pre.cstick_y.to_bits()],
                        "trigger": pre.trigger.to_bits(),
                        "held": pre.buttons_processed,
                        "physical": pre.buttons_physical,
                        "triggers": [
                            pre.physical_l_trigger.to_bits(),
                            pre.physical_r_trigger.to_bits(),
                        ],
                        "raw_stick": [pre.raw_joystick_x, pre.raw_joystick_y],
                        "raw_cstick": [pre.raw_cstick_x, pre.raw_cstick_y],
                    },
                }));
            }
        }
        println!(
            "{}",
            json!({
                "frame": slp::harness_frame(frame.number),
                "fighters": fighters,
            })
        );
    }
    Ok(())
}
