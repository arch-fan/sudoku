//! Everything that survives a restart: settings, statistics and the game in
//! progress. Serialized as JSON into `localStorage` on the web, or a file in
//! the OS app-data directory natively. No server involved.

use serde::{Deserialize, Serialize};
use sudoku_core::{Difficulty, Game};

use crate::i18n::Lang;

const SAVE_VERSION: u32 = 1;

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub lang: Lang,
    pub theme: usize,
    pub mistake_limit: bool,
    pub show_timer: bool,
    pub highlight_same: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            lang: crate::platform::preferred_lang(),
            theme: 0,
            mistake_limit: true,
            show_timer: true,
            highlight_same: true,
        }
    }
}

/// A finished (won) game.
#[derive(Clone, Serialize, Deserialize)]
pub struct Record {
    pub score: u32,
    pub time_secs: u64,
    pub mistakes: u8,
    pub hints: u8,
    /// Unix timestamp (seconds, UTC).
    pub date: i64,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DifficultyStats {
    pub played: u32,
    pub won: u32,
    pub total_time: u64,
    pub best_time: Option<u64>,
    pub best_score: u32,
    /// Best games, highest score first.
    pub records: Vec<Record>,
}

impl DifficultyStats {
    pub fn win_rate(&self) -> f32 {
        if self.played == 0 { 0.0 } else { self.won as f32 / self.played as f32 }
    }
    pub fn avg_time(&self) -> Option<u64> {
        (self.won > 0).then(|| self.total_time / self.won as u64)
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Stats {
    pub per: [DifficultyStats; 5],
    pub streak: u32,
    pub best_streak: u32,
}

/// Outcome of recording a win, used to celebrate.
pub struct WinInfo {
    pub best_score: bool,
    pub best_time: bool,
}

impl Stats {
    pub fn get(&self, d: Difficulty) -> &DifficultyStats {
        &self.per[d.index()]
    }

    pub fn total_wins(&self) -> u32 {
        self.per.iter().map(|s| s.won).sum()
    }

    pub fn record_start(&mut self, d: Difficulty) {
        self.per[d.index()].played += 1;
    }

    pub fn record_loss(&mut self) {
        self.streak = 0;
    }

    pub fn record_win(&mut self, game: &Game) -> WinInfo {
        let s = &mut self.per[game.difficulty().index()];
        let t = game.elapsed_secs;
        let info = WinInfo {
            best_score: game.score > s.best_score,
            best_time: s.best_time.is_none_or(|b| t < b),
        };
        s.won += 1;
        s.total_time += t;
        s.best_time = Some(s.best_time.map_or(t, |b| b.min(t)));
        s.best_score = s.best_score.max(game.score);
        s.records.push(Record {
            score: game.score,
            time_secs: t,
            mistakes: game.mistakes,
            hints: game.hints_used,
            date: chrono::Utc::now().timestamp(),
        });
        s.records.sort_by(|a, b| b.score.cmp(&a.score).then(a.time_secs.cmp(&b.time_secs)));
        s.records.truncate(25);
        self.streak += 1;
        self.best_streak = self.best_streak.max(self.streak);
        info
    }
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SaveData {
    pub version: u32,
    pub settings: Settings,
    pub stats: Stats,
    /// Game in progress, restored on launch.
    pub current: Option<Game>,
}

impl SaveData {
    pub fn load() -> SaveData {
        crate::platform::read_save()
            .and_then(|json| serde_json::from_str::<SaveData>(&json).ok())
            .unwrap_or_default()
    }

    pub fn store(&mut self) {
        self.version = SAVE_VERSION;
        match serde_json::to_string(self) {
            Ok(json) => crate::platform::write_save(&json),
            Err(err) => eprintln!("failed to serialize save: {err}"),
        }
    }
}

/// `dd/mm/yyyy · hh:mm` in local time.
pub fn format_date(ts: i64) -> String {
    use chrono::TimeZone;
    match chrono::Local.timestamp_opt(ts, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%d/%m/%Y · %H:%M").to_string(),
        _ => String::new(),
    }
}
