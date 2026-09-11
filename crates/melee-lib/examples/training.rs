//! A caller-owned policy, rollout horizon, score, and episode loop.
//! cargo run -p melee-lib --release --example training -- harness/roms/files
use melee_lib::{
    Character, GameAssets, Inputs, Match, MatchConfig, Observation, PlayerConfig, Port, Seed, Stage,
};

fn score(view: &Observation<'_>) -> f32 {
    let us = view.fighter(Port::P1).unwrap();
    let opponent = view.fighter(Port::P2).unwrap();
    // This example's objective belongs to the consumer, not the simulation.
    -(us.position().x - opponent.position().x).abs()
}
fn choose_action(game: &Match) -> Result<Inputs, Box<dyn std::error::Error>> {
    let mut best = (f32::NEG_INFINITY, Inputs::default());
    for direction in [-1.0, 0.0, 1.0] {
        let mut inputs = Inputs::default();
        inputs[Port::P1].stick.x = direction;
        let mut branch = game.clone();
        for _ in 0..4 {
            if !branch.status().is_running() {
                break;
            }
            branch.step(&inputs)?;
        }
        let value = score(&branch.observe()?);
        if value > best.0 {
            best = (value, inputs);
        }
    }
    Ok(best.1)
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = std::env::args()
        .nth(1)
        .ok_or("usage: training <game-files-directory>")?;
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
    for episode in 0..3 {
        game.reset(Seed(episode))?;
        for _ in 0..180 {
            if !game.status().is_running() {
                break;
            }
            let action = choose_action(&game)?;
            game.step(&action)?;
        }
        println!(
            "episode {episode}: tick {}, score {}",
            game.tick().0,
            score(&game.observe()?)
        );
    }
    Ok(())
}
