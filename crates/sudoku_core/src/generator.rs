//! Puzzle generation: random full grid → dig holes while the solution stays
//! unique → grade → retry until the grade fits the requested difficulty.

use crate::difficulty::Difficulty;
use crate::grader::{Technique, grade};
use crate::grid::Grid;
use crate::rng::Rng;
use crate::solver::{has_unique_solution, random_solution};

#[derive(Clone, Debug)]
pub struct Puzzle {
    pub givens: Grid,
    pub solution: Grid,
    pub difficulty: Difficulty,
    pub technique: Technique,
    pub seed: u64,
}

const MAX_ATTEMPTS: usize = 40;

/// Generates a unique-solution puzzle for `difficulty`, deterministically from `seed`.
pub fn generate(difficulty: Difficulty, seed: u64) -> Puzzle {
    let mut rng = Rng::new(seed);
    let (lo, hi) = difficulty.technique_range();
    let mut best: Option<(Puzzle, u32)> = None;

    for _ in 0..MAX_ATTEMPTS {
        let solution = random_solution(&mut rng);
        let givens = dig(&solution, difficulty.target_clues(), &mut rng);
        let technique = grade(&givens);
        let puzzle = Puzzle { givens, solution, difficulty, technique, seed };
        if technique >= lo && technique <= hi {
            return puzzle;
        }
        // distance from the acceptable band, used if we never hit it exactly
        let dist = if technique < lo { lo as u32 - technique as u32 } else { technique as u32 - hi as u32 };
        if best.as_ref().is_none_or(|(_, d)| dist < *d) {
            best = Some((puzzle, dist));
        }
    }
    best.expect("at least one attempt").0
}

/// Removes clues while uniqueness holds: first in 180°-symmetric pairs (prettier
/// layouts), then single cells to push below what symmetry alone allows.
fn dig(solution: &Grid, target: usize, rng: &mut Rng) -> Grid {
    let mut grid = *solution;
    let mut clues = 81;

    let mut pairs: Vec<usize> = (0..41).collect(); // 40 symmetric pairs + centre
    rng.shuffle(&mut pairs);
    for i in pairs {
        if clues <= target {
            return grid;
        }
        let j = 80 - i;
        let removed = if i == j { 1 } else { 2 };
        if clues - removed < target {
            continue;
        }
        let (a, b) = (grid.0[i], grid.0[j]);
        grid.0[i] = 0;
        grid.0[j] = 0;
        if has_unique_solution(&grid) {
            clues -= removed;
        } else {
            grid.0[i] = a;
            grid.0[j] = b;
        }
    }

    let mut singles: Vec<usize> = (0..81).filter(|&i| grid.0[i] != 0).collect();
    rng.shuffle(&mut singles);
    for i in singles {
        if clues <= target {
            break;
        }
        let a = grid.0[i];
        grid.0[i] = 0;
        if has_unique_solution(&grid) {
            clues -= 1;
        } else {
            grid.0[i] = a;
        }
    }
    grid
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::solve;

    #[test]
    fn every_difficulty_generates_unique_puzzles() {
        for (k, d) in Difficulty::ALL.into_iter().enumerate() {
            let p = generate(d, 1234 + k as u64);
            assert!(has_unique_solution(&p.givens), "{d:?}");
            assert_eq!(solve(&p.givens).unwrap(), p.solution);
            for i in 0..81 {
                if p.givens.0[i] != 0 {
                    assert_eq!(p.givens.0[i], p.solution.0[i]);
                }
            }
            println!("{d:?}: {} clues, {:?}", p.givens.filled(), p.technique);
        }
    }

    #[test]
    fn generation_is_fast() {
        let start = std::time::Instant::now();
        for seed in 0..20 {
            generate(Difficulty::Extreme, seed);
        }
        println!("20 extreme puzzles in {:?}", start.elapsed());
    }

    #[test]
    fn deterministic() {
        let a = generate(Difficulty::Hard, 99);
        let b = generate(Difficulty::Hard, 99);
        assert_eq!(a.givens, b.givens);
    }
}
