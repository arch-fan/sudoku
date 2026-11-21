//! Compact, versionable serde representation of a [`Game`].
//! (Fixed-size `[_; 81]` arrays aren't serde-friendly, so we go through `Vec`s/strings.)

use serde::{Deserialize, Serialize};

use crate::difficulty::Difficulty;
use crate::game::{Cell, Game, GameStatus};
use crate::generator::Puzzle;
use crate::grader::Technique;
use crate::grid::{Grid, Notes};

#[derive(Serialize, Deserialize)]
pub(crate) struct GameRepr {
    givens: String,
    solution: String,
    difficulty: Difficulty,
    technique: Technique,
    seed: u64,
    values: String,
    notes: Vec<u16>,
    /// One entry per user action: previous `(cell, value, notes)` of every touched cell.
    history: Vec<Vec<(u8, u8, u16)>>,
    scored: Vec<bool>,
    mistakes: u8,
    max_mistakes: u8,
    hints_left: u8,
    #[serde(default)]
    hints_used: u8,
    score: u32,
    streak: u32,
    elapsed_secs: u64,
    status: GameStatus,
}

fn grid_string(g: &Grid) -> String {
    g.0.iter().map(|&d| (b'0' + d) as char).collect()
}

impl From<Game> for GameRepr {
    fn from(g: Game) -> Self {
        let mut values = Grid::default();
        for i in 0..81 {
            values.0[i] = g.cells[i].value;
        }
        GameRepr {
            givens: grid_string(&g.puzzle.givens),
            solution: grid_string(&g.puzzle.solution),
            difficulty: g.puzzle.difficulty,
            technique: g.puzzle.technique,
            seed: g.puzzle.seed,
            values: grid_string(&values),
            notes: g.cells.iter().map(|c| c.notes.0).collect(),
            history: g
                .history
                .iter()
                .map(|change| change.iter().map(|(i, c)| (*i as u8, c.value, c.notes.0)).collect())
                .collect(),
            scored: g.scored.to_vec(),
            mistakes: g.mistakes,
            max_mistakes: g.max_mistakes,
            hints_left: g.hints_left,
            hints_used: g.hints_used,
            score: g.score,
            streak: g.streak,
            elapsed_secs: g.elapsed_secs,
            status: g.status,
        }
    }
}

impl TryFrom<GameRepr> for Game {
    type Error = String;

    fn try_from(r: GameRepr) -> Result<Self, Self::Error> {
        let givens = Grid::parse(&r.givens).ok_or("bad givens")?;
        let solution = Grid::parse(&r.solution).ok_or("bad solution")?;
        let values = Grid::parse(&r.values).ok_or("bad values")?;
        if r.notes.len() != 81 || r.scored.len() != 81 {
            return Err("bad lengths".into());
        }
        let puzzle = Puzzle { givens, solution, difficulty: r.difficulty, technique: r.technique, seed: r.seed };
        let mut game = Game::new(puzzle);
        for i in 0..81 {
            game.cells[i] = Cell { value: values.0[i], given: givens.0[i] != 0, notes: Notes(r.notes[i]) };
            game.scored[i] = r.scored[i];
        }
        game.history = r
            .history
            .into_iter()
            .map(|change| {
                change
                    .into_iter()
                    .filter(|(i, ..)| (*i as usize) < 81)
                    .map(|(i, v, n)| (i as usize, Cell { value: v, given: givens.0[i as usize] != 0, notes: Notes(n) }))
                    .collect::<Vec<_>>()
            })
            .filter(|c: &Vec<_>| !c.is_empty())
            .collect();
        game.mistakes = r.mistakes;
        game.max_mistakes = r.max_mistakes;
        game.hints_left = r.hints_left;
        game.hints_used = r.hints_used;
        game.score = r.score;
        game.streak = r.streak;
        game.elapsed_secs = r.elapsed_secs;
        game.status = r.status;
        Ok(game)
    }
}

#[cfg(test)]
mod tests {
    use crate::{Difficulty, Game, generate};

    #[test]
    fn roundtrip() {
        let mut g = Game::new(generate(Difficulty::Hard, 5));
        let i = (0..81).find(|&i| g.cell(i).value == 0).unwrap();
        g.toggle_note(i, 3);
        g.place(i, g.solution(i));
        g.tick(42);
        let json = serde_json::to_string(&g).unwrap();
        let back: Game = serde_json::from_str(&json).unwrap();
        assert_eq!(back.elapsed_secs, 42);
        assert_eq!(back.cell(i).value, g.solution(i));
        assert_eq!(back.score, g.score);
        assert!(back.can_undo());
    }
}
