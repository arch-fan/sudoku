//! Root view: navigation, the [`Game`], persistence and per-cell effect bookkeeping.

use std::time::Duration;

use gpui::{
    Context, FocusHandle, KeyDownEvent, MouseButton, Pixels, Subscription, Task, Window, div,
    prelude::*, px,
};
use sudoku_core::{
    Difficulty, Game, GameStatus, Hint, HintKind, MoveOutcome, Notes, find_hint, generate, grid::PEERS,
};

use crate::assets::FONT_FAMILY;
use crate::i18n::{Lang, Strings};
use crate::platform;
use crate::store::SaveData;
use crate::theme::{THEMES, Theme};

/// One-shot visual effect on a cell's content.
#[derive(Clone, Copy, Default, PartialEq)]
pub enum ContentFx {
    #[default]
    None,
    Pop,
    Wrong,
    Hint,
    Undo,
    Intro { delay_ms: u64 },
}

/// One-shot flash/ripple behind a cell.
#[derive(Clone, Copy, Default, PartialEq)]
pub enum FlashFx {
    #[default]
    None,
    Wave { delay_ms: u64 },
    Hint,
    Wrong,
}

#[derive(Clone, Copy, Default)]
pub struct Fx<T> {
    pub kind: T,
    pub generation: u64,
}

/// A digit that just disappeared from a cell and fades out.
#[derive(Clone, Copy, Default)]
pub struct Ghost {
    pub digit: u8,
    pub value: bool,
    pub generation: u64,
}

#[derive(Clone, Copy, Default)]
pub struct NoteFx {
    pub digit: u8,
    pub generation: u64,
}

pub struct FloatingPoints {
    pub idx: usize,
    pub points: u32,
    pub generation: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Home,
    Game,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Play,
    Records,
    Settings,
}

/// A guided hint being walked through: look → why → answer.
pub struct HintSession {
    pub hint: Hint,
    pub step: usize,
    pub generation: u64,
    /// Digits visible from the hint cell (for "single candidate" explanations).
    pub seen: Notes,
}

pub const HINT_STEPS: usize = 3;

pub struct SudokuApp {
    focus: FocusHandle,
    pub save: SaveData,

    // navigation
    pub screen: Screen,
    pub tab: Tab,
    pub tab_generation: u64,
    pub sheet_open: bool,
    pub sheet_generation: u64,
    pub records_filter: Difficulty,
    pub records_generation: u64,
    pub reset_armed: bool,

    // game
    pub game: Game,
    /// A game that can be continued from the home screen.
    pub has_game: bool,
    pub selected: Option<usize>,
    pub notes_mode: bool,
    pub paused: bool,
    pub hint: Option<HintSession>,
    pub new_record: bool,

    // effects
    pub generation: u64,
    pub board_generation: u64,
    pub selection_generation: u64,
    pub content_fx: [Fx<ContentFx>; 81],
    pub flash_fx: [Fx<FlashFx>; 81],
    pub ghosts: [Ghost; 81],
    pub note_fx: [NoteFx; 81],
    pub hl_track: [crate::board::HlTrack; 81],
    pub floating: Option<FloatingPoints>,
    /// Streak celebration in the middle of the screen: `(streak, generation)`.
    pub streak_burst: Option<(u32, u64)>,
    pub score_from: u32,
    pub score_generation: u64,
    pub mistake_generation: u64,
    pub combo_generation: u64,
    pub pad_generation: [u64; 10],
    pub action_generation: [u64; 4],
    pub no_hint_generation: u64,
    pub end_generation: u64,
    pub pause_generation: u64,

    ticks_since_save: u32,
    _timer: Task<()>,
    _activation: Subscription,
}

impl SudokuApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let timer = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                if this.update(cx, |this, cx| this.tick(cx)).is_err() {
                    break;
                }
            }
        });
        // Auto-pause when the app goes to the background (switching apps on iOS, etc.).
        let activation = cx.observe_window_activation(window, |this, window, cx| {
            if !window.is_window_active() && this.screen == Screen::Game && this.playing() && !this.paused {
                this.paused = true;
                this.pause_generation = this.next_gen();
                this.persist();
                cx.notify();
            }
        });

        let mut save = SaveData::load();
        save.settings.theme = save.settings.theme.min(THEMES.len() - 1);
        let current = save.current.take().filter(|g| g.status == GameStatus::Playing);
        let has_game = current.is_some();
        let game = current.unwrap_or_else(|| Game::new(generate(Difficulty::Medium, seed())));

        let app = SudokuApp {
            focus: cx.focus_handle(),
            save,
            screen: Screen::Home,
            tab: Tab::Play,
            tab_generation: 1,
            sheet_open: false,
            sheet_generation: 0,
            records_filter: game.difficulty(),
            records_generation: 1,
            reset_armed: false,
            game,
            has_game,
            selected: None,
            notes_mode: false,
            paused: false,
            hint: None,
            new_record: false,
            generation: 10,
            board_generation: 1,
            selection_generation: 0,
            content_fx: [Fx::default(); 81],
            flash_fx: [Fx::default(); 81],
            ghosts: [Ghost::default(); 81],
            note_fx: [NoteFx::default(); 81],
            hl_track: [Default::default(); 81],
            floating: None,
            streak_burst: None,
            score_from: 0,
            score_generation: 0,
            mistake_generation: 0,
            combo_generation: 0,
            pad_generation: [0; 10],
            action_generation: [0; 4],
            no_hint_generation: 0,
            end_generation: 0,
            pause_generation: 0,
            ticks_since_save: 0,
            _timer: timer,
            _activation: activation,
        };
        platform::set_chrome_color(app.theme().bg_hex());
        app
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus
    }

    pub fn theme(&self) -> &'static Theme {
        &THEMES[self.save.settings.theme]
    }

    pub fn lang(&self) -> Lang {
        self.save.settings.lang
    }

    pub fn t(&self) -> &'static Strings {
        self.lang().t()
    }

    pub fn next_gen(&mut self) -> u64 {
        self.generation += 1;
        self.generation
    }

    pub fn playing(&self) -> bool {
        self.game.status == GameStatus::Playing
    }

    fn input_blocked(&self) -> bool {
        self.screen != Screen::Game || self.paused || self.hint.is_some() || !self.playing()
    }

    fn tick(&mut self, cx: &mut Context<Self>) {
        if self.screen == Screen::Game && !self.paused && self.playing() {
            self.game.tick(1);
            self.ticks_since_save += 1;
            if self.ticks_since_save >= 5 {
                self.persist();
            }
            cx.notify();
        }
    }

    /// Writes settings, stats and the game in progress to local storage.
    pub fn persist(&mut self) {
        self.ticks_since_save = 0;
        self.save.current = (self.has_game && self.playing()).then(|| self.game.clone());
        self.save.store();
        self.save.current = None;
    }

    // ---------------------------------------------------------- navigation

    pub fn set_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        if self.tab != tab {
            self.tab = tab;
            self.tab_generation = self.next_gen();
            self.reset_armed = false;
        }
        cx.notify();
    }

    pub fn go_home(&mut self, tab: Tab, cx: &mut Context<Self>) {
        self.screen = Screen::Home;
        self.hint = None;
        self.sheet_open = false;
        if self.game.status == GameStatus::Won {
            self.records_filter = self.game.difficulty();
        }
        self.set_tab(tab, cx);
        self.tab_generation = self.next_gen();
        self.persist();
        cx.notify();
    }

    pub fn open_sheet(&mut self, cx: &mut Context<Self>) {
        self.sheet_open = true;
        self.sheet_generation = self.next_gen();
        cx.notify();
    }

    pub fn close_sheet(&mut self, cx: &mut Context<Self>) {
        self.sheet_open = false;
        cx.notify();
    }

    pub fn continue_game(&mut self, cx: &mut Context<Self>) {
        self.screen = Screen::Game;
        self.paused = false;
        self.board_generation = self.next_gen();
        self.intro_wave();
        cx.notify();
    }

    pub fn set_records_filter(&mut self, d: Difficulty, cx: &mut Context<Self>) {
        self.records_filter = d;
        self.records_generation = self.next_gen();
        cx.notify();
    }

    // ------------------------------------------------------------ settings

    pub fn set_lang(&mut self, lang: Lang, cx: &mut Context<Self>) {
        self.save.settings.lang = lang;
        self.persist();
        cx.notify();
    }

    pub fn set_theme(&mut self, ix: usize, cx: &mut Context<Self>) {
        self.save.settings.theme = ix % THEMES.len();
        platform::set_chrome_color(self.theme().bg_hex());
        self.persist();
        cx.notify();
    }

    pub fn cycle_theme(&mut self, cx: &mut Context<Self>) {
        self.set_theme(self.save.settings.theme + 1, cx);
    }

    fn max_mistakes(&self) -> u8 {
        if self.save.settings.mistake_limit { 3 } else { u8::MAX }
    }

    pub fn toggle_mistake_limit(&mut self, cx: &mut Context<Self>) {
        self.save.settings.mistake_limit ^= true;
        if self.playing() {
            self.game.max_mistakes = self.max_mistakes().max(self.game.mistakes + 1);
        }
        self.persist();
        cx.notify();
    }

    pub fn toggle_show_timer(&mut self, cx: &mut Context<Self>) {
        self.save.settings.show_timer ^= true;
        self.persist();
        cx.notify();
    }

    pub fn toggle_highlight_same(&mut self, cx: &mut Context<Self>) {
        self.save.settings.highlight_same ^= true;
        self.persist();
        cx.notify();
    }

    pub fn reset_stats(&mut self, cx: &mut Context<Self>) {
        if self.reset_armed {
            self.save.stats = Default::default();
            self.reset_armed = false;
            self.records_generation = self.next_gen();
            self.persist();
        } else {
            self.reset_armed = true;
        }
        cx.notify();
    }

    // ------------------------------------------------------------- actions

    pub fn new_game(&mut self, difficulty: Difficulty, cx: &mut Context<Self>) {
        if self.has_game && self.playing() && self.game.progress() > 0.0 {
            self.save.stats.record_loss(); // abandoning breaks the streak
        }
        self.save.stats.record_start(difficulty);
        self.game = Game::new(generate(difficulty, seed()));
        self.game.max_mistakes = self.max_mistakes();
        self.has_game = true;
        self.screen = Screen::Game;
        self.sheet_open = false;
        self.selected = None;
        self.notes_mode = false;
        self.paused = false;
        self.hint = None;
        self.new_record = false;
        self.floating = None;
        self.streak_burst = None;
        self.score_from = 0;
        self.ghosts = [Ghost::default(); 81];
        self.flash_fx = [Fx::default(); 81];
        self.note_fx = [NoteFx::default(); 81];
        self.board_generation = self.next_gen();
        self.intro_wave();
        self.persist();
        cx.notify();
    }

    fn intro_wave(&mut self) {
        let generation = self.next_gen();
        for i in 0..81 {
            let (r, c) = ((i / 9) as u64, (i % 9) as u64);
            self.content_fx[i] = Fx { kind: ContentFx::Intro { delay_ms: (r + c) * 22 }, generation };
        }
    }

    pub fn toggle_pause(&mut self, cx: &mut Context<Self>) {
        if !self.playing() || self.screen != Screen::Game {
            return;
        }
        self.paused = !self.paused;
        self.pause_generation = self.next_gen();
        self.persist();
        cx.notify();
    }

    pub fn select(&mut self, i: usize, cx: &mut Context<Self>) {
        if self.input_blocked() {
            return;
        }
        if self.selected != Some(i) {
            self.selected = Some(i);
            self.selection_generation = self.next_gen();
        }
        cx.notify();
    }

    fn move_selection(&mut self, dr: i32, dc: i32, cx: &mut Context<Self>) {
        let Some(i) = self.selected else {
            self.select(40, cx);
            return;
        };
        let (r, c) = ((i as i32 / 9 + dr).rem_euclid(9), (i as i32 % 9 + dc).rem_euclid(9));
        self.select((r * 9 + c) as usize, cx);
    }

    pub fn press_digit(&mut self, d: u8, cx: &mut Context<Self>) {
        if self.input_blocked() {
            return;
        }
        self.pad_generation[d as usize] = self.next_gen();
        if let Some(i) = self.selected {
            let out = self.game.input(i, d, self.notes_mode);
            self.apply(out);
        }
        cx.notify();
    }

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        if self.input_blocked() {
            return;
        }
        self.action_generation[0] = self.next_gen();
        let out = self.game.undo();
        self.apply(out);
        cx.notify();
    }

    pub fn erase(&mut self, cx: &mut Context<Self>) {
        if self.input_blocked() {
            return;
        }
        self.action_generation[1] = self.next_gen();
        if let Some(i) = self.selected {
            self.erase_cell(i);
        }
        cx.notify();
    }

    fn erase_cell(&mut self, i: usize) {
        let before = self.game.cell(i);
        if let MoveOutcome::Erased { idx } = self.game.erase(i) {
            let generation = self.next_gen();
            if before.value != 0 {
                self.ghosts[idx] = Ghost { digit: before.value, value: true, generation };
            }
            self.content_fx[idx] = Fx { kind: ContentFx::None, generation };
            self.persist();
        }
    }

    pub fn toggle_notes(&mut self, cx: &mut Context<Self>) {
        if self.input_blocked() {
            return;
        }
        self.notes_mode = !self.notes_mode;
        self.action_generation[2] = self.next_gen();
        cx.notify();
    }

    // --------------------------------------------------------- guided hints

    pub fn hint(&mut self, cx: &mut Context<Self>) {
        if self.input_blocked() {
            return;
        }
        self.action_generation[3] = self.next_gen();
        if self.game.hints_left == 0 {
            self.no_hint_generation = self.next_gen();
            cx.notify();
            return;
        }
        let Some(hint) = find_hint(&self.game, self.selected) else { return };
        if !self.game.use_hint() {
            return;
        }
        let board = self.game.known_board();
        let mut seen = Notes::EMPTY;
        for &p in &PEERS[hint.cell] {
            let d = board.0[p as usize];
            if d != 0 {
                seen.insert(d);
            }
        }
        self.selected = Some(hint.cell);
        self.selection_generation = self.next_gen();
        let generation = self.next_gen();
        self.hint = Some(HintSession { hint, step: 0, generation, seen });
        self.persist();
        cx.notify();
    }

    pub fn hint_step(&mut self, delta: i32, cx: &mut Context<Self>) {
        let generation = self.next_gen();
        if let Some(h) = self.hint.as_mut() {
            let next = h.step as i32 + delta;
            if next >= HINT_STEPS as i32 {
                self.hint_apply(cx);
                return;
            }
            h.step = next.max(0) as usize;
            h.generation = generation;
        }
        cx.notify();
    }

    pub fn hint_apply(&mut self, cx: &mut Context<Self>) {
        let Some(session) = self.hint.take() else { return };
        let i = session.hint.cell;
        match session.hint.kind {
            HintKind::Mistake { .. } => self.erase_cell(i),
            _ => {
                let out = self.game.reveal(i);
                self.apply(out);
                let generation = self.next_gen();
                self.content_fx[i] = Fx { kind: ContentFx::Hint, generation };
                self.flash_fx[i] = Fx { kind: FlashFx::Hint, generation };
            }
        }
        self.selected = Some(i);
        cx.notify();
    }

    pub fn hint_close(&mut self, cx: &mut Context<Self>) {
        self.hint = None;
        cx.notify();
    }

    pub fn second_chance(&mut self, cx: &mut Context<Self>) {
        self.game.second_chance();
        self.mistake_generation = self.next_gen();
        self.persist();
        cx.notify();
    }

    fn debug_fill(&mut self, cx: &mut Context<Self>) {
        let open: Vec<usize> = (0..81).filter(|&i| !self.game.is_locked(i)).collect();
        for &i in open.iter().skip(1) {
            let d = self.game.solution(i);
            self.game.place(i, d);
        }
        self.selected = open.first().copied();
        self.selection_generation = self.next_gen();
        cx.notify();
    }

    /// Translates a core [`MoveOutcome`] into animations, stats and a save.
    fn apply(&mut self, out: MoveOutcome) {
        let generation = self.next_gen();
        match out {
            MoveOutcome::Ignored => return,
            MoveOutcome::Placed { idx, digit, correct, completed, digit_done, cleared_notes, points, status } => {
                if correct {
                    self.content_fx[idx] = Fx { kind: ContentFx::Pop, generation };
                } else {
                    self.content_fx[idx] = Fx { kind: ContentFx::Wrong, generation };
                    self.flash_fx[idx] = Fx { kind: FlashFx::Wrong, generation };
                    self.mistake_generation = generation;
                }
                for c in cleared_notes {
                    self.ghosts[c] = Ghost { digit, value: false, generation };
                }
                for unit in completed {
                    let cells = unit.cells();
                    let origin = cells.iter().position(|&c| c as usize == idx).unwrap_or(0) as i64;
                    for (k, &c) in cells.iter().enumerate() {
                        let delay_ms = (k as i64 - origin).unsigned_abs() * 55;
                        self.flash_fx[c as usize] = Fx { kind: FlashFx::Wave { delay_ms }, generation };
                    }
                }
                if points > 0 {
                    self.score_from = self.game.score - points;
                    self.score_generation = generation;
                    self.floating = Some(FloatingPoints { idx, points, generation });
                    if self.game.streak % 4 == 0 {
                        self.combo_generation = generation;
                    }
                    let s = self.game.streak;
                    if status == GameStatus::Playing && (s == 3 || s == 5 || (s >= 8 && s % 4 == 0)) {
                        self.streak_burst = Some((s, generation));
                    }
                }
                if digit_done {
                    self.pad_generation[digit as usize] = generation;
                }
                match status {
                    GameStatus::Won => {
                        self.end_generation = generation;
                        let (r0, c0) = ((idx / 9) as i64, (idx % 9) as i64);
                        for i in 0..81 {
                            let (r, c) = ((i / 9) as i64, (i % 9) as i64);
                            let d = (r - r0).abs().max((c - c0).abs()) as u64;
                            self.flash_fx[i] = Fx { kind: FlashFx::Wave { delay_ms: 250 + d * 70 }, generation };
                        }
                        self.selected = None;
                        let info = self.save.stats.record_win(&self.game);
                        self.new_record = info.best_score || info.best_time;
                        self.has_game = false;
                    }
                    GameStatus::Lost => {
                        self.end_generation = generation;
                        self.save.stats.record_loss();
                    }
                    GameStatus::Playing => {}
                }
            }
            MoveOutcome::NoteToggled { idx, digit, on } => {
                if on {
                    self.note_fx[idx] = NoteFx { digit, generation };
                } else {
                    self.ghosts[idx] = Ghost { digit, value: false, generation };
                }
            }
            MoveOutcome::Erased { idx } => {
                self.content_fx[idx] = Fx { kind: ContentFx::None, generation };
            }
            MoveOutcome::Undone { idx } => {
                self.content_fx[idx] = Fx { kind: ContentFx::Undo, generation };
                if self.selected != Some(idx) {
                    self.selected = Some(idx);
                    self.selection_generation = generation;
                }
            }
        }
        self.persist();
    }

    fn on_key(&mut self, ev: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let ks = &ev.keystroke;
        let secondary = ks.modifiers.secondary(cx);
        let key = ks.key.as_str();

        if self.screen == Screen::Home {
            match key {
                "escape" if self.sheet_open => self.close_sheet(cx),
                "enter" | "space" if !self.sheet_open => self.open_sheet(cx),
                _ => return,
            }
            cx.stop_propagation();
            return;
        }

        if self.hint.is_some() {
            match key {
                "enter" | "right" | "space" => self.hint_step(1, cx),
                "left" => self.hint_step(-1, cx),
                "escape" => self.hint_close(cx),
                _ => return,
            }
            cx.stop_propagation();
            return;
        }

        match key {
            "z" if secondary => self.undo(cx),
            k if k.len() == 1 && (b'1'..=b'9').contains(&k.as_bytes()[0]) && !secondary => {
                self.press_digit(k.as_bytes()[0] - b'0', cx)
            }
            "up" | "w" => self.move_selection(-1, 0, cx),
            "down" | "s" => self.move_selection(1, 0, cx),
            "left" | "a" => self.move_selection(0, -1, cx),
            "right" | "d" => self.move_selection(0, 1, cx),
            "backspace" | "delete" | "0" | "x" => self.erase(cx),
            "n" | "space" => self.toggle_notes(cx),
            "h" => self.hint(cx),
            "p" => self.toggle_pause(cx),
            "f9" if cfg!(debug_assertions) => self.debug_fill(cx),
            // Debug builds only: play the correct digit in the next open cell.
            "f8" if cfg!(debug_assertions) => {
                if let Some(i) = (0..81).find(|&i| !self.game.is_locked(i)) {
                    self.selected = Some(i);
                    let d = self.game.solution(i);
                    let notes = std::mem::replace(&mut self.notes_mode, false);
                    self.press_digit(d, cx);
                    self.notes_mode = notes;
                }
            }
            "escape" => self.toggle_pause(cx),
            _ => return,
        }
        cx.stop_propagation();
    }
}

/// Layout metrics derived from the viewport so the game fits any window,
/// from a phone (with notch / home indicator) to a tall desktop window.
#[derive(Clone, Copy)]
pub struct Metrics {
    pub board: Pixels,
    pub gap: Pixels,
    pub cell: Pixels,
    pub top_inset: Pixels,
    pub bottom_inset: Pixels,
    pub pad_font: f32,
    pub pad_h: Pixels,
    pub actions_h: Pixels,
    pub viewport: gpui::Size<Pixels>,
}

impl Metrics {
    pub fn compute(window: &Window) -> Metrics {
        let viewport = window.viewport_size();
        let (safe_top, safe_bottom) = platform::safe_area();
        let top = if cfg!(target_os = "macos") { 30.0 } else { (safe_top + 4.0).max(10.0) };
        let bottom = (safe_bottom + 4.0).max(12.0);
        let width = f32::from(viewport.width) - 16.0;
        let actions_h = 76.0;
        // Number pad keys: at least ~44pt wide (full 1/9 column) and ~58pt tall,
        // the comfortable touch-target size on phones.
        let pad = |board: f32| {
            let key_w = board / 9.0;
            ((key_w * 0.92).clamp(30.0, 54.0), (key_w * 1.45).clamp(58.0, 80.0))
        };
        let fixed = top + bottom + 48.0 + 64.0 + actions_h + 4.0 * 8.0;
        let (_, pad_h_guess) = pad(width.min(640.0));
        let board = width.min(f32::from(viewport.height) - fixed - pad_h_guess).clamp(260.0, 640.0);
        let (pad_font, pad_h) = pad(board);
        let used = top + bottom + 48.0 + 64.0 + actions_h + pad_h + board;
        let gap = ((f32::from(viewport.height) - used) / 5.0).clamp(8.0, 34.0);
        Metrics {
            board: px(board),
            gap: px(gap),
            cell: px((board - 14.0) / 9.0),
            top_inset: px(top),
            bottom_inset: px(bottom),
            pad_font,
            pad_h: px(pad_h),
            actions_h: px(actions_h),
            viewport,
        }
    }
}

impl Render for SudokuApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = *self.theme();
        let m = Metrics::compute(window);
        if self.screen == Screen::Game {
            self.track_highlights();
        }

        let root = div()
            .id("root")
            .track_focus(&self.focus)
            .key_context("Sudoku")
            .on_key_down(cx.listener(Self::on_key))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.focus.focus(window, cx)))
            .size_full()
            .relative()
            .overflow_hidden()
            .bg(theme.bg)
            .font_family(FONT_FAMILY)
            .text_color(theme.text);

        match self.screen {
            Screen::Home => root.child(self.render_home(&m, cx)),
            Screen::Game => root
                .flex()
                .flex_col()
                .items_center()
                .child(
                    div()
                        .w(m.board)
                        .h_full()
                        .pt(m.top_inset)
                        .pb(m.bottom_inset)
                        .flex()
                        .flex_col()
                        .justify_center()
                        .gap(m.gap)
                        .child(self.render_header(&m, cx))
                        .child(self.render_stats(&m, cx))
                        .child(self.render_board(&m, cx))
                        .child(self.render_controls(&m, cx)),
                )
                .children(self.render_overlays(&m, cx)),
        }
    }
}

/// Fresh seed per game from the clock (works natively and in the browser).
fn seed() -> u64 {
    let nanos = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x5eed);
    nanos ^ (nanos >> 29) ^ 0x9E37_79B9_7F4A_7C15
}
