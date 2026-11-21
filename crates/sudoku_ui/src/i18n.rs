//! Translations. Add a language by adding a `Lang` variant and a `Strings` table.

use serde::{Deserialize, Serialize};
use sudoku_core::{Difficulty, Digit, Hint, HintKind, Notes, Unit};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub enum Lang {
    #[default]
    Es,
    En,
}

impl Lang {
    pub const ALL: [Lang; 2] = [Lang::Es, Lang::En];

    pub fn t(self) -> &'static Strings {
        match self {
            Lang::Es => &ES,
            Lang::En => &EN,
        }
    }

    pub fn native_name(self) -> &'static str {
        match self {
            Lang::Es => "Español",
            Lang::En => "English",
        }
    }

    /// Best guess from a BCP-47 tag such as `es-ES` or `en-US`.
    pub fn from_tag(tag: &str) -> Lang {
        if tag.to_ascii_lowercase().starts_with("en") { Lang::En } else { Lang::Es }
    }

    pub fn difficulty(self, d: Difficulty) -> &'static str {
        let t = self.t();
        match d {
            Difficulty::Easy => t.easy,
            Difficulty::Medium => t.medium,
            Difficulty::Hard => t.hard,
            Difficulty::Expert => t.expert,
            Difficulty::Extreme => t.extreme,
        }
    }

    pub fn difficulty_blurb(self, d: Difficulty) -> &'static str {
        let t = self.t();
        match d {
            Difficulty::Easy => t.easy_blurb,
            Difficulty::Medium => t.medium_blurb,
            Difficulty::Hard => t.hard_blurb,
            Difficulty::Expert => t.expert_blurb,
            Difficulty::Extreme => t.extreme_blurb,
        }
    }

    /// "la fila 3" / "row 3".
    fn unit(self, u: Unit) -> String {
        match (self, u) {
            (Lang::Es, Unit::Row(r)) => format!("la fila {}", r + 1),
            (Lang::Es, Unit::Col(c)) => format!("la columna {}", c + 1),
            (Lang::Es, Unit::Box(b)) => format!("el bloque {}", b + 1),
            (Lang::En, Unit::Row(r)) => format!("row {}", r + 1),
            (Lang::En, Unit::Col(c)) => format!("column {}", c + 1),
            (Lang::En, Unit::Box(b)) => format!("box {}", b + 1),
        }
    }

    /// "de la fila 3" / "del bloque 2" / "in row 3".
    fn of_unit(self, u: Unit) -> String {
        match (self, u) {
            (Lang::Es, Unit::Box(b)) => format!("del bloque {}", b + 1),
            (Lang::Es, u) => format!("de {}", self.unit(u)),
            (Lang::En, u) => format!("in {}", self.unit(u)),
        }
    }

    fn list(self, digits: impl Iterator<Item = Digit>) -> String {
        let v: Vec<String> = digits.map(|d| d.to_string()).collect();
        match v.len() {
            0 => String::new(),
            1 => v[0].clone(),
            n => {
                let and = if self == Lang::Es { "y" } else { "and" };
                format!("{} {and} {}", v[..n - 1].join(", "), v[n - 1])
            }
        }
    }

    pub fn hint_title(self, h: &Hint) -> &'static str {
        let t = self.t();
        match h.kind {
            HintKind::Mistake { .. } => t.hint_mistake,
            HintKind::FullHouse { .. } => t.hint_full_house,
            HintKind::HiddenSingle { .. } => t.hint_hidden_single,
            HintKind::NakedSingle => t.hint_naked_single,
            HintKind::Reveal { .. } => t.hint_reveal,
        }
    }

    /// Text for each of the 3 guided steps: look → why → answer.
    pub fn hint_step(self, h: &Hint, step: usize, seen: Notes) -> String {
        let d = h.digit;
        let es = self == Lang::Es;
        match (&h.kind, step) {
            (HintKind::Mistake { .. }, 0) => if es {
                "Revisa la casilla marcada: el número que has puesto no es correcto.".into()
            } else {
                "Check the highlighted cell: the number you entered is not correct.".into()
            },
            (HintKind::Mistake { wrong }, 1) => match (es, h.blockers.is_empty()) {
                (true, false) => format!("Ese {wrong} choca con otro {wrong} de su fila, columna o bloque."),
                (true, true) => format!("El {wrong} no choca a simple vista, pero no lleva a la solución."),
                (false, false) => format!("That {wrong} clashes with another {wrong} in its row, column or box."),
                (false, true) => format!("The {wrong} doesn't clash directly, but it doesn't lead to the solution."),
            },
            (HintKind::Mistake { .. }, _) => if es {
                "Bórralo y vuelve a pensar qué número encaja.".into()
            } else {
                "Erase it and think again about which number fits.".into()
            },

            (HintKind::FullHouse { unit }, 0) => if es {
                format!("Mira {}: solo le queda una casilla libre.", self.unit(*unit))
            } else {
                format!("Look at {}: it has a single empty cell left.", self.unit(*unit))
            },
            (HintKind::FullHouse { .. }, 1) => if es {
                format!("Cada fila, columna y bloque lleva del 1 al 9 sin repetir. Aquí solo falta el {d}.")
            } else {
                format!("Every row, column and box holds 1 to 9 exactly once. Only the {d} is missing here.")
            },

            (HintKind::HiddenSingle { unit }, 0) => if es {
                format!("Busca dónde puede ir el {d} {}. Solo hay un sitio posible.", self.of_unit(*unit))
            } else {
                format!("Find where the {d} can go {}. There is only one possible spot.", self.of_unit(*unit))
            },
            (HintKind::HiddenSingle { unit }, 1) => if es {
                format!(
                    "Los {d} resaltados ya ocupan esas filas, columnas o bloques, así que ninguna otra casilla libre {} puede ser un {d}.",
                    self.of_unit(*unit)
                )
            } else {
                format!(
                    "The highlighted {d}s already cover those rows, columns or boxes, so no other empty cell {} can be a {d}.",
                    self.of_unit(*unit)
                )
            },

            (HintKind::NakedSingle, 0) => if es {
                "Mira la casilla marcada y todo lo que ve: su fila, su columna y su bloque.".into()
            } else {
                "Look at the highlighted cell and everything it sees: its row, column and box.".into()
            },
            (HintKind::NakedSingle, 1) => if es {
                format!("Ahí ya aparecen {}. El único número que falta es el {d}.", self.list(seen.iter()))
            } else {
                format!("You can already see {} there. The only number missing is {d}.", self.list(seen.iter()))
            },

            (HintKind::Reveal { candidates }, 0) => if es {
                format!(
                    "Ahora no hay un paso sencillo. Esta es la casilla más restringida: puede ser {}.",
                    self.list(candidates.iter())
                )
            } else {
                format!(
                    "There's no simple step right now. This is the most constrained cell: it can be {}.",
                    self.list(candidates.iter())
                )
            },
            (HintKind::Reveal { .. }, 1) => if es {
                format!("Con técnicas avanzadas (parejas, cadenas…) se descartan las demás opciones y queda el {d}.")
            } else {
                format!("Advanced techniques (pairs, chains…) rule out the other options, leaving {d}.")
            },

            (_, _) => if es {
                format!("Por tanto, la casilla marcada es un {d}.")
            } else {
                format!("So the highlighted cell is a {d}.")
            },
        }
    }

    /// Escalating cheer for streak milestones.
    pub fn streak_word(self, streak: u32) -> &'static str {
        let ix = match streak {
            0..=3 => 0,
            4..=5 => 1,
            6..=11 => 2,
            12..=19 => 3,
            _ => 4,
        };
        match self {
            Lang::Es => ["¡Bien!", "¡Genial!", "¡Increíble!", "¡Imparable!", "¡Leyenda!"][ix],
            Lang::En => ["Nice!", "Great!", "Amazing!", "Unstoppable!", "Legendary!"][ix],
        }
    }

    pub fn streak_label(self, streak: u32) -> String {
        match self {
            Lang::Es => format!("{streak} seguidas"),
            Lang::En => format!("{streak} in a row"),
        }
    }

    pub fn place_digit(self, d: Digit) -> String {
        match self {
            Lang::Es => format!("Colocar {d}"),
            Lang::En => format!("Place {d}"),
        }
    }

    pub fn step_of(self, step: usize, total: usize) -> String {
        match self {
            Lang::Es => format!("Paso {} de {}", step + 1, total),
            Lang::En => format!("Step {} of {}", step + 1, total),
        }
    }
}

pub struct Strings {
    pub tagline: &'static str,
    pub play: &'static str,
    pub continue_game: &'static str,
    pub choose_difficulty: &'static str,
    pub tab_play: &'static str,
    pub tab_records: &'static str,
    pub tab_settings: &'static str,
    pub easy: &'static str,
    pub medium: &'static str,
    pub hard: &'static str,
    pub expert: &'static str,
    pub extreme: &'static str,
    pub easy_blurb: &'static str,
    pub medium_blurb: &'static str,
    pub hard_blurb: &'static str,
    pub expert_blurb: &'static str,
    pub extreme_blurb: &'static str,
    pub best: &'static str,
    pub combo: &'static str,
    pub difficulty: &'static str,
    pub mistakes: &'static str,
    pub time: &'static str,
    pub undo: &'static str,
    pub erase: &'static str,
    pub notes: &'static str,
    pub hint: &'static str,
    pub paused: &'static str,
    pub excellent: &'static str,
    pub solved: &'static str,
    pub score: &'static str,
    pub new_game: &'static str,
    pub home: &'static str,
    pub game_over: &'static str,
    pub game_over_body: &'static str,
    pub second_chance: &'static str,
    pub new_record: &'static str,
    pub records_title: &'static str,
    pub played: &'static str,
    pub won: &'static str,
    pub win_rate: &'static str,
    pub best_time: &'static str,
    pub avg_time: &'static str,
    pub best_score: &'static str,
    pub top_games: &'static str,
    pub no_records: &'static str,
    pub wins: &'static str,
    pub streak: &'static str,
    pub settings_title: &'static str,
    pub language: &'static str,
    pub theme: &'static str,
    pub gameplay: &'static str,
    pub mistake_limit: &'static str,
    pub mistake_limit_sub: &'static str,
    pub show_timer: &'static str,
    pub show_timer_sub: &'static str,
    pub highlight_same: &'static str,
    pub highlight_same_sub: &'static str,
    pub data: &'static str,
    pub reset_stats: &'static str,
    pub reset_confirm: &'static str,
    pub saved_locally: &'static str,
    pub next: &'static str,
    pub back: &'static str,
    pub erase_it: &'static str,
    pub hint_mistake: &'static str,
    pub hint_full_house: &'static str,
    pub hint_hidden_single: &'static str,
    pub hint_naked_single: &'static str,
    pub hint_reveal: &'static str,
    pub no_hints_left: &'static str,
}

pub static ES: Strings = Strings {
    tagline: "Entrena la mente, un número a la vez",
    play: "Jugar",
    continue_game: "Continuar",
    choose_difficulty: "Elige la dificultad",
    tab_play: "Jugar",
    tab_records: "Récords",
    tab_settings: "Ajustes",
    easy: "Fácil",
    medium: "Medio",
    hard: "Difícil",
    expert: "Experto",
    extreme: "Extremo",
    easy_blurb: "Para calentar",
    medium_blurb: "Relajado pero con miga",
    hard_blurb: "Candidatos bloqueados",
    expert_blurb: "Parejas, tríos y más",
    extreme_blurb: "Solo para valientes",
    best: "Mejor",
    combo: "Combo",
    difficulty: "Dificultad",
    mistakes: "Errores",
    time: "Tiempo",
    undo: "Deshacer",
    erase: "Borrar",
    notes: "Notas",
    hint: "Pista",
    paused: "Pausado",
    excellent: "¡Excelente!",
    solved: "Sudoku completado",
    score: "Puntuación",
    new_game: "Nueva partida",
    home: "Inicio",
    game_over: "Fin de la partida",
    game_over_body: "Has cometido 3 errores. ¿Quieres seguir con una última oportunidad?",
    second_chance: "Segunda oportunidad",
    new_record: "¡Nuevo récord!",
    records_title: "Récords",
    played: "Jugadas",
    won: "Ganadas",
    win_rate: "Victorias",
    best_time: "Mejor tiempo",
    avg_time: "Tiempo medio",
    best_score: "Mejor puntuación",
    top_games: "Mejores partidas",
    no_records: "Aún no hay partidas ganadas en este nivel. ¡A por la primera!",
    wins: "Ganadas",
    streak: "Racha",
    settings_title: "Ajustes",
    language: "Idioma",
    theme: "Tema",
    gameplay: "Juego",
    mistake_limit: "Límite de errores",
    mistake_limit_sub: "La partida termina al tercer error",
    show_timer: "Mostrar tiempo",
    show_timer_sub: "Cronómetro durante la partida",
    highlight_same: "Resaltar iguales",
    highlight_same_sub: "Marca los números iguales al seleccionado",
    data: "Datos",
    reset_stats: "Borrar estadísticas",
    reset_confirm: "Toca otra vez para confirmar",
    saved_locally: "Todo se guarda en este dispositivo, sin servidores.",
    next: "Siguiente",
    back: "Atrás",
    erase_it: "Borrar número",
    hint_mistake: "Hay un error",
    hint_full_house: "Última casilla",
    hint_hidden_single: "Único lugar",
    hint_naked_single: "Único candidato",
    hint_reveal: "Paso difícil",
    no_hints_left: "No te quedan pistas",
};

pub static EN: Strings = Strings {
    tagline: "Train your brain, one number at a time",
    play: "Play",
    continue_game: "Continue",
    choose_difficulty: "Choose difficulty",
    tab_play: "Play",
    tab_records: "Records",
    tab_settings: "Settings",
    easy: "Easy",
    medium: "Medium",
    hard: "Hard",
    expert: "Expert",
    extreme: "Extreme",
    easy_blurb: "A gentle warm-up",
    medium_blurb: "Relaxed with a bite",
    hard_blurb: "Locked candidates",
    expert_blurb: "Pairs, triples and more",
    extreme_blurb: "Only for the brave",
    best: "Best",
    combo: "Combo",
    difficulty: "Difficulty",
    mistakes: "Mistakes",
    time: "Time",
    undo: "Undo",
    erase: "Erase",
    notes: "Notes",
    hint: "Hint",
    paused: "Paused",
    excellent: "Excellent!",
    solved: "Sudoku solved",
    score: "Score",
    new_game: "New game",
    home: "Home",
    game_over: "Game over",
    game_over_body: "You made 3 mistakes. Keep going with one last chance?",
    second_chance: "Second chance",
    new_record: "New record!",
    records_title: "Records",
    played: "Played",
    won: "Won",
    win_rate: "Win rate",
    best_time: "Best time",
    avg_time: "Average time",
    best_score: "Best score",
    top_games: "Top games",
    no_records: "No wins at this level yet. Go get the first one!",
    wins: "Wins",
    streak: "Streak",
    settings_title: "Settings",
    language: "Language",
    theme: "Theme",
    gameplay: "Gameplay",
    mistake_limit: "Mistake limit",
    mistake_limit_sub: "The game ends on the third mistake",
    show_timer: "Show timer",
    show_timer_sub: "Stopwatch while playing",
    highlight_same: "Highlight matches",
    highlight_same_sub: "Mark numbers equal to the selected one",
    data: "Data",
    reset_stats: "Reset statistics",
    reset_confirm: "Tap again to confirm",
    saved_locally: "Everything is stored on this device, no servers.",
    next: "Next",
    back: "Back",
    erase_it: "Erase number",
    hint_mistake: "There's a mistake",
    hint_full_house: "Last cell",
    hint_hidden_single: "Only spot",
    hint_naked_single: "Single candidate",
    hint_reveal: "Tough spot",
    no_hints_left: "No hints left",
};
