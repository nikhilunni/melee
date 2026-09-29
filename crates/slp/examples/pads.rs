//! Print a port's pre-frame pad fields over a frame range.
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let replay = slp::Replay::parse(&std::fs::read(&args[1])?)?;
    let port: usize = args[2].parse()?;
    let from: i32 = args[3].parse()?;
    let to: i32 = args[4].parse()?;
    for n in from..=to {
        let f = &replay.frames[&(n + slp::SLIPPI_FIRST_FRAME)];
        let pre = f.ports[port].leader.pre.as_ref().unwrap();
        let post = f.ports[port].leader.post.as_ref().unwrap();
        println!(
            "tick {n:4} phys {:04X} proc {:08X} stick ({:.4},{:.4}) raw {:?} c ({:.3},{:.3}) trig {:.3} L {:.3} R {:.3} | action {} frame {:?}",
            pre.buttons_physical, pre.buttons_processed, pre.joystick_x, pre.joystick_y, pre.raw_joystick_x,
            pre.cstick_x, pre.cstick_y, pre.trigger, pre.physical_l_trigger, pre.physical_r_trigger,
            post.action_state, post.action_state_frame
        );
    }
    Ok(())
}
