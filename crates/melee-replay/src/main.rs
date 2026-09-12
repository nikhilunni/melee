use melee_lib::GameAssets;
use melee_replay::Recording;
use std::path::Path;

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: melee-replay <extracted-files-directory> <replay.json>".into());
    }
    let recording = Recording::load(Path::new(&args[1]))?;
    let config = recording.config.decode()?;
    let assets = GameAssets::load(Path::new(&args[0]), &config).map_err(|e| e.to_string())?;
    match recording.replay(&assets) {
        Ok(game) => {
            println!("replayed {} ticks; {:?}", game.tick().0, game.status());
            Ok(())
        }
        Err(fault) => Err(format!(
            "attempted tick {}: {}",
            fault.attempt, fault.message
        )),
    }
}
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
