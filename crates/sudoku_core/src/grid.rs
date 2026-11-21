//! Board geometry and small bit-set helpers.

/// A digit `1..=9`; `0` means "empty".
pub type Digit = u8;

/// Cell index `0..81`, row-major.
pub type Idx = usize;

/// A set of digits stored as bits `1..=9` of a `u16`.
#[derive(Clone, Copy, Default, PartialEq, Eq, Hash, Debug)]
pub struct Notes(pub u16);

pub const ALL_DIGITS: u16 = 0b11_1111_1110;

impl Notes {
    pub const EMPTY: Notes = Notes(0);

    #[inline]
    pub fn contains(self, d: Digit) -> bool {
        self.0 & (1 << d) != 0
    }
    #[inline]
    pub fn insert(&mut self, d: Digit) {
        self.0 |= 1 << d;
    }
    #[inline]
    pub fn remove(&mut self, d: Digit) {
        self.0 &= !(1 << d);
    }
    #[inline]
    pub fn toggle(&mut self, d: Digit) {
        self.0 ^= 1 << d;
    }
    #[inline]
    pub fn is_empty(self) -> bool {
        self.0 == 0
    }
    #[inline]
    pub fn len(self) -> u32 {
        self.0.count_ones()
    }
    pub fn iter(self) -> impl Iterator<Item = Digit> {
        (1..=9).filter(move |&d| self.contains(d))
    }
}

#[inline]
pub const fn row_of(i: Idx) -> usize {
    i / 9
}
#[inline]
pub const fn col_of(i: Idx) -> usize {
    i % 9
}
#[inline]
pub const fn box_of(i: Idx) -> usize {
    (i / 27) * 3 + (i % 9) / 3
}
#[inline]
pub const fn idx(row: usize, col: usize) -> Idx {
    row * 9 + col
}

/// The 27 units: 9 rows, then 9 columns, then 9 boxes.
pub static UNITS: [[u8; 9]; 27] = build_units();
/// For each cell, the 20 other cells sharing a row, column or box.
pub static PEERS: [[u8; 20]; 81] = build_peers();

const fn build_units() -> [[u8; 9]; 27] {
    let mut units = [[0u8; 9]; 27];
    let mut u = 0;
    while u < 9 {
        let mut k = 0;
        while k < 9 {
            units[u][k] = (u * 9 + k) as u8; // row u
            units[9 + u][k] = (k * 9 + u) as u8; // column u
            let br = (u / 3) * 3;
            let bc = (u % 3) * 3;
            units[18 + u][k] = ((br + k / 3) * 9 + bc + k % 3) as u8; // box u
            k += 1;
        }
        u += 1;
    }
    units
}

const fn build_peers() -> [[u8; 20]; 81] {
    let mut peers = [[0u8; 20]; 81];
    let mut i = 0;
    while i < 81 {
        let mut n = 0;
        let mut j = 0;
        while j < 81 {
            if j != i && (row_of(i) == row_of(j) || col_of(i) == col_of(j) || box_of(i) == box_of(j)) {
                peers[i][n] = j as u8;
                n += 1;
            }
            j += 1;
        }
        i += 1;
    }
    peers
}

/// Plain 81-cell board of digits (`0` = empty).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Grid(pub [Digit; 81]);

impl Default for Grid {
    fn default() -> Self {
        Grid([0; 81])
    }
}

impl Grid {
    pub fn get(&self, i: Idx) -> Digit {
        self.0[i]
    }
    pub fn set(&mut self, i: Idx, d: Digit) {
        self.0[i] = d;
    }
    pub fn filled(&self) -> usize {
        self.0.iter().filter(|&&d| d != 0).count()
    }
    pub fn is_complete(&self) -> bool {
        self.0.iter().all(|&d| d != 0)
    }

    /// Parse 81 chars of `1-9` with `0`/`.` for empty cells (whitespace ignored).
    pub fn parse(s: &str) -> Option<Grid> {
        let mut g = Grid::default();
        let mut n = 0;
        for ch in s.chars().filter(|c| !c.is_whitespace()) {
            if n >= 81 {
                return None;
            }
            g.0[n] = match ch {
                '1'..='9' => ch as u8 - b'0',
                '0' | '.' => 0,
                _ => return None,
            };
            n += 1;
        }
        (n == 81).then_some(g)
    }

    /// Candidates for an empty cell given the digits currently on the board.
    pub fn candidates(&self, i: Idx) -> Notes {
        let mut used = 0u16;
        for &p in &PEERS[i] {
            used |= 1 << self.0[p as usize];
        }
        Notes(ALL_DIGITS & !used)
    }
}

impl std::fmt::Debug for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for r in 0..9 {
            for c in 0..9 {
                let d = self.0[idx(r, c)];
                write!(f, "{}", if d == 0 { '.' } else { (b'0' + d) as char })?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn geometry() {
        assert_eq!(box_of(idx(4, 4)), 4);
        assert_eq!(box_of(idx(8, 0)), 6);
        for p in PEERS.iter() {
            let mut v = p.to_vec();
            v.sort();
            v.dedup();
            assert_eq!(v.len(), 20);
        }
        assert_eq!(UNITS[18 + 4][0] as usize, idx(3, 3));
    }
}
