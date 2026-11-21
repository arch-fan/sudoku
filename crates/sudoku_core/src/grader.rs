//! Human-style logical solver used to rate how hard a puzzle feels.
//!
//! Techniques are tried from easiest to hardest; the grade is the hardest
//! technique the solve *needed*.

use crate::grid::{ALL_DIGITS, Grid, PEERS, UNITS, box_of, col_of, row_of};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Technique {
    /// Naked & hidden singles.
    Singles = 1,
    /// Pointing / claiming (locked candidates).
    LockedCandidates = 2,
    /// Naked & hidden pairs, naked triples.
    Subsets = 3,
    /// Not solvable with the techniques above (needs chains, fish, guessing…).
    Advanced = 4,
}

struct Board {
    values: [u8; 81],
    cands: [u16; 81],
}

impl Board {
    fn new(g: &Grid) -> Board {
        let mut b = Board { values: [0; 81], cands: [ALL_DIGITS; 81] };
        for i in 0..81 {
            if g.0[i] != 0 {
                b.place(i, g.0[i]);
            }
        }
        b
    }

    fn place(&mut self, i: usize, d: u8) {
        self.values[i] = d;
        self.cands[i] = 0;
        let bit = !(1u16 << d);
        for &p in &PEERS[i] {
            self.cands[p as usize] &= bit;
        }
    }

    fn solved(&self) -> bool {
        self.values.iter().all(|&v| v != 0)
    }

    fn broken(&self) -> bool {
        (0..81).any(|i| self.values[i] == 0 && self.cands[i] == 0)
    }

    fn singles(&mut self) -> bool {
        let mut progress = false;
        // naked singles
        for i in 0..81 {
            if self.values[i] == 0 && self.cands[i].count_ones() == 1 {
                self.place(i, self.cands[i].trailing_zeros() as u8);
                progress = true;
            }
        }
        // hidden singles
        for unit in &UNITS {
            for d in 1..=9u8 {
                let bit = 1u16 << d;
                let mut spot = None;
                let mut n = 0;
                for &c in unit {
                    let c = c as usize;
                    if self.values[c] == d {
                        n = 2; // already placed
                        break;
                    }
                    if self.cands[c] & bit != 0 {
                        n += 1;
                        spot = Some(c);
                    }
                }
                if n == 1 {
                    self.place(spot.unwrap(), d);
                    progress = true;
                }
            }
        }
        progress
    }

    fn locked_candidates(&mut self) -> bool {
        let mut progress = false;
        for d in 1..=9u8 {
            let bit = 1u16 << d;
            // pointing: box → line
            for b in 0..9 {
                let cells: Vec<usize> = UNITS[18 + b]
                    .iter()
                    .map(|&c| c as usize)
                    .filter(|&c| self.cands[c] & bit != 0)
                    .collect();
                if cells.len() < 2 {
                    continue;
                }
                let r = row_of(cells[0]);
                if cells.iter().all(|&c| row_of(c) == r) {
                    for &c in &UNITS[r] {
                        let c = c as usize;
                        if box_of(c) != b && self.cands[c] & bit != 0 {
                            self.cands[c] &= !bit;
                            progress = true;
                        }
                    }
                }
                let col = col_of(cells[0]);
                if cells.iter().all(|&c| col_of(c) == col) {
                    for &c in &UNITS[9 + col] {
                        let c = c as usize;
                        if box_of(c) != b && self.cands[c] & bit != 0 {
                            self.cands[c] &= !bit;
                            progress = true;
                        }
                    }
                }
            }
            // claiming: line → box
            for line in 0..18 {
                let cells: Vec<usize> = UNITS[line]
                    .iter()
                    .map(|&c| c as usize)
                    .filter(|&c| self.cands[c] & bit != 0)
                    .collect();
                if cells.len() < 2 {
                    continue;
                }
                let b = box_of(cells[0]);
                if cells.iter().all(|&c| box_of(c) == b) {
                    for &c in &UNITS[18 + b] {
                        let c = c as usize;
                        if !UNITS[line].contains(&(c as u8)) && self.cands[c] & bit != 0 {
                            self.cands[c] &= !bit;
                            progress = true;
                        }
                    }
                }
            }
        }
        progress
    }

    fn subsets(&mut self) -> bool {
        let mut progress = false;
        for unit in &UNITS {
            let empties: Vec<usize> =
                unit.iter().map(|&c| c as usize).filter(|&c| self.values[c] == 0).collect();
            // naked pairs & triples
            for size in 2..=3usize {
                let n = empties.len();
                if n <= size {
                    continue;
                }
                let mut combo = vec![0usize; size];
                // iterate combinations of `size` cells
                fn next(combo: &mut [usize], n: usize) -> bool {
                    let k = combo.len();
                    let mut i = k;
                    while i > 0 {
                        i -= 1;
                        if combo[i] < n - k + i {
                            combo[i] += 1;
                            for j in i + 1..k {
                                combo[j] = combo[j - 1] + 1;
                            }
                            return true;
                        }
                    }
                    false
                }
                for (i, c) in combo.iter_mut().enumerate() {
                    *c = i;
                }
                loop {
                    let union = combo.iter().fold(0u16, |acc, &k| acc | self.cands[empties[k]]);
                    if union.count_ones() as usize == size {
                        for (k, &c) in empties.iter().enumerate() {
                            if !combo.contains(&k) && self.cands[c] & union != 0 {
                                self.cands[c] &= !union;
                                progress = true;
                            }
                        }
                    }
                    if !next(&mut combo, n) {
                        break;
                    }
                }
            }
            // hidden pairs
            for d1 in 1..=8u8 {
                for d2 in d1 + 1..=9u8 {
                    let (b1, b2) = (1u16 << d1, 1u16 << d2);
                    let s1: Vec<usize> = empties.iter().copied().filter(|&c| self.cands[c] & b1 != 0).collect();
                    let s2: Vec<usize> = empties.iter().copied().filter(|&c| self.cands[c] & b2 != 0).collect();
                    if s1.len() == 2 && s1 == s2 {
                        for &c in &s1 {
                            if self.cands[c] & !(b1 | b2) != 0 {
                                self.cands[c] &= b1 | b2;
                                progress = true;
                            }
                        }
                    }
                }
            }
        }
        progress
    }
}

/// Rates a puzzle by the hardest technique required to solve it logically.
pub fn grade(puzzle: &Grid) -> Technique {
    let mut b = Board::new(puzzle);
    let mut hardest = Technique::Singles;
    loop {
        if b.solved() {
            return hardest;
        }
        if b.broken() {
            return Technique::Advanced;
        }
        if b.singles() {
            continue;
        }
        if b.locked_candidates() {
            hardest = hardest.max(Technique::LockedCandidates);
            continue;
        }
        if b.subsets() {
            hardest = hardest.max(Technique::Subsets);
            continue;
        }
        return Technique::Advanced;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn easy_puzzle_is_singles() {
        let g = Grid::parse(
            "530070000600195000098000060800060003400803001700020006060000280000419005000080079",
        )
        .unwrap();
        assert_eq!(grade(&g), Technique::Singles);
    }
}
