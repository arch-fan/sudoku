//! Bitmask backtracking solver with "minimum remaining values" ordering.

use crate::grid::{ALL_DIGITS, Grid, box_of, col_of, row_of};
use crate::rng::Rng;

struct State {
    cells: [u8; 81],
    rows: [u16; 9],
    cols: [u16; 9],
    boxes: [u16; 9],
}

impl State {
    fn new(grid: &Grid) -> Option<State> {
        let mut s = State { cells: grid.0, rows: [0; 9], cols: [0; 9], boxes: [0; 9] };
        for i in 0..81 {
            let d = s.cells[i];
            if d != 0 {
                let bit = 1u16 << d;
                let (r, c, b) = (row_of(i), col_of(i), box_of(i));
                if (s.rows[r] | s.cols[c] | s.boxes[b]) & bit != 0 {
                    return None; // contradictory givens
                }
                s.rows[r] |= bit;
                s.cols[c] |= bit;
                s.boxes[b] |= bit;
            }
        }
        Some(s)
    }

    #[inline]
    fn cands(&self, i: usize) -> u16 {
        ALL_DIGITS & !(self.rows[row_of(i)] | self.cols[col_of(i)] | self.boxes[box_of(i)])
    }

    #[inline]
    fn toggle(&mut self, i: usize, d: u8) {
        let bit = 1u16 << d;
        self.rows[row_of(i)] ^= bit;
        self.cols[col_of(i)] ^= bit;
        self.boxes[box_of(i)] ^= bit;
    }

    /// Picks the empty cell with the fewest candidates. `None` = board full.
    fn pick(&self) -> Option<(usize, u16)> {
        let mut best: Option<(usize, u16)> = None;
        let mut best_n = 10;
        for i in 0..81 {
            if self.cells[i] == 0 {
                let c = self.cands(i);
                let n = c.count_ones();
                if n < best_n {
                    best_n = n;
                    best = Some((i, c));
                    if n <= 1 {
                        break;
                    }
                }
            }
        }
        best
    }

    fn count(&mut self, limit: usize, found: &mut usize, first: &mut Option<[u8; 81]>) {
        let Some((i, mut c)) = self.pick() else {
            *found += 1;
            if first.is_none() {
                *first = Some(self.cells);
            }
            return;
        };
        while c != 0 {
            let d = c.trailing_zeros() as u8;
            c &= c - 1;
            self.cells[i] = d;
            self.toggle(i, d);
            self.count(limit, found, first);
            self.toggle(i, d);
            self.cells[i] = 0;
            if *found >= limit {
                return;
            }
        }
    }

    fn fill_random(&mut self, rng: &mut Rng) -> bool {
        let Some((i, c)) = self.pick() else { return true };
        let mut digits: Vec<u8> = (1..=9).filter(|&d| c & (1 << d) != 0).collect();
        rng.shuffle(&mut digits);
        for d in digits {
            self.cells[i] = d;
            self.toggle(i, d);
            if self.fill_random(rng) {
                return true;
            }
            self.toggle(i, d);
            self.cells[i] = 0;
        }
        false
    }
}

/// Counts solutions, stopping once `limit` is reached.
pub fn count_solutions(grid: &Grid, limit: usize) -> usize {
    let Some(mut s) = State::new(grid) else { return 0 };
    let mut found = 0;
    let mut first = None;
    s.count(limit, &mut found, &mut first);
    found
}

/// Returns `true` when the puzzle has exactly one solution.
pub fn has_unique_solution(grid: &Grid) -> bool {
    count_solutions(grid, 2) == 1
}

/// Solves the puzzle (first solution found).
pub fn solve(grid: &Grid) -> Option<Grid> {
    let mut s = State::new(grid)?;
    let mut found = 0;
    let mut first = None;
    s.count(1, &mut found, &mut first);
    first.map(Grid)
}

/// Produces a random, fully solved grid.
pub fn random_solution(rng: &mut Rng) -> Grid {
    let mut s = State::new(&Grid::default()).expect("empty grid is valid");
    let ok = s.fill_random(rng);
    debug_assert!(ok);
    Grid(s.cells)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn solves_known_puzzle() {
        let g = Grid::parse(
            "530070000600195000098000060800060003400803001700020006060000280000419005000080079",
        )
        .unwrap();
        assert!(has_unique_solution(&g));
        let s = solve(&g).unwrap();
        assert!(s.is_complete());
        assert_eq!(s.0[2], 4);
    }

    #[test]
    fn random_solution_is_valid() {
        let mut rng = Rng::new(7);
        let g = random_solution(&mut rng);
        assert!(g.is_complete());
        assert_eq!(count_solutions(&g, 2), 1);
    }
}
