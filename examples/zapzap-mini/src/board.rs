use std::collections::VecDeque;

/// Direction flags for tile connections.
/// 4-bit bitmask: right=1, up=2, left=4, down=8.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Direction(pub u8);

impl Direction {
    pub const RIGHT: Direction = Direction(1 << 0); // 0001
    pub const UP: Direction = Direction(1 << 1);    // 0010
    pub const LEFT: Direction = Direction(1 << 2);  // 0100
    pub const DOWN: Direction = Direction(1 << 3);  // 1000
}

/// Marking state for each tile during connection checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Marking {
    Left = 0,
    Right = 1,
    Ok = 2,
    None = 3,
    Animating = 4,
}

/// A single tile with a 4-bit connection bitmask (RIGHT|UP|LEFT|DOWN).
#[derive(Debug, Clone, Copy)]
pub struct Tile {
    pub connections: u8,
}

impl Tile {
    pub fn new(connections: u8) -> Self {
        Self { connections: connections & 0x0F }
    }

    pub fn has_connection(&self, dir: Direction) -> bool {
        (self.connections & dir.0) != 0
    }

    /// Rotate connections 90 degrees counter-clockwise (bit-shift left with wrap).
    pub fn rotate(&mut self) {
        let c = self.connections;
        self.connections = ((c << 1) | (c >> 3)) & 0x0F;
    }
}

/// Column-major grid of optional tiles.
#[derive(Debug, Clone)]
pub struct Grid {
    pub width: usize,
    pub height: usize,
    cells: Vec<Option<Tile>>,
}

impl Grid {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cells: vec![None; width * height],
        }
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        x * self.height + y
    }

    pub fn get(&self, x: usize, y: usize) -> Option<&Tile> {
        if x < self.width && y < self.height {
            self.cells[self.idx(x, y)].as_ref()
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut Tile> {
        if x < self.width && y < self.height {
            let i = self.idx(x, y);
            self.cells[i].as_mut()
        } else {
            None
        }
    }

    pub fn set(&mut self, x: usize, y: usize, tile: Option<Tile>) {
        if x < self.width && y < self.height {
            let i = self.idx(x, y);
            self.cells[i] = tile;
        }
    }
}

/// Atlas column lookup: connection bitmask (0-15) → texture atlas column.
pub const GRID_CODEP: [u8; 16] = [0, 12, 15, 5, 14, 1, 4, 7, 13, 6, 2, 8, 3, 9, 10, 11];

/// Atlas row for normal game tiles.
pub const ATLAS_ROW_NORMAL: f32 = 1.0;
/// Atlas row for pin sprites.
pub const ATLAS_ROW_PINS: f32 = 3.0;
/// Atlas column for left pin.
pub const ATLAS_COL_LEFT_PIN: f32 = 12.0;
/// Atlas column for right pin.
pub const ATLAS_COL_RIGHT_PIN: f32 = 14.0;

/// Simple xorshift64 PRNG (deterministic, same as native).
#[derive(Debug, Clone)]
pub struct Rng {
    pub state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut s = self.state;
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        self.state = s;
        s
    }

    pub fn next_int(&mut self, upper_bound: u32) -> u32 {
        (self.next_u64() % upper_bound as u64) as u32
    }
}

/// The core game board: grid + markings + RNG.
#[derive(Debug, Clone)]
pub struct GameBoard {
    pub width: usize,
    pub height: usize,
    pub grid: Grid,
    pub markings: Vec<Marking>,
    pub rng: Rng,
    pub left_pins_connect: usize,
    pub right_pins_connect: usize,
    missing_links: usize,
    new_elements: usize,
    missing_link_elements: usize,
}

impl GameBoard {
    pub fn new(width: usize, height: usize, missing_links: usize, seed: u64) -> Self {
        let mut board = GameBoard {
            width,
            height,
            grid: Grid::new(width, height),
            markings: vec![Marking::None; width * height],
            rng: Rng::new(seed),
            left_pins_connect: 0,
            right_pins_connect: 0,
            missing_links,
            new_elements: 0,
            missing_link_elements: 0,
        };
        board.reset_table();
        board
    }

    fn marking_idx(&self, x: usize, y: usize) -> usize {
        x * self.height + y
    }

    pub fn get_marking(&self, x: usize, y: usize) -> Marking {
        self.markings[self.marking_idx(x, y)]
    }

    pub fn set_marking(&mut self, x: usize, y: usize, m: Marking) {
        let i = self.marking_idx(x, y);
        self.markings[i] = m;
    }

    /// Generate a random tile connection value (1..=15).
    /// Controls the ratio of dead-end tiles (single connections).
    pub fn get_new_element(&mut self) -> u8 {
        let mut k = (self.rng.next_int(15) + 1) as u8;
        self.new_elements += 1;

        if self.new_elements > 0
            && (100 * self.missing_link_elements / self.new_elements) > self.missing_links
        {
            while matches!(k, 1 | 2 | 4 | 8) {
                k = (self.rng.next_int(15) + 1) as u8;
            }
        }

        if matches!(k, 1 | 2 | 4 | 8) {
            self.missing_link_elements += 1;
        }

        k
    }

    /// Reset the entire board with fresh random tiles.
    pub fn reset_table(&mut self) {
        self.new_elements = 0;
        self.missing_link_elements = 0;

        for j in 0..self.height {
            for i in 0..self.width {
                let conn = self.get_new_element();
                self.grid.set(i, j, Some(Tile::new(conn)));
                self.set_marking(i, j, Marking::None);
            }
        }
    }

    /// Iterative BFS flood-fill that marks connected tiles.
    fn expand_connections_bfs(
        &mut self,
        cx: usize,
        cy: usize,
        incoming_dir: Direction,
        marker: Marking,
    ) {
        if marker == Marking::None || marker == Marking::Animating {
            return;
        }

        let mut queue: VecDeque<(usize, usize, Direction)> =
            VecDeque::with_capacity(self.width * self.height);
        queue.push_back((cx, cy, incoming_dir));

        while let Some((x, y, ctype)) = queue.pop_front() {
            if x >= self.width || y >= self.height {
                continue;
            }
            let current = self.get_marking(x, y);
            // Skip if already at the target marking or already Ok
            if current == marker || current == Marking::Ok {
                continue;
            }

            let tile = match self.grid.get(x, y) {
                Some(t) if t.has_connection(ctype) => *t,
                _ => continue,
            };

            // Upgrade to Ok if cell is already marked from the other side
            let effective = match (marker, current) {
                (Marking::Left, Marking::Right) | (Marking::Right, Marking::Left) => Marking::Ok,
                (Marking::Ok, _) => Marking::Ok,
                _ => marker,
            };
            self.set_marking(x, y, effective);

            if tile.has_connection(Direction::LEFT) && x > 0 {
                queue.push_back((x - 1, y, Direction::RIGHT));
            }
            if tile.has_connection(Direction::UP) && y > 0 {
                queue.push_back((x, y - 1, Direction::DOWN));
            }
            if tile.has_connection(Direction::RIGHT) {
                queue.push_back((x + 1, y, Direction::LEFT));
            }
            if tile.has_connection(Direction::DOWN) {
                queue.push_back((x, y + 1, Direction::UP));
            }
        }
    }

    /// Check connections from both sides. Returns 1 if any left-to-right path exists.
    pub fn check_connections(&mut self) -> i32 {
        let mut result = 0;
        self.left_pins_connect = 0;
        self.right_pins_connect = 0;

        // Reset markings (preserve Animating)
        for j in 0..self.height {
            for i in 0..self.width {
                if self.get_marking(i, j) != Marking::Animating {
                    self.set_marking(i, j, Marking::None);
                }
            }
        }

        // Pass 1: Flood-fill from right edge
        for j in 0..self.height {
            if let Some(tile) = self.grid.get(self.width - 1, j) {
                if tile.has_connection(Direction::RIGHT) {
                    self.expand_connections_bfs(
                        self.width - 1, j, Direction::RIGHT, Marking::Right,
                    );
                }
            }
        }

        // Pass 2: Flood-fill from left edge
        for j in 0..self.height {
            if let Some(tile) = self.grid.get(0, j) {
                if tile.has_connection(Direction::LEFT) {
                    let marker = if self.get_marking(0, j) == Marking::Right
                        || self.get_marking(0, j) == Marking::Ok
                    {
                        Marking::Ok
                    } else {
                        Marking::Left
                    };
                    self.expand_connections_bfs(0, j, Direction::LEFT, marker);
                }
            }
        }

        // Pass 3: Count connecting pins
        for j in 0..self.height {
            if let Some(tile) = self.grid.get(0, j) {
                if tile.has_connection(Direction::LEFT) && self.get_marking(0, j) == Marking::Ok {
                    result = 1;
                    self.left_pins_connect += 1;
                }
            }
            if let Some(tile) = self.grid.get(self.width - 1, j) {
                if tile.has_connection(Direction::RIGHT)
                    && self.get_marking(self.width - 1, j) == Marking::Ok
                {
                    self.right_pins_connect += 1;
                }
            }
        }

        result
    }

    /// Remove Ok-marked tiles and shift remaining tiles down (gravity).
    /// New random tiles fill from the top. Returns per-column fall info for animations.
    pub fn remove_and_shift(&mut self) -> Vec<(usize, usize, usize)> {
        let mut falls = Vec::new(); // (x, new_y, distance_fallen)

        for x in 0..self.width {
            // Collect surviving tiles bottom-up
            let mut survivors: Vec<Tile> = Vec::with_capacity(self.height);
            for y in (0..self.height).rev() {
                if self.get_marking(x, y) != Marking::Ok {
                    if let Some(tile) = self.grid.get(x, y) {
                        survivors.push(*tile);
                    }
                }
            }

            let num_new = self.height - survivors.len();

            // Place survivors at the bottom
            for (i, tile) in survivors.into_iter().enumerate() {
                let y = self.height - 1 - i;
                self.grid.set(x, y, Some(tile));
                // Original y was (y - num_new) positions higher, but we track the shift distance
                if num_new > 0 {
                    falls.push((x, y, num_new));
                }
            }

            // Fill the top with new random tiles
            for y in 0..num_new {
                let conn = self.get_new_element();
                self.grid.set(x, y, Some(Tile::new(conn)));
                falls.push((x, y, num_new));
            }
        }

        falls
    }

    /// Count tiles currently marked as Ok (for scoring).
    pub fn count_ok_tiles(&self) -> usize {
        let mut count = 0;
        for x in 0..self.width {
            for y in 0..self.height {
                if self.get_marking(x, y) == Marking::Ok {
                    count += 1;
                }
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rng_deterministic() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_int(100), b.next_int(100));
        }
    }

    #[test]
    fn new_element_range() {
        let mut board = GameBoard::new(8, 8, 3, 42);
        for _ in 0..500 {
            let e = board.get_new_element();
            assert!((1..=15).contains(&e), "got {}", e);
        }
    }

    #[test]
    fn tile_rotation() {
        // RIGHT(1) rotates to UP(2)
        let mut tile = Tile::new(0b0001);
        tile.rotate();
        assert_eq!(tile.connections, 0b0010);
        // Full cycle
        tile.rotate();
        tile.rotate();
        tile.rotate();
        assert_eq!(tile.connections, 0b0001);
    }

    #[test]
    fn check_connections_simple() {
        let mut board = GameBoard::new(3, 1, 0, 1);
        board.grid.set(0, 0, Some(Tile::new(0b0101))); // LEFT + RIGHT
        board.grid.set(1, 0, Some(Tile::new(0b0101)));
        board.grid.set(2, 0, Some(Tile::new(0b0101)));
        assert_eq!(board.check_connections(), 1);
        assert_eq!(board.get_marking(0, 0), Marking::Ok);
        assert_eq!(board.get_marking(1, 0), Marking::Ok);
        assert_eq!(board.get_marking(2, 0), Marking::Ok);
    }

    #[test]
    fn check_connections_no_path() {
        let mut board = GameBoard::new(3, 1, 0, 1);
        board.grid.set(0, 0, Some(Tile::new(0b0100))); // LEFT only
        board.grid.set(1, 0, Some(Tile::new(0b1000))); // DOWN only
        board.grid.set(2, 0, Some(Tile::new(0b0001))); // RIGHT only
        assert_eq!(board.check_connections(), 0);
    }

    #[test]
    fn check_connections_l_shaped() {
        let mut board = GameBoard::new(3, 2, 0, 1);
        board.grid.set(0, 0, Some(Tile::new(0b0101))); // LEFT + RIGHT
        board.grid.set(1, 0, Some(Tile::new(0b1100))); // LEFT + DOWN
        board.grid.set(2, 0, Some(Tile::new(0b1000))); // DOWN only
        board.grid.set(0, 1, Some(Tile::new(0b0010))); // UP only
        board.grid.set(1, 1, Some(Tile::new(0b0011))); // RIGHT + UP
        board.grid.set(2, 1, Some(Tile::new(0b0101))); // LEFT + RIGHT
        assert_eq!(board.check_connections(), 1);
        assert_eq!(board.get_marking(0, 0), Marking::Ok);
    }

    #[test]
    fn remove_and_shift_basic() {
        let mut board = GameBoard::new(3, 3, 0, 99);
        // Row 1: full horizontal connection
        board.grid.set(0, 1, Some(Tile::new(0b0101)));
        board.grid.set(1, 1, Some(Tile::new(0b0101)));
        board.grid.set(2, 1, Some(Tile::new(0b0101)));
        // Other rows
        for x in 0..3 {
            board.grid.set(x, 0, Some(Tile::new(0b1111)));
            board.grid.set(x, 2, Some(Tile::new(0b0010)));
        }
        let _ = board.check_connections();
        let old_top = board.grid.get(0, 0).unwrap().connections;
        board.remove_and_shift();
        // Old top row tile should shift down to row 1
        let shifted = board.grid.get(0, 1).unwrap();
        assert_eq!(shifted.connections, old_top);
    }
}
