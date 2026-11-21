//! Explainable hints.
//!
//! A [`Hint`] says *where to look* (`area`), *why* (`blockers`: the digits that
//! rule out every alternative) and *what* (`cell` = `digit`). The UI walks the
//! player through those three steps.

use crate::game::{Game, Unit};
use crate::grid::{Digit, Grid, Idx, Notes, PEERS, UNITS, box_of, col_of, row_of};

#[derive(Clone, Debug, PartialEq)]
pub enum HintKind {
    /// The player entered a wrong digit in `cell` (it's erased when applied).
    Mistake {
        wrong: Digit,
    },
    /// `cell` is the only empty cell of `unit`.
    FullHouse { unit: Unit },
    /// In `unit`, `digit` fits only in `cell`.
    HiddenSingle { unit: Unit },
    /// `digit` is the only candidate left for `cell`.
    NakedSingle,
    /// No simple deduction available: reveal the most constrained cell.
    Reveal { candidates: Notes },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hint {
    pub kind: HintKind,
    pub cell: Idx,
    /// The correct digit for `cell`.
    pub digit: Digit,
    /// Cells to spotlight while explaining.
    pub area: Vec<Idx>,
    /// Cells whose digits justify the deduction.
    pub blockers: Vec<Idx>,
}

fn unit_cells(u: Unit) -> Vec<Idx> {
    u.cells().iter().map(|&c| c as usize).collect()
}

fn units_of(i: Idx) -> [Unit; 3] {
    [Unit::Box(box_of(i) as u8), Unit::Row(row_of(i) as u8), Unit::Col(col_of(i) as u8)]
}

/// Finds the friendliest next step, preferring the `preferred` (selected) cell.
pub fn find_hint(game: &Game, preferred: Option<Idx>) -> Option<Hint> {
    // 1. Fix mistakes first: deductions are meaningless on a wrong board.
    let wrong = preferred.filter(|&i| game.is_wrong(i)).or_else(|| (0..81).find(|&i| game.is_wrong(i)));
    if let Some(i) = wrong {
        let d = game.cell(i).value;
        let blockers =
            PEERS[i].iter().map(|&p| p as usize).filter(|&p| game.cell(p).value == d).collect();
        return Some(Hint { kind: HintKind::Mistake { wrong: d }, cell: i, digit: game.solution(i), area: vec![i], blockers });
    }

    let board = game.known_board();
    let empty = |i: Idx| board.0[i] == 0;

    // 2. A simple deduction on the selected cell, if there is one.
    if let Some(i) = preferred.filter(|&i| empty(i)) {
        if let Some(h) = full_house_at(&board, i).or_else(|| hidden_single_at(&board, i)).or_else(|| naked_single_at(&board, i)) {
            return Some(h);
        }
    }

    // 3. Anywhere on the board, easiest technique first.
    for u in 0..27 {
        let cells = &UNITS[u];
        let empties: Vec<Idx> = cells.iter().map(|&c| c as usize).filter(|&c| empty(c)).collect();
        if empties.len() == 1 {
            if let Some(h) = full_house_at(&board, empties[0]) {
                return Some(h);
            }
        }
    }
    for pass in [0usize, 1] {
        for i in (0..81).filter(|&i| empty(i)) {
            let h = if pass == 0 { hidden_single_at(&board, i) } else { naked_single_at(&board, i) };
            if h.is_some() {
                return h;
            }
        }
    }

    // 4. Fallback: reveal the most constrained cell.
    let i = preferred.filter(|&i| empty(i)).or_else(|| (0..81).filter(|&i| empty(i)).min_by_key(|&i| board.candidates(i).len()))?;
    Some(Hint {
        kind: HintKind::Reveal { candidates: board.candidates(i) },
        cell: i,
        digit: game.solution(i),
        area: units_of(i).iter().flat_map(|&u| unit_cells(u)).collect(),
        blockers: vec![],
    })
}

fn full_house_at(board: &Grid, i: Idx) -> Option<Hint> {
    for unit in units_of(i) {
        let cells = unit_cells(unit);
        if cells.iter().filter(|&&c| board.0[c] == 0).count() == 1 {
            let used = cells.iter().fold(0u16, |m, &c| m | 1 << board.0[c]);
            let digit = (1..=9).find(|&d| used & (1 << d) == 0)?;
            return Some(Hint { kind: HintKind::FullHouse { unit }, cell: i, digit, area: cells, blockers: vec![] });
        }
    }
    None
}

fn hidden_single_at(board: &Grid, i: Idx) -> Option<Hint> {
    let cands = board.candidates(i);
    for unit in units_of(i) {
        let cells = unit_cells(unit);
        for d in cands.iter() {
            let others: Vec<Idx> = cells.iter().copied().filter(|&c| c != i && board.0[c] == 0).collect();
            if others.iter().all(|&c| !board.candidates(c).contains(d)) {
                // For each other empty cell, one peer already holding `d` explains why it can't.
                let mut blockers: Vec<Idx> = Vec::new();
                for &c in &others {
                    if let Some(p) = PEERS[c]
                        .iter()
                        .map(|&p| p as usize)
                        .filter(|&p| board.0[p] == d)
                        .min_by_key(|&p| (blockers.contains(&p) as u8 ^ 1, p))
                    {
                        if !blockers.contains(&p) {
                            blockers.push(p);
                        }
                    }
                }
                return Some(Hint { kind: HintKind::HiddenSingle { unit }, cell: i, digit: d, area: cells, blockers });
            }
        }
    }
    None
}

fn naked_single_at(board: &Grid, i: Idx) -> Option<Hint> {
    let cands = board.candidates(i);
    if cands.len() != 1 {
        return None;
    }
    let digit = cands.iter().next()?;
    let mut blockers = Vec::new();
    for d in (1..=9).filter(|&d| d != digit) {
        if let Some(p) = PEERS[i].iter().map(|&p| p as usize).find(|&p| board.0[p] == d) {
            blockers.push(p);
        }
    }
    let area = units_of(i).iter().flat_map(|&u| unit_cells(u)).collect();
    Some(Hint { kind: HintKind::NakedSingle, cell: i, digit, area, blockers })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Difficulty, generate};

    #[test]
    fn hints_are_always_correct_and_solve_the_puzzle() {
        for d in Difficulty::ALL {
            let mut g = Game::new(generate(d, 77));
            let mut steps = 0;
            while g.status == crate::GameStatus::Playing {
                let h = find_hint(&g, None).expect("a hint while playing");
                assert_eq!(h.digit, g.solution(h.cell));
                g.reveal(h.cell);
                steps += 1;
                assert!(steps <= 81);
            }
            assert_eq!(g.status, crate::GameStatus::Won);
        }
    }

    #[test]
    fn mistakes_come_first() {
        let mut g = Game::new(generate(Difficulty::Easy, 3));
        let i = (0..81).find(|&i| g.cell(i).value == 0).unwrap();
        let wrong = (1..=9).find(|&d| d != g.solution(i)).unwrap();
        g.place(i, wrong);
        let h = find_hint(&g, None).unwrap();
        assert_eq!(h.cell, i);
        assert!(matches!(h.kind, HintKind::Mistake { .. }));
    }

    #[test]
    fn hidden_single_blockers_cover_alternatives() {
        let g = Game::new(generate(Difficulty::Medium, 11));
        let h = find_hint(&g, None).unwrap();
        if let HintKind::HiddenSingle { unit } = h.kind {
            let board = g.known_board();
            for c in unit_cells(unit).into_iter().filter(|&c| c != h.cell && board.0[c] == 0) {
                assert!(h.blockers.iter().any(|&b| Game::are_peers(b, c) && board.0[b] == h.digit));
            }
        }
    }
}
