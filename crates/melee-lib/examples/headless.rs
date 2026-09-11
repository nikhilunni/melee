//! cargo run -p melee-lib --release --example headless -- harness/roms/files
use melee_lib::{
    Character, GameAssets, Inputs, Match, MatchConfig, PlayerConfig, Port, Seed, Stage,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args()
        .nth(1)
        .ok_or("usage: headless <game-files-directory>")?;
    let config = MatchConfig::versus(
        Stage::FinalDestination,
        [
            PlayerConfig::new(Port::P1, Character::Fox),
            PlayerConfig::new(Port::P2, Character::Marth),
        ],
    )
    .with_seed(Seed(42));
    let assets = GameAssets::load(directory, &config)?;
    let mut game = Match::new(&assets, config)?;
    // A caller-owned budget makes this example terminate even with neutral pads.
    for _ in 0..600 {
        if !game.status().is_running() {
            break;
        }
        let mut inputs = Inputs::default();
        if game.tick().0 > 120 {
            inputs[Port::P1].stick.x = 1.0;
        }
        game.step(&inputs)?;
    }
    let view = game.observe()?;
    println!("tick {}: {:?}", view.tick.0, view.status);
    for fighter in view.fighters() {
        println!(
            "{:?} {:?}: position {:?}, {}%, {} stocks",
            fighter.port(),
            fighter.character(),
            fighter.position(),
            fighter.percent(),
            fighter.stocks()
        );
    }
    Ok(())
}
