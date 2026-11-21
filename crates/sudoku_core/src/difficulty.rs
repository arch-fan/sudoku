//! Difficulty levels and their generation / scoring parameters.

use crate::grader::Technique;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Difficulty {
    Easy,
    #[default]
    Medium,
    Hard,
    Expert,
    Extreme,
}

impl Difficulty {
    pub const ALL: [Difficulty; 5] =
        [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard, Difficulty::Expert, Difficulty::Extreme];

    /// Stable index `0..5`, handy for per-difficulty tables.
    pub fn index(self) -> usize {
        self as usize
    }

    /// Number of givens the generator aims for (it stops removing at this point).
    pub fn target_clues(self) -> usize {
        match self {
            Difficulty::Easy => 43,
            Difficulty::Medium => 35,
            Difficulty::Hard => 29,
            Difficulty::Expert => 25,
            Difficulty::Extreme => 17, // as low as uniqueness allows
        }
    }

    /// Acceptable range of required techniques.
    pub fn technique_range(self) -> (Technique, Technique) {
        use Technique::*;
        match self {
            Difficulty::Easy => (Singles, Singles),
            Difficulty::Medium => (Singles, LockedCandidates),
            Difficulty::Hard => (LockedCandidates, Subsets),
            Difficulty::Expert => (Subsets, Advanced),
            Difficulty::Extreme => (Advanced, Advanced),
        }
    }

    /// Points for each correct placement.
    pub fn base_points(self) -> u32 {
        match self {
            Difficulty::Easy => 10,
            Difficulty::Medium => 20,
            Difficulty::Hard => 35,
            Difficulty::Expert => 50,
            Difficulty::Extreme => 80,
        }
    }

    pub fn hints(self) -> u8 {
        match self {
            Difficulty::Easy | Difficulty::Medium => 3,
            Difficulty::Hard | Difficulty::Expert => 2,
            Difficulty::Extreme => 1,
        }
    }
}
