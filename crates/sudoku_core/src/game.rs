//! A playable game: the state machine every frontend drives.
//!
//! All mutating calls return a [`MoveOutcome`] describing *what happened*,
//! so the UI can trigger the right animation without diffing state.

use crate::difficulty::Difficulty;
use crate::generator::Puzzle;
use crate::grid::{Digit, Idx, Notes, PEERS, UNITS, box_of, col_of, row_of};

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub struct Cell {
    pub value: Digit,
    pub given: bool,
    pub notes: Notes,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum GameStatus {
    Playing,
    Won,
    Lost,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Unit {
    Row(u8),
    Col(u8),
    Box(u8),
}

impl Unit {
    pub fn cells(self) -> &'static [u8; 9] {
        match self {
            Unit::Row(r) => &UNITS[r as usize],
            Unit::Col(c) => &UNITS[9 + c as usize],
            Unit::Box(b) => &UNITS[18 + b as usize],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MoveOutcome {
    /// Nothing changed (locked cell, game over, no-op…).
    Ignored,
    Placed {
        idx: Idx,
        digit: Digit,
        correct: bool,
        /// Rows/columns/boxes completed by this placement.
        completed: Vec<Unit>,
        /// All nine copies of `digit` are now on the board.
        digit_done: bool,
        /// Peer cells whose note for `digit` was auto-removed.
        cleared_notes: Vec<Idx>,
        points: u32,
        status: GameStatus,
    },
    NoteToggled {
        idx: Idx,
        digit: Digit,
        on: bool,
    },
    Erased {
        idx: Idx,
    },
    Undone {
        idx: Idx,
    },
}

/// The previous contents of every cell touched by one user action.
pub(crate) type Change = Vec<(Idx, Cell)>;

#[derive(Clone, Debug)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(into = "crate::save::GameRepr", try_from = "crate::save::GameRepr")
)]
pub struct Game {
    pub(crate) puzzle: Puzzle,
    pub(crate) cells: [Cell; 81],
    pub(crate) history: Vec<Change>,
    pub(crate) scored: [bool; 81],
    pub mistakes: u8,
    pub max_mistakes: u8,
    pub hints_left: u8,
    pub hints_used: u8,
    pub score: u32,
    pub streak: u32,
    pub elapsed_secs: u64,
    pub status: GameStatus,
}

impl Game {
    pub fn new(puzzle: Puzzle) -> Game {
        let mut cells = [Cell::default(); 81];
        for (i, cell) in cells.iter_mut().enumerate() {
            let d = puzzle.givens.0[i];
            *cell = Cell { value: d, given: d != 0, notes: Notes::EMPTY };
        }
        Game {
            hints_left: puzzle.difficulty.hints(),
            hints_used: 0,
            puzzle,
            cells,
            history: Vec::new(),
            scored: [false; 81],
            mistakes: 0,
            max_mistakes: 3,
            score: 0,
            streak: 0,
            elapsed_secs: 0,
            status: GameStatus::Playing,
        }
    }

    // ---------------------------------------------------------------- queries

    pub fn puzzle(&self) -> &Puzzle {
        &self.puzzle
    }
    pub fn difficulty(&self) -> Difficulty {
        self.puzzle.difficulty
    }
    pub fn cell(&self, i: Idx) -> Cell {
        self.cells[i]
    }
    pub fn solution(&self, i: Idx) -> Digit {
        self.puzzle.solution.0[i]
    }
    pub fn is_correct(&self, i: Idx) -> bool {
        self.cells[i].value != 0 && self.cells[i].value == self.solution(i)
    }
    pub fn is_wrong(&self, i: Idx) -> bool {
        self.cells[i].value != 0 && self.cells[i].value != self.solution(i)
    }
    /// Givens and correctly solved cells can't be changed.
    pub fn is_locked(&self, i: Idx) -> bool {
        self.cells[i].given || self.is_correct(i)
    }
    pub fn can_undo(&self) -> bool {
        !self.history.is_empty() && self.status == GameStatus::Playing
    }
    /// How many *correct* copies of `d` are on the board.
    pub fn digit_count(&self, d: Digit) -> usize {
        (0..81).filter(|&i| self.cells[i].value == d && self.is_correct(i)).count()
    }
    /// Fraction of non-given cells solved, `0.0..=1.0`.
    pub fn progress(&self) -> f32 {
        let open = (0..81).filter(|&i| !self.cells[i].given).count().max(1);
        let done = (0..81).filter(|&i| !self.cells[i].given && self.is_correct(i)).count();
        done as f32 / open as f32
    }
    /// Whether `i` and `j` share a row, column or box.
    pub fn are_peers(i: Idx, j: Idx) -> bool {
        row_of(i) == row_of(j) || col_of(i) == col_of(j) || box_of(i) == box_of(j)
    }

    pub fn tick(&mut self, secs: u64) {
        if self.status == GameStatus::Playing {
            self.elapsed_secs += secs;
        }
    }

    // -------------------------------------------------------------- mutations

    /// Enter `digit` in cell `i`, as a value or as a note.
    pub fn input(&mut self, i: Idx, digit: Digit, notes_mode: bool) -> MoveOutcome {
        if notes_mode { self.toggle_note(i, digit) } else { self.place(i, digit) }
    }

    pub fn place(&mut self, i: Idx, digit: Digit) -> MoveOutcome {
        if self.status != GameStatus::Playing || self.is_locked(i) || self.cells[i].value == digit {
            return MoveOutcome::Ignored;
        }
        self.place_inner(i, digit, true)
    }

    fn place_inner(&mut self, i: Idx, digit: Digit, award: bool) -> MoveOutcome {
        let mut change: Change = vec![(i, self.cells[i])];
        self.cells[i].value = digit;
        self.cells[i].notes = Notes::EMPTY;

        let correct = digit == self.solution(i);
        let mut cleared_notes = Vec::new();
        let mut completed = Vec::new();
        let mut points = 0;

        if correct {
            // Auto-remove this digit from the notes of every peer.
            for &p in &PEERS[i] {
                let p = p as usize;
                if self.cells[p].notes.contains(digit) {
                    change.push((p, self.cells[p]));
                    self.cells[p].notes.remove(digit);
                    cleared_notes.push(p);
                }
            }
            for unit in [Unit::Row(row_of(i) as u8), Unit::Col(col_of(i) as u8), Unit::Box(box_of(i) as u8)] {
                if unit.cells().iter().all(|&c| self.is_correct(c as usize)) {
                    completed.push(unit);
                }
            }
            if award && !self.scored[i] {
                self.streak += 1;
                let base = self.difficulty().base_points();
                let combo = 1 + (self.streak.min(12) / 4);
                points = base * combo + completed.len() as u32 * base * 3;
            }
            self.scored[i] = true;
            if self.cells.iter().enumerate().all(|(k, _)| self.is_correct(k)) {
                self.status = GameStatus::Won;
                if award {
                    let base = self.difficulty().base_points();
                    let speed = 1800u64.saturating_sub(self.elapsed_secs) as u32 / 10;
                    points += base * 10 + speed + (self.max_mistakes - self.mistakes) as u32 * base * 2;
                }
            }
            self.score += points;
        } else {
            self.streak = 0;
            self.mistakes += 1;
            if self.mistakes >= self.max_mistakes {
                self.status = GameStatus::Lost;
            }
        }
        self.history.push(change);

        MoveOutcome::Placed {
            idx: i,
            digit,
            correct,
            completed,
            digit_done: correct && self.digit_count(digit) == 9,
            cleared_notes,
            points,
            status: self.status,
        }
    }

    pub fn toggle_note(&mut self, i: Idx, digit: Digit) -> MoveOutcome {
        if self.status != GameStatus::Playing || self.is_locked(i) {
            return MoveOutcome::Ignored;
        }
        let mut change: Change = vec![(i, self.cells[i])];
        let cell = &mut self.cells[i];
        if cell.value != 0 {
            // Writing a note over a wrong value clears that value first.
            cell.value = 0;
        }
        cell.notes.toggle(digit);
        let on = cell.notes.contains(digit);
        change.truncate(1);
        self.history.push(change);
        MoveOutcome::NoteToggled { idx: i, digit, on }
    }

    pub fn erase(&mut self, i: Idx) -> MoveOutcome {
        if self.status != GameStatus::Playing || self.is_locked(i) {
            return MoveOutcome::Ignored;
        }
        let cell = self.cells[i];
        if cell.value == 0 && cell.notes.is_empty() {
            return MoveOutcome::Ignored;
        }
        self.history.push(vec![(i, cell)]);
        self.cells[i].value = 0;
        self.cells[i].notes = Notes::EMPTY;
        MoveOutcome::Erased { idx: i }
    }

    pub fn undo(&mut self) -> MoveOutcome {
        if self.status != GameStatus::Playing {
            return MoveOutcome::Ignored;
        }
        let Some(change) = self.history.pop() else { return MoveOutcome::Ignored };
        let idx = change[0].0;
        for (i, cell) in change.into_iter().rev() {
            self.cells[i] = cell;
        }
        MoveOutcome::Undone { idx }
    }

    /// Spends one hint. Returns `false` when none are left or the game is over.
    pub fn use_hint(&mut self) -> bool {
        if self.status != GameStatus::Playing || self.hints_left == 0 {
            return false;
        }
        self.hints_left -= 1;
        self.hints_used = self.hints_used.saturating_add(1);
        true
    }

    /// Places the solution digit in `i` (used to apply a hint). Gives no points.
    pub fn reveal(&mut self, i: Idx) -> MoveOutcome {
        if self.status != GameStatus::Playing || self.is_locked(i) {
            return MoveOutcome::Ignored;
        }
        self.scored[i] = true;
        self.place_inner(i, self.solution(i), false)
    }

    /// The board as the player has *correctly* filled it (givens + right answers).
    pub fn known_board(&self) -> crate::grid::Grid {
        let mut board = self.puzzle.givens;
        for i in 0..81 {
            if self.is_correct(i) {
                board.0[i] = self.cells[i].value;
            }
        }
        board
    }

    /// After losing: continue with one mistake left.
    pub fn second_chance(&mut self) {
        if self.status == GameStatus::Lost {
            self.mistakes = self.max_mistakes - 1;
            self.status = GameStatus::Playing;
        }
    }

    /// Back to the initial givens (same puzzle).
    pub fn restart(&mut self) {
        *self = Game::new(self.puzzle.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::generate;

    fn game() -> Game {
        Game::new(generate(Difficulty::Easy, 42))
    }

    fn empty_cell(g: &Game) -> Idx {
        (0..81).find(|&i| g.cell(i).value == 0).unwrap()
    }

    #[test]
    fn correct_placement_clears_peer_notes() {
        let mut g = game();
        let i = empty_cell(&g);
        let d = g.solution(i);
        let peer = PEERS[i].iter().map(|&p| p as usize).find(|&p| g.cell(p).value == 0).unwrap();
        g.toggle_note(peer, d);
        g.toggle_note(i, d);
        assert!(g.cell(peer).notes.contains(d));
        let out = g.place(i, d);
        assert!(matches!(out, MoveOutcome::Placed { correct: true, .. }));
        assert!(!g.cell(peer).notes.contains(d));
        assert!(g.cell(i).notes.is_empty());
        assert!(g.is_locked(i));
        // undo restores both the value and the notes
        g.undo();
        assert_eq!(g.cell(i).value, 0);
        assert!(g.cell(peer).notes.contains(d));
    }

    #[test]
    fn mistakes_lose_and_erase_works() {
        let mut g = game();
        let i = empty_cell(&g);
        let wrong = (1..=9).find(|&d| d != g.solution(i)).unwrap();
        let wrong2 = (1..=9).find(|&d| d != g.solution(i) && d != wrong).unwrap();
        let wrong3 = (1..=9).find(|&d| d != g.solution(i) && d != wrong && d != wrong2).unwrap();
        g.place(i, wrong);
        assert!(g.is_wrong(i));
        assert!(matches!(g.erase(i), MoveOutcome::Erased { .. }));
        assert_eq!(g.cell(i).value, 0);
        g.place(i, wrong2);
        g.place(i, wrong3);
        assert_eq!(g.status, GameStatus::Lost);
        g.second_chance();
        assert_eq!(g.status, GameStatus::Playing);
    }

    #[test]
    fn solving_everything_wins() {
        let mut g = game();
        for i in 0..81 {
            if g.cell(i).value == 0 {
                g.place(i, g.solution(i));
            }
        }
        assert_eq!(g.status, GameStatus::Won);
        assert!(g.score > 0);
    }
}
