//! What the menus offer. Ids are indices into these lists and are stable
//! within a build: characters in the retail character select screen's
//! reading order, stages in the stage select order.
use melee_lib::{Character, Stage};

pub fn characters() -> &'static [Character] {
    &Character::ALL
}
pub fn stages() -> &'static [Stage] {
    &Stage::ALL
}
pub fn character_id(character: Character) -> u32 {
    characters()
        .iter()
        .position(|&c| c == character)
        .expect("every character is in the catalog") as u32
}
pub fn character(id: u32) -> Option<Character> {
    characters().get(id as usize).copied()
}
pub fn stage_id(stage: Stage) -> u32 {
    stages()
        .iter()
        .position(|&s| s == stage)
        .expect("every stage is in the catalog") as u32
}
pub fn stage(id: u32) -> Option<Stage> {
    stages().get(id as usize).copied()
}
/// Characters per row of the retail character select grid, in catalog
/// order. Retail rows hold 9, 9 and 7 icons; here Sheik has her own cell
/// (retail shares Zelda's) and fills the middle row's Kirby gap.
pub const ROWS: [usize; 3] = [9, 9, 7];
/// Grid row and column of a character id.
pub fn grid_cell(id: u32) -> Option<(u32, u32)> {
    let mut start = 0;
    for (row, &len) in ROWS.iter().enumerate() {
        if (id as usize) < start + len {
            return Some((row as u32, id - start as u32));
        }
        start += len;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_round_trip_and_unknown_ids_are_rejected() {
        for &c in characters() {
            assert_eq!(character(character_id(c)), Some(c));
        }
        for &s in stages() {
            assert_eq!(stage(stage_id(s)), Some(s));
        }
        assert_eq!(character(characters().len() as u32), None);
        assert_eq!(stage(99), None);
    }
    #[test]
    fn the_grid_has_retail_rows() {
        assert_eq!(ROWS.iter().sum::<usize>(), characters().len());
        let cell = |c| grid_cell(character_id(c)).unwrap();
        assert_eq!(cell(Character::DrMario), (0, 0));
        assert_eq!(cell(Character::Ganondorf), (0, 8));
        assert_eq!(cell(Character::Falco), (1, 0));
        assert_eq!(cell(Character::Pichu), (2, 0));
        assert_eq!(cell(Character::Roy), (2, 6));
        assert_eq!(grid_cell(characters().len() as u32), None);
    }
}
