//! Which retail motion-state rows the port implements, for the function-level
//! parity inventory (`harness/parity_inventory.py --motion-rows`, docs/PARITY.md).
//!
//!     cargo run -q --release -p melee-lib --example motion_rows > target/parity/motion_rows.tsv
//!
//! One line per row: `table` (`common` or the character's kind), the retail
//! action id (the character tables start at 341) and whether the row is
//! implemented (`MotionRow::implemented`; an unported row fails closed).
use melee_ft::fighter::state::{MotionRow, COMMON};
use melee_ft::fighter::CharacterTable;

fn print_rows(table: &str, rows: &[MotionRow]) {
    for row in rows {
        println!("{table}\t{}\t{}", row.action.0, row.implemented);
    }
}

fn main() {
    let tables: [&CharacterTable; 27] = [
        &ft_captain::init::TABLE,
        &ft_donkey::init::TABLE,
        &ft_drmario::init::TABLE,
        &ft_emblem::init::TABLE,
        &ft_falco::init::TABLE,
        &ft_fox::init::TABLE,
        &ft_gamewatch::init::TABLE,
        &ft_ganon::init::TABLE,
        &ft_iceclimbers::init::POPO_TABLE,
        &ft_iceclimbers::init::NANA_TABLE,
        &ft_kirby::init::TABLE,
        &ft_koopa::init::TABLE,
        &ft_link::init::TABLE,
        &ft_luigi::init::TABLE,
        &ft_mario::init::TABLE,
        &ft_mars::init::TABLE,
        &ft_mewtwo::init::TABLE,
        &ft_ness::init::TABLE,
        &ft_peach::init::TABLE,
        &ft_pichu::init::TABLE,
        &ft_pikachu::init::TABLE,
        &ft_purin::init::TABLE,
        &ft_samus::init::TABLE,
        &ft_seak::init::TABLE,
        &ft_yoshi::init::TABLE,
        &ft_younglink::init::TABLE,
        &ft_zelda::init::TABLE,
    ];
    println!("table\taction\timplemented");
    print_rows("common", &COMMON);
    for table in tables {
        let kind = (table.descriptor)().kind;
        print_rows(&format!("{kind:?}"), table.special_rows);
    }
}
