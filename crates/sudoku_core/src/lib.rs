//! # sudoku_core
//!
//! Pure, dependency-free Sudoku engine shared by every frontend.
//!
//! * [`grid`]       – board geometry (rows, columns, boxes, peers) and bit-set helpers.
//! * [`solver`]     – fast bitmask backtracking solver / solution counter.
//! * [`grader`]     – human-style logical solver used to rate puzzle difficulty.
//! * [`generator`]  – unique-solution puzzle generator per [`Difficulty`].
//! * [`game`]       – a playable game: placements, notes, undo, mistakes, hints, score.
//! * [`hint`]       – explainable hints (what to look at, why, and the answer).
//! * [`rng`]        – tiny deterministic PRNG (no `getrandom`, works on wasm out of the box).

pub mod difficulty;
pub mod game;
pub mod generator;
pub mod grader;
pub mod grid;
pub mod hint;
pub mod rng;
#[cfg(feature = "serde")]
mod save;
pub mod solver;

pub use difficulty::Difficulty;
pub use game::{Cell, Game, GameStatus, MoveOutcome, Unit};
pub use generator::{Puzzle, generate};
pub use grid::{Digit, Grid, Idx, Notes};
pub use hint::{Hint, HintKind, find_hint};
pub use rng::Rng;
