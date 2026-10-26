//! The blocks (map 6, grGreens_80214674 / 8021479C): two grids of three
//! columns by five rows. A block falls in from the top of the blast zone
//! into a column chosen by weight, stacks on what is below it, lands on its
//! grid spot; a hit breaks it (a bomb explodes first), as does a fighter
//! bumping a falling block from below. Its collision joint's lines are
//! stitched to its landed neighbours' (mpLib_800581DC).
use super::{randrange, Parameters};
use gekko_math::{fma::fmadds, HsdRng};

pub const ROWS: usize = 5;
pub const COLUMNS: usize = 6;
pub const SLOTS: usize = 30;
/// grGr_803E7840: slot `n`'s bone in map 6 (its collision joint is `n`).
pub const FIRST_BONE: usize = 5;
/// The block GObjs' maps: a plain block and a bomb.
pub const BLOCK_MAP: u8 = 1;
pub const BOMB_MAP: u8 = 3;
/// grGreens_80215358's capsule: both ends at (0, 5 * scale, 0).
pub const CAPSULE_HEIGHT: f32 = 5.0;
/// efSync_Spawn ids: a bomb's blast (1039) and a block's break (1032).
pub const BLAST_EFFECT: u16 = 1039;
pub const BREAK_EFFECT: u16 = 1032;
/// grGr_8049F9E0's initial layout (grGreens_80214B58): these cells get a
/// block, in this order of draws.
pub const INITIAL_CELLS: [usize; 18] = [
    0x12, 0x0C, 0x0D, 0x06, 0x07, 0x08, 0x00, 0x01, 0x02, 0x17, 0x10, 0x11, 0x09, 0x0A, 0x0B,
    0x03, 0x04, 0x05,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    None = 0,
    Falling = 1,
    /// Falling on top of the block below.
    Stacked = 2,
    Landed = 3,
}

/// `grGreens_BlockVars`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    pub status: Status,
    /// Its slot: bone `FIRST_BONE + slot`, collision joint `slot`.
    pub slot: usize,
    /// x1_1.
    pub bomb: bool,
    /// x1_2: the bomb's blast plays.
    pub exploding: bool,
    /// x1_3: removed at this step's end.
    pub breaking: bool,
    /// x1_4: a fighter bumped it from below (fn_80216DE4).
    pub bumped: bool,
    /// x1_5: stitch its lines to its landed neighbours.
    pub stitch: bool,
    /// x1_6: its cell emptied; restitch around it.
    pub emptied: bool,
    /// x1_7: hit once already.
    pub hit: bool,
    /// x4 / x8.
    pub speed: f32,
    pub y: f32,
    /// x10: its material item.
    pub item: Option<u32>,
}

impl Block {
    const EMPTY: Self = Self {
        status: Status::None,
        slot: 0,
        bomb: false,
        exploding: false,
        breaking: false,
        bumped: false,
        stitch: false,
        emptied: false,
        hit: false,
        speed: 0.0,
        y: 0.0,
        item: None,
    };
    /// x18: Ground_801C32D4(6, its bone).
    pub fn joint(&self) -> i32 {
        self.slot as i32
    }
}

/// What the blocks ask of the stage.
pub trait BlockEngine {
    /// grGreens_80213524(map) for a new block in `slot`, its bone shown,
    /// its material item (state 0, the capsule, hurt on, grounded); returns
    /// the item.
    fn create(&mut self, slot: usize, bomb: bool) -> u32;
    /// The block GObj at (x, y, 0), its bone at that over the map scale,
    /// and (`snap`) its joint moved now (mpLib_80055E9C, 80057424).
    fn place(&mut self, slot: usize, x: f32, y: f32, snap: bool);
    /// grMaterial_801C8E48: the item's script ended.
    fn script_ended(&self, item: u32) -> bool;
    /// grGreens_802159B8's side effects on a hit block: hurt off; a bomb's
    /// GObj hidden, item state 1 and its blast; a block's quake and break.
    fn break_block(&mut self, slot: usize, item: u32, bomb: bool);
    /// The cleanup: Ground_801C4A08 on the block GObj, the item destroyed,
    /// the bone hidden.
    fn remove(&mut self, slot: usize, item: u32);
    /// Stage_GetBlastZoneTopOffset.
    fn blast_top(&self) -> f32;
    /// mpLib_800581DC.
    fn stitch(&mut self, a: i32, b: i32);
    /// The shared random stream (the effects draw from it too).
    fn rng(&mut self) -> &mut HsdRng;
}

/// The grid (map 6, `grGreens_GroundVars`).
#[derive(Clone, Debug, PartialEq)]
pub struct Blocks {
    /// x4: each cell's resting spot.
    pub spots: [[(f32, f32); COLUMNS]; ROWS],
    /// x8: the cells, `[row][column]`.
    pub cells: [[Block; COLUMNS]; ROWS],
    /// xC: ticks to the next falling block.
    pub timer: i32,
    /// x0 b0: set until the stage starts (fn_80214658).
    pub waiting: bool,
}

/// grGreens_8021483C (0x8021483C): the spots from the slots' bones'
/// bind-pose world positions, one grid per side of x = 0. `positions[n]`
/// is slot `n`'s.
pub fn spots(positions: &[(f32, f32); SLOTS]) -> [[(f32, f32); COLUMNS]; ROWS] {
    let mut left = [f32::MIN, f32::MAX, f32::MIN, f32::MAX];
    let mut right = left;
    for &(x, y) in positions {
        let side = if x < 0.0 { &mut left } else { &mut right };
        if side[0] < x {
            side[0] = x;
        }
        if side[1] > x {
            side[1] = x;
        }
        if side[2] < y {
            side[2] = y;
        }
        if side[3] > y {
            side[3] = y;
        }
    }
    let mut spots = [[(0.0, 0.0); COLUMNS]; ROWS];
    for (first, side) in [(0, left), (3, right)] {
        let [max, min, top, bottom] = side;
        // retail 0x80214988 / 0x8021499C: (max - min) * 0.5, (top - bottom) * 0.25.
        let x_step = (max - min) * 0.5;
        let y_step = (top - bottom) * 0.25;
        for (row, cells) in spots.iter_mut().enumerate() {
            for column in 0..3 {
                // retail 0x802149A4.. / 0x802149FC: fmadds.
                cells[first + column] = (
                    fmadds(x_step, column as f32, min),
                    fmadds(y_step, row as f32, bottom),
                );
            }
        }
    }
    spots
}

/// grGreens_80214B58's draw: a bomb when HSD_Randi(x20) is zero.
fn draw_bomb(p: &Parameters, rng: &mut HsdRng) -> bool {
    super::whispy::randi_or_zero(p.bomb_odds, rng) == 0
}

impl Blocks {
    /// grGreens_80214674 (0x80214674): the spots, the initial layout's
    /// draws (grGreens_80214B58), its blocks landed (grGreens_80214FA8),
    /// then the first wait.
    pub fn new<E: BlockEngine>(
        positions: &[(f32, f32); SLOTS],
        p: &Parameters,
        engine: &mut E,
    ) -> Self {
        let mut blocks = Self {
            spots: spots(positions),
            cells: [[Block::EMPTY; COLUMNS]; ROWS],
            timer: 0,
            waiting: true,
        };
        let mut layout = [0u8; SLOTS];
        for cell in INITIAL_CELLS {
            layout[cell] = if draw_bomb(p, engine.rng()) { 2 } else { 1 };
        }
        for row in 0..ROWS {
            for column in 0..COLUMNS {
                match layout[row * COLUMNS + column] {
                    1 => blocks.add(column, row, false, Status::Landed, engine),
                    2 => blocks.add(column, row, true, Status::Landed, engine),
                    _ => {}
                }
            }
        }
        blocks.timer = randrange(p.block_wait[0], p.block_wait[1], engine.rng());
        blocks
    }

    /// grGreens_80215358 (0x80215358): a block in the first free slot.
    fn add<E: BlockEngine>(
        &mut self,
        column: usize,
        row: usize,
        bomb: bool,
        status: Status,
        engine: &mut E,
    ) {
        assert_eq!(self.cells[row][column].status, Status::None, "grgreens.c:1305");
        let mut used = [false; SLOTS];
        for block in self.cells.iter().flatten() {
            if block.status != Status::None {
                used[block.slot] = true;
            }
        }
        let slot = used.iter().position(|&u| !u).expect("grgreens.c:1316");
        let item = engine.create(slot, bomb);
        let y = if status == Status::Falling {
            engine.blast_top()
        } else {
            self.spots[row][column].1
        };
        self.cells[row][column] = Block {
            status,
            slot,
            bomb,
            exploding: false,
            breaking: false,
            bumped: false,
            stitch: status == Status::Landed,
            emptied: false,
            hit: false,
            speed: 0.0,
            y,
            item: Some(item),
        };
        engine.place(slot, self.spots[row][column].0, y, true);
    }

    /// fn_80216DE4 (0x80216DE4): a fighter's head met `joint`.
    pub fn bump(&mut self, joint: i32) {
        if let Some(block) = self
            .cells
            .iter_mut()
            .flatten()
            .find(|b| b.status != Status::None && b.joint() == joint)
        {
            block.bumped = true;
        }
    }

    /// The cell holding `item`'s block (find_block).
    pub fn cell_of_item(&self, item: u32) -> Option<(usize, usize)> {
        for row in 0..ROWS {
            for column in 0..COLUMNS {
                let block = &self.cells[row][column];
                if block.status != Status::None && block.item == Some(item) {
                    return Some((row, column));
                }
            }
        }
        None
    }

    /// grGreens_802159B8 (0x802159B8): a block is hit (fn_80215B84, or a
    /// bump while falling).
    pub fn strike<E: BlockEngine>(&mut self, row: usize, column: usize, engine: &mut E) {
        let block = &mut self.cells[row][column];
        let Some(item) = block.item else {
            return;
        };
        if block.hit {
            return;
        }
        block.hit = true;
        if block.bomb {
            block.exploding = true;
        } else {
            block.breaking = true;
        }
        engine.break_block(block.slot, item, block.bomb);
    }

    /// grGreens_802166C4 (0x802166C4): the next falling block, each
    /// block's fall, then the removals.
    pub fn step<E: BlockEngine>(&mut self, p: &Parameters, engine: &mut E) {
        let timer = self.timer;
        self.timer -= 1;
        if timer < 0 {
            if let Some(column) = self.pick_column(p, engine.rng()) {
                let mut row = ROWS - 1;
                while row > 0 && self.cells[row - 1][column].status == Status::None {
                    row -= 1;
                }
                let bomb = draw_bomb(p, engine.rng());
                self.add(column, row, bomb, Status::Falling, engine);
            }
            self.timer = randrange(p.block_wait[0], p.block_wait[1], engine.rng());
        }
        for row in 0..ROWS {
            for column in 0..COLUMNS {
                self.fall(column, row, p, engine);
            }
        }
        self.remove_broken(engine);
    }

    /// grGreens_802166C4's choice: weights by each column's height; a side
    /// with a full column is left out.
    fn pick_column(&self, p: &Parameters, rng: &mut HsdRng) -> Option<usize> {
        let mut weights = [0u8; COLUMNS];
        for (column, weight) in weights.iter_mut().enumerate() {
            *weight = p.empty_column_weight as u8;
            for row in (0..ROWS).rev() {
                if self.cells[row][column].status != Status::None {
                    *weight = p.column_weights[row] as u8;
                    break;
                }
            }
        }
        let left_full = weights[..3].contains(&0);
        let right_full = weights[3..].contains(&0);
        let (first, count) = match (left_full, right_full) {
            (false, false) => (0, 6),
            (false, true) => (0, 3),
            (true, false) => (3, 3),
            (true, true) => return None,
        };
        let total: i32 = weights[first..first + count].iter().map(|&w| i32::from(w)).sum();
        let mut pick = super::whispy::randi_or_zero(total, rng);
        let mut column = first;
        for &weight in &weights[first..first + count] {
            pick -= i32::from(weight);
            if pick < 0 {
                break;
            }
            column += 1;
        }
        assert!(column < first + count, "grgreens.c:1693: ix past its side");
        Some(column)
    }

    fn place_cell<E: BlockEngine>(&self, row: usize, column: usize, engine: &mut E) {
        let block = &self.cells[row][column];
        engine.place(block.slot, self.spots[row][column].0, block.y, false);
    }

    /// grGreens_80215ED8 (0x80215ED8): one block's fall.
    fn fall<E: BlockEngine>(&mut self, column: usize, row: usize, p: &Parameters, engine: &mut E) {
        let block = self.cells[row][column];
        if block.status == Status::None || block.breaking {
            return;
        }
        if block.exploding {
            if engine.script_ended(block.item.expect("a bomb's item")) {
                self.cells[row][column].breaking = true;
            }
            return;
        }
        if block.status != Status::Falling {
            return;
        }
        if block.bumped {
            let block = &mut self.cells[row][column];
            block.speed = 0.0;
            block.bumped = false;
            self.strike(row, column, engine);
        } else {
            let block = &mut self.cells[row][column];
            block.speed += p.fall_acceleration;
            if block.speed > p.fall_speed_max {
                block.speed = p.fall_speed_max;
            }
            block.y -= block.speed;
            if row > 0 && matches!(self.cells[row - 1][column].status, Status::Falling | Status::Stacked) {
                let spacing = self.spots[row][column].1 - self.spots[row - 1][column].1;
                let below = self.cells[row - 1][column].y;
                let block = &mut self.cells[row][column];
                if block.y - below < spacing {
                    block.status = Status::Stacked;
                    block.y = spacing + below;
                }
            }
        }
        let spot = self.spots[row][column].1;
        if self.cells[row][column].y < spot {
            let block = &mut self.cells[row][column];
            block.y = spot;
            block.status = Status::Landed;
            block.stitch = true;
            for above in row + 1..ROWS {
                if self.cells[above][column].status != Status::Stacked {
                    break;
                }
                let block = &mut self.cells[above][column];
                block.y = self.spots[above][column].1;
                block.status = Status::Landed;
                block.stitch = true;
                self.place_cell(above, column, engine);
            }
        } else {
            for above in row + 1..ROWS {
                if self.cells[above][column].status != Status::Stacked {
                    break;
                }
                let spacing = self.spots[above][column].1 - self.spots[above - 1][column].1;
                self.cells[above][column].y = spacing + self.cells[above - 1][column].y;
                self.place_cell(above, column, engine);
            }
        }
        self.place_cell(row, column, engine);
    }

    /// grGreens_802166C4's cleanup: each breaking block goes, its column
    /// closes up (grGreens_80215D54), and the scan restarts.
    fn remove_broken<E: BlockEngine>(&mut self, engine: &mut E) {
        'scan: loop {
            for row in 0..ROWS {
                for column in 0..COLUMNS {
                    if self.cells[row][column].breaking {
                        let block = &mut self.cells[row][column];
                        block.breaking = false;
                        block.emptied = true;
                        block.status = Status::None;
                        let (slot, item) = (block.slot, block.item.expect("a block's item"));
                        engine.remove(slot, item);
                        self.close_up(column);
                        continue 'scan;
                    }
                }
            }
            break;
        }
    }

    /// grGreens_80215D54 (0x80215D54): a falling block over an empty cell
    /// trades places up the column until the hole reaches the top.
    fn close_up(&mut self, column: usize) {
        let mut row = 1;
        while row < ROWS {
            if matches!(self.cells[row][column].status, Status::Falling | Status::Stacked)
                && self.cells[row - 1][column].status == Status::None
            {
                for r in row..ROWS {
                    let lower = self.cells[r - 1][column];
                    self.cells[r - 1][column] = self.cells[r][column];
                    self.cells[r][column] = lower;
                }
                if self.cells[row - 1][column].status == Status::Stacked {
                    self.cells[row - 1][column].status = Status::Falling;
                }
                row = 1;
                continue;
            }
            row += 1;
        }
    }

    /// grGreens_80216C20 (0x80216C20), after Ground_801C2FE0: new landings
    /// stitch to their landed neighbours; emptied cells restitch around.
    pub fn stitch<E: BlockEngine>(&mut self, engine: &mut E) {
        for row in 0..ROWS {
            for column in 0..COLUMNS {
                if self.cells[row][column].stitch {
                    let joint = self.cells[row][column].joint();
                    let landed = |r: usize, c: usize, cells: &[[Block; COLUMNS]; ROWS]| {
                        cells[r][c].status == Status::Landed
                    };
                    if row > 0 && landed(row - 1, column, &self.cells) {
                        engine.stitch(joint, self.cells[row - 1][column].joint());
                    }
                    if column > 0 && landed(row, column - 1, &self.cells) {
                        engine.stitch(joint, self.cells[row][column - 1].joint());
                    }
                    if row < ROWS - 1 && landed(row + 1, column, &self.cells) {
                        engine.stitch(joint, self.cells[row + 1][column].joint());
                    }
                    if column < COLUMNS - 1 && landed(row, column + 1, &self.cells) {
                        engine.stitch(joint, self.cells[row][column + 1].joint());
                    }
                    self.cells[row][column].stitch = false;
                } else if self.cells[row][column].emptied {
                    self.restitch(column, row, engine);
                    self.cells[row][column].emptied = false;
                }
            }
        }
    }

    /// The cell `[row][column]` read as retail's flat array does (a column
    /// of -1 is the previous row's last cell). Before the first cell lies
    /// the HSD_MemAlloc cell header, whose first word (a heap list pointer
    /// or null) never reads as a landed block's status.
    fn flat(&self, row: usize, column: isize) -> &Block {
        const BEFORE_THE_ARRAY: Block = Block::EMPTY;
        let index = row as isize * COLUMNS as isize + column;
        if index < 0 {
            return &BEFORE_THE_ARRAY;
        }
        let index = index as usize;
        &self.cells[index / COLUMNS][index % COLUMNS]
    }

    /// grGreens_802150C4 (0x802150C4): around an emptied cell, each landed
    /// diagonal neighbour is stitched to a landed orthogonal one.
    fn restitch<E: BlockEngine>(&self, column: usize, row: usize, engine: &mut E) {
        let landed = |b: &Block| b.status == Status::Landed;
        let c = column as isize;
        if row > 0 && column > 0 && landed(self.flat(row - 1, c - 1)) {
            if landed(self.flat(row - 1, c)) {
                engine.stitch(self.flat(row - 1, c - 1).joint(), self.flat(row - 1, c).joint());
            } else if landed(self.flat(row, c - 1)) {
                engine.stitch(self.flat(row - 1, c - 1).joint(), self.flat(row, c - 1).joint());
            }
        }
        if row < ROWS - 1 && column > 0 && landed(self.flat(row + 1, c - 1)) {
            if landed(self.flat(row + 1, c)) {
                engine.stitch(self.flat(row + 1, c - 1).joint(), self.flat(row + 1, c).joint());
            } else if landed(self.flat(row, c - 1)) {
                engine.stitch(self.flat(row + 1, c - 1).joint(), self.flat(row, c - 1).joint());
            }
        }
        if row > 0 && column < COLUMNS - 1 && landed(self.flat(row - 1, c + 1)) {
            if landed(self.flat(row - 1, c)) {
                engine.stitch(self.flat(row - 1, c + 1).joint(), self.flat(row - 1, c).joint());
            } else if landed(self.flat(row, c + 1)) {
                engine.stitch(self.flat(row - 1, c + 1).joint(), self.flat(row, c + 1).joint());
            }
        }
        if row < ROWS - 1 && column < COLUMNS - 1 && landed(self.flat(row + 1, c + 1)) {
            if landed(self.flat(row + 1, c)) {
                engine.stitch(self.flat(row + 1, c + 1).joint(), self.flat(row + 1, c).joint());
            } else if landed(self.flat(row, c - 1)) {
                // grgreens.c:1286: retail tests the left cell, then
                // stitches the right one.
                engine.stitch(self.flat(row + 1, c + 1).joint(), self.flat(row, c + 1).joint());
            }
        }
    }
}
