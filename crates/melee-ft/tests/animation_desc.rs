use hsd_archive::{Archive, ArchiveHeader};
use melee_ft::desc::{read_fighter_animations, read_fox_animations, AnimationDescError};

struct Fixture {
    data: Vec<u8>,
    relocs: Vec<u32>,
    table: u32,
    fighter: u32,
}

impl Fixture {
    fn new(symbol_at_zero: bool) -> Self {
        let (table, symbol) = if symbol_at_zero { (8, 0) } else { (0, 72) };
        let fighter = 80;
        let mut fixture = Self {
            data: vec![0; 96],
            relocs: Vec::new(),
            table,
            fighter,
        };
        fixture.data[symbol as usize..symbol as usize + 5].copy_from_slice(b"Wait\0");
        fixture.word(fighter + 12, table);
        fixture.relocs.push(fighter + 12);
        for row in [0, 2] {
            let offset = table + row * 24;
            fixture.word(offset, symbol);
            fixture.word(offset + 4, 4);
            fixture.word(offset + 8, 8);
            fixture.relocs.push(offset);
        }
        fixture
    }

    fn word(&mut self, offset: u32, value: u32) {
        self.data[offset as usize..offset as usize + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn archive(&self) -> Archive {
        let header = ArchiveHeader {
            file_size: (ArchiveHeader::SIZE + self.data.len() + 4 * self.relocs.len()) as u32,
            data_size: self.data.len() as u32,
            nb_reloc: self.relocs.len() as u32,
            nb_public: 0,
            nb_extern: 0,
            version: *b"001B",
        };
        let mut bytes = header.to_bytes().to_vec();
        bytes.extend(&self.data);
        for slot in &self.relocs {
            bytes.extend(slot.to_be_bytes());
        }
        Archive::parse(&bytes).unwrap()
    }
}

#[test]
fn relocated_zero_tables_and_names_preserve_empty_rows_and_aliases() {
    for symbol_at_zero in [false, true] {
        let fixture = Fixture::new(symbol_at_zero);
        let table = read_fighter_animations(&fixture.archive(), fixture.fighter, 3).unwrap();
        assert_eq!(table.table_offset, Some(fixture.table));
        assert_eq!(table.entries.len(), 3);
        assert_eq!(table.entries[0].symbol_name.as_deref(), Some("Wait"));
        assert_eq!(table.entries[0], table.entries[2]);
        assert_eq!(table.entries[1].symbol_name, None);
        assert_eq!(table.entries[1].sub_archive(&[]).unwrap(), None);
        assert_eq!(
            table.entries[0].sub_archive(b"pad!archive!tail").unwrap(),
            Some(&b"archive!"[..])
        );
        assert!(table.entries[0].sub_archive(b"too short").is_err());
    }
}

#[test]
fn invalid_pointers_counts_and_rows_are_errors() {
    let mut fixture = Fixture::new(false);
    fixture.word(fixture.table, 1);
    fixture.relocs.retain(|&slot| slot != fixture.table);
    assert!(matches!(
        read_fighter_animations(&fixture.archive(), fixture.fighter, 3),
        Err(AnimationDescError::UnrelocatedPointer { .. })
    ));

    for (field, value) in [(4, u32::MAX), (8, u32::MAX), (8, 0x8001), (8, 0)] {
        let mut fixture = Fixture::new(false);
        fixture.word(fixture.table + field, value);
        assert!(matches!(
            read_fighter_animations(&fixture.archive(), fixture.fighter, 3),
            Err(AnimationDescError::InvalidRow { .. })
        ));
    }
    let mut fixture = Fixture::new(false);
    assert!(read_fighter_animations(&fixture.archive(), fixture.fighter, u32::MAX).is_err());
    assert!(read_fighter_animations(&fixture.archive(), u32::MAX, 3).is_err());
    assert!(read_fighter_animations(&fixture.archive(), fixture.fighter, 5).is_err());
    assert!(matches!(
        read_fox_animations(&fixture.archive()),
        Err(AnimationDescError::MissingPublic(_))
    ));
    fixture.relocs.retain(|&slot| slot != fixture.fighter + 12);
    assert_eq!(
        read_fighter_animations(&fixture.archive(), fixture.fighter, 3),
        Err(AnimationDescError::NullTable)
    );
    let empty = read_fighter_animations(&fixture.archive(), fixture.fighter, 0).unwrap();
    assert!(empty.entries.is_empty());
    assert_eq!(empty.table_offset, None);
}
