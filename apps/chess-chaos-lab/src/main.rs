use chess_chaos_ai::{Style, choose, material_balance, perft};
use chess_chaos_core::clock::{Clock, ClockMode};
use chess_chaos_core::{Game, display_target};
use chess_chaos_uci::{Analysis, analyze};
use eframe::egui::{self, Color32, RichText, Vec2};
use shakmaty::{Color, Move, Position, Role, Square, uci::UciMove};
use std::{
    fs,
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Opponent {
    Local,
    Computer(Style),
    External,
    ComputerVsComputer(Style, Style),
}

struct App {
    game: Game,
    opponent: Opponent,
    selected: Option<Square>,
    promotion: Role,
    flipped: bool,
    dark: bool,
    fen_input: String,
    message: String,
    thinking: Option<Receiver<Result<Move, String>>>,
    generation: u64,
    ai_generation: u64,
    show_pgn: bool,
    pgn_input: String,
    autoplay: bool,
    engine_path: String,
    analysis_rx: Option<Receiver<Result<Analysis, String>>>,
    analysis_generation: u64,
    analysis_text: String,
    clock: Option<Clock>,
    clock_minutes: u32,
    increment_seconds: u32,
    clock_mode: ClockMode,
    last_tick: Instant,
    last_clock_save: Instant,
}

impl Default for App {
    fn default() -> Self {
        let (game, mut clock) = load_game().unwrap_or_else(|| (Game::default(), None));
        if let Some(clock) = &mut clock {
            clock.pause();
        }
        let clock_mode = clock.as_ref().map_or(ClockMode::Fischer, Clock::mode);
        let bonus_seconds = clock.as_ref().map_or(5, Clock::bonus_seconds);
        let now = Instant::now();
        Self {
            fen_input: game.fen(),
            game,
            opponent: Opponent::Local,
            selected: None,
            promotion: Role::Queen,
            flipped: false,
            dark: true,
            message: String::new(),
            thinking: None,
            generation: 0,
            ai_generation: 0,
            show_pgn: false,
            pgn_input: String::new(),
            autoplay: false,
            engine_path: String::new(),
            analysis_rx: None,
            analysis_generation: 0,
            analysis_text: String::new(),
            clock,
            clock_minutes: 10,
            increment_seconds: bonus_seconds,
            clock_mode,
            last_tick: now,
            last_clock_save: now,
        }
    }
}

impl App {
    fn reset(&mut self, game: Game) {
        self.game = game;
        self.clock = None;
        self.last_tick = Instant::now();
        self.selected = None;
        self.thinking = None;
        self.analysis_rx = None;
        self.analysis_text.clear();
        self.generation = self.generation.wrapping_add(1);
        self.fen_input = self.game.fen();
        self.message.clear();
        self.persist();
    }

    fn persist(&mut self) {
        if let Err(e) = save_game(&self.game, self.clock.as_ref()) {
            self.message = format!("Could not save game: {e}");
        }
    }

    fn human_turn(&self) -> bool {
        match self.opponent {
            Opponent::Local => true,
            Opponent::Computer(_) => self.game.position().turn() == Color::White,
            Opponent::External => self.game.position().turn() == Color::White,
            Opponent::ComputerVsComputer(_, _) => false,
        }
    }

    fn clock_paused(&self) -> bool {
        self.clock.as_ref().is_some_and(|clock| !clock.running()) && !self.game.is_over()
    }

    fn click_square(&mut self, sq: Square) {
        if self.game.is_over() || self.clock_paused() || !self.human_turn() {
            return;
        }
        if let Some(from) = self.selected
            && let Some(m) = self
                .game
                .move_from_to(from, sq, Some(self.promotion))
                .or_else(|| self.game.move_from_to(from, sq, None))
            && self.game.play(m).is_ok()
        {
            self.after_play();
            return;
        }
        self.selected = if self
            .game
            .position()
            .board()
            .piece_at(sq)
            .is_some_and(|p| p.color == self.game.position().turn())
        {
            Some(sq)
        } else {
            None
        };
    }

    fn after_move(&mut self) {
        self.selected = None;
        self.fen_input = self.game.fen();
        self.generation = self.generation.wrapping_add(1);
        self.analysis_text.clear();
        self.persist();
    }

    fn after_play(&mut self) {
        if let Some(clock) = &mut self.clock {
            clock.moved(!self.game.position().turn());
            if self.game.is_over() {
                clock.pause();
            }
        }
        self.after_move();
    }

    fn poll_clock(&mut self, ctx: &egui::Context) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_tick).as_millis() as u64;
        self.last_tick = now;
        if let Some(clock) = &mut self.clock
            && clock.running()
            && !self.game.is_over()
        {
            if let Some(loser) = clock.tick(elapsed) {
                self.game.forfeit_on_time(loser);
                self.persist();
            } else if now.duration_since(self.last_clock_save) >= Duration::from_secs(1) {
                self.persist();
                self.last_clock_save = now;
            }
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }

    fn poll_ai(&mut self, ctx: &egui::Context) {
        if let Some(rx) = &self.analysis_rx
            && let Ok(result) = rx.try_recv()
        {
            self.analysis_rx = None;
            if self.analysis_generation == self.generation {
                self.analysis_text = match result {
                    Ok(a) => {
                        let depth = a.depth.map_or_else(|| "—".into(), |v| v.to_string());
                        let score = a
                            .score_cp
                            .map_or_else(|| "—".into(), |v| format!("{v} cp (side to move)"));
                        let nodes = a.nodes.map_or_else(|| "—".into(), |v| v.to_string());
                        let pv = if a.pv.is_empty() {
                            "—".into()
                        } else {
                            a.pv.join(" ")
                        };
                        format!(
                            "{}: {} · depth {depth} · score {score} · {nodes} nodes · PV {pv}",
                            a.engine, a.best_move
                        )
                    }
                    Err(e) => e,
                };
            }
        }
        if let Some(rx) = &self.thinking
            && let Ok(result) = rx.try_recv()
        {
            self.thinking = None;
            if self.ai_generation == self.generation {
                match result {
                    Ok(m) => {
                        if self.game.play(m).is_ok() {
                            self.after_play();
                        } else {
                            self.message = "Engine returned an illegal move".into();
                            self.opponent = Opponent::Local;
                        }
                    }
                    Err(error) => {
                        self.message = error;
                        self.opponent = Opponent::Local;
                    }
                }
            }
        }
        if self.thinking.is_none()
            && !self.game.is_over()
            && !self.clock_paused()
            && !self.human_turn()
            && (self.autoplay || !matches!(self.opponent, Opponent::ComputerVsComputer(_, _)))
        {
            let opponent = self.opponent;
            let pos = self.game.position().clone();
            let fen = self.game.fen();
            let engine_path = PathBuf::from(self.engine_path.trim());
            let seed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |t| t.as_nanos() as u64);
            let (tx, rx) = mpsc::channel();
            self.ai_generation = self.generation;
            self.thinking = Some(rx);
            thread::spawn(move || {
                let result = match opponent {
                    Opponent::Computer(style) => {
                        choose(&pos, style, seed).ok_or("No legal move".into())
                    }
                    Opponent::ComputerVsComputer(white, black) => {
                        let style = if pos.turn() == Color::White {
                            white
                        } else {
                            black
                        };
                        choose(&pos, style, seed).ok_or("No legal move".into())
                    }
                    Opponent::External => analyze(&engine_path, &fen, 1000).and_then(|analysis| {
                        let uci: UciMove = analysis
                            .best_move
                            .parse()
                            .map_err(|e| format!("Invalid engine move: {e}"))?;
                        uci.to_move(&pos)
                            .map_err(|e| format!("Illegal engine move: {e}"))
                    }),
                    Opponent::Local => return,
                };
                let _ = tx.send(result);
            });
        }
        if self.thinking.is_some() || self.analysis_rx.is_some() || self.autoplay {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    fn board(&mut self, ui: &mut egui::Ui) {
        let window = ui.ctx().content_rect();
        let cell = ((window.width() - 370.0).min(window.height() - 130.0) / 8.0).clamp(32.0, 74.0);
        ui.set_width(cell * 8.0 + 30.0);
        let legal = self
            .selected
            .map(|sq| {
                self.game
                    .legal_moves()
                    .into_iter()
                    .filter(|m| m.from() == Some(sq))
                    .map(display_target)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for visual_rank in 0..8 {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label(format!(
                    "{}  ",
                    if self.flipped {
                        visual_rank + 1
                    } else {
                        8 - visual_rank
                    }
                ));
                for visual_file in 0..8 {
                    let file = if self.flipped {
                        7 - visual_file
                    } else {
                        visual_file
                    };
                    let rank = if self.flipped {
                        visual_rank
                    } else {
                        7 - visual_rank
                    };
                    let sq = Square::new((rank * 8 + file) as u32);
                    let piece = self.game.position().board().piece_at(sq);
                    let label = piece
                        .map(|p| {
                            let role = match p.role {
                                Role::King => "K",
                                Role::Queen => "Q",
                                Role::Rook => "R",
                                Role::Bishop => "B",
                                Role::Knight => "N",
                                Role::Pawn => "P",
                            };
                            format!(
                                "{}{}",
                                if p.color == Color::White { "W" } else { "B" },
                                role
                            )
                        })
                        .unwrap_or_default();
                    let base = if (rank + file) % 2 == 0 {
                        Color32::from_rgb(234, 224, 205)
                    } else {
                        Color32::from_rgb(160, 184, 141)
                    };
                    let fill = if self.selected == Some(sq) {
                        Color32::from_rgb(224, 178, 82)
                    } else if legal.contains(&sq) {
                        Color32::from_rgb(143, 181, 204)
                    } else {
                        base
                    };
                    let text = RichText::new(label).size(cell * 0.36).strong().color(
                        match piece.map(|p| p.color) {
                            Some(Color::White) => Color32::from_rgb(29, 58, 80),
                            Some(Color::Black) => Color32::from_rgb(76, 35, 43),
                            None => Color32::TRANSPARENT,
                        },
                    );
                    let button = egui::Button::new(text)
                        .fill(fill)
                        .min_size(Vec2::splat(cell));
                    if ui.add(button).on_hover_text(sq.to_string()).clicked() {
                        self.click_square(sq);
                    }
                }
            });
            ui.add_space(0.0);
        }
        ui.horizontal(|ui| {
            ui.add_space(23.0);
            for file in 0..8 {
                ui.add_sized(
                    [cell, 16.0],
                    egui::Label::new(format!(
                        "{}",
                        (b'a' + if self.flipped { 7 - file } else { file }) as char
                    )),
                );
            }
        });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_clock(&ctx);
        self.poll_ai(&ctx);
        let visuals = if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        ctx.set_visuals(visuals);
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("CHESS CHAOS LAB");
                ui.separator();
                if ui.button("New game").clicked() {
                    self.reset(Game::default());
                }
                if ui
                    .add_enabled(self.clock.is_none(), egui::Button::new("Undo"))
                    .clicked()
                    && self.game.undo()
                {
                    if matches!(self.opponent, Opponent::Computer(_) | Opponent::External)
                        && self.game.position().turn() == Color::Black
                    {
                        self.game.undo();
                    }
                    self.after_move();
                }
                if ui
                    .add_enabled(self.clock.is_none(), egui::Button::new("Redo"))
                    .clicked()
                    && self.game.redo()
                {
                    self.after_move();
                }
                if ui.button("Flip board").clicked() {
                    self.flipped = !self.flipped;
                }
                if ui
                    .button(if self.dark {
                        "Light theme"
                    } else {
                        "Dark theme"
                    })
                    .clicked()
                {
                    self.dark = !self.dark;
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    self.board(ui);
                });
                ui.separator();
                ui.vertical(|ui| {
                    ui.set_width(320.0);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.heading("Game");
                            ui.label(self.game.status());
                            if let Some(clock) = &self.clock {
                                ui.label(format!(
                                    "White {}  ·  Black {}",
                                    format_clock(clock.remaining_ms(Color::White)),
                                    format_clock(clock.remaining_ms(Color::Black))
                                ));
                            }
                            ui.horizontal(|ui| {
                                ui.label("Minutes");
                                ui.add(
                                    egui::DragValue::new(&mut self.clock_minutes).range(1..=180),
                                );
                                if self.clock_mode != ClockMode::Hourglass {
                                    ui.label(if self.clock_mode == ClockMode::Fischer {
                                        "Increment (s)"
                                    } else {
                                        "Delay (s)"
                                    });
                                    ui.add(
                                        egui::DragValue::new(&mut self.increment_seconds)
                                            .range(0..=60),
                                    );
                                }
                            });
                            ui.horizontal(|ui| {
                                ui.label("Clock mode");
                                egui::ComboBox::from_id_salt("clock_mode")
                                    .selected_text(match self.clock_mode {
                                        ClockMode::Fischer => "Fischer increment",
                                        ClockMode::Bronstein => "Bronstein delay",
                                        ClockMode::Hourglass => "Hourglass",
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut self.clock_mode,
                                            ClockMode::Fischer,
                                            "Fischer increment",
                                        );
                                        ui.selectable_value(
                                            &mut self.clock_mode,
                                            ClockMode::Bronstein,
                                            "Bronstein delay",
                                        );
                                        ui.selectable_value(
                                            &mut self.clock_mode,
                                            ClockMode::Hourglass,
                                            "Hourglass",
                                        );
                                    });
                            });
                            if ui.button("New timed game").clicked() {
                                self.reset(Game::default());
                                self.clock = Clock::new_with_mode(
                                    self.clock_minutes,
                                    self.increment_seconds,
                                    self.clock_mode,
                                )
                                .ok();
                                self.last_tick = Instant::now();
                                self.persist();
                            }
                            if let Some(clock) = &mut self.clock {
                                let paused = !clock.running();
                                if ui
                                    .add_enabled(
                                        !self.game.is_over(),
                                        egui::Button::new(if paused {
                                            "Resume clock"
                                        } else {
                                            "Pause clock"
                                        }),
                                    )
                                    .clicked()
                                {
                                    if paused {
                                        clock.resume();
                                    } else {
                                        clock.pause();
                                    }
                                    self.last_tick = Instant::now();
                                    self.persist();
                                }
                            }
                            if self.thinking.is_some() {
                                ui.label("Computer thinking…");
                            }
                            ui.add_space(10.0);
                            let previous_opponent = self.opponent;
                            egui::ComboBox::from_label("Opponent")
                                .selected_text(match self.opponent {
                                    Opponent::Local => "Two players",
                                    Opponent::Computer(s) => s.name(),
                                    Opponent::External => "Local UCI engine",
                                    Opponent::ComputerVsComputer(_, _) => "Computer vs computer",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.opponent,
                                        Opponent::Local,
                                        "Two players",
                                    );
                                    for style in Style::ALL {
                                        ui.selectable_value(
                                            &mut self.opponent,
                                            Opponent::Computer(style),
                                            style.name(),
                                        );
                                    }
                                    ui.selectable_value(
                                        &mut self.opponent,
                                        Opponent::External,
                                        "Local UCI engine",
                                    );
                                    ui.selectable_value(
                                        &mut self.opponent,
                                        Opponent::ComputerVsComputer(
                                            Style::Material,
                                            Style::Search,
                                        ),
                                        "Computer vs computer",
                                    );
                                });
                            if self.opponent != previous_opponent {
                                self.thinking = None;
                                self.generation = self.generation.wrapping_add(1);
                            }
                            if matches!(self.opponent, Opponent::External) {
                                ui.label("Engine executable path");
                                ui.text_edit_singleline(&mut self.engine_path);
                            }
                            if matches!(self.opponent, Opponent::ComputerVsComputer(_, _)) {
                                ui.checkbox(&mut self.autoplay, "Autoplay");
                            }
                            ui.horizontal(|ui| {
                                ui.label("Promote to");
                                for (role, name) in [
                                    (Role::Queen, "Q"),
                                    (Role::Rook, "R"),
                                    (Role::Bishop, "B"),
                                    (Role::Knight, "N"),
                                ] {
                                    ui.selectable_value(&mut self.promotion, role, name);
                                }
                            });
                            if ui
                                .add_enabled(
                                    !matches!(self.opponent, Opponent::ComputerVsComputer(_, _)),
                                    egui::Button::new("Resign"),
                                )
                                .clicked()
                            {
                                let resigning_side = if matches!(
                                    self.opponent,
                                    Opponent::Computer(_) | Opponent::External
                                ) {
                                    Color::White
                                } else {
                                    self.game.position().turn()
                                };
                                self.game.resign(resigning_side);
                                if let Some(clock) = &mut self.clock {
                                    clock.pause();
                                }
                                self.persist();
                            }
                            if self.game.can_claim_draw() && ui.button("Claim draw").clicked() {
                                self.game.claim_draw();
                                if let Some(clock) = &mut self.clock {
                                    clock.pause();
                                }
                                self.persist();
                            }
                            ui.separator();
                            ui.heading("Moves");
                            egui::ScrollArea::vertical()
                                .max_height(200.0)
                                .show(ui, |ui| {
                                    for (i, pair) in self.game.san_history().chunks(2).enumerate() {
                                        ui.label(format!(
                                            "{}. {} {}",
                                            i + 1,
                                            pair[0],
                                            pair.get(1).map_or("", String::as_str)
                                        ));
                                    }
                                });
                            ui.separator();
                            ui.heading("Position & export");
                            ui.label("FEN");
                            ui.text_edit_multiline(&mut self.fen_input);
                            if ui.button("Load FEN").clicked() {
                                match Game::from_fen(&self.fen_input) {
                                    Ok(g) => self.reset(g),
                                    Err(e) => self.message = e,
                                }
                            }
                            if ui.button("Copy FEN").clicked() {
                                ui.ctx().copy_text(self.game.fen());
                            }
                            if ui.button("Copy PGN").clicked() {
                                ui.ctx().copy_text(self.game.pgn());
                            }
                            if ui.button("Show PGN").clicked() {
                                self.show_pgn = !self.show_pgn;
                            }
                            if self.show_pgn {
                                ui.label(self.game.pgn());
                            }
                            ui.separator();
                            ui.label("Paste plain PGN mainline");
                            ui.text_edit_multiline(&mut self.pgn_input);
                            if ui.button("Load PGN").clicked() {
                                match Game::from_pgn(&self.pgn_input) {
                                    Ok(g) => self.reset(g),
                                    Err(e) => self.message = e,
                                }
                            }
                            ui.separator();
                            ui.heading("Developer lab");
                            ui.label(format!(
                                "Material balance: {}",
                                material_balance(self.game.position())
                            ));
                            if ui.button("Perft depth 3").clicked() {
                                self.message =
                                    format!("{} leaf nodes", perft(self.game.position(), 3));
                            }
                            ui.separator();
                            ui.heading("Local UCI analysis");
                            ui.label("Engine executable path");
                            ui.text_edit_singleline(&mut self.engine_path);
                            if ui
                                .add_enabled(
                                    self.analysis_rx.is_none(),
                                    egui::Button::new("Analyze (1 second)"),
                                )
                                .clicked()
                            {
                                let path = PathBuf::from(self.engine_path.trim());
                                let fen = self.game.fen();
                                let (tx, rx) = mpsc::channel();
                                self.analysis_generation = self.generation;
                                self.analysis_rx = Some(rx);
                                self.analysis_text = "Analyzing…".into();
                                thread::spawn(move || {
                                    let _ = tx.send(analyze(&path, &fen, 1000));
                                });
                            }
                            if !self.analysis_text.is_empty() {
                                ui.label(&self.analysis_text);
                            }
                            if !self.message.is_empty() {
                                ui.separator();
                                ui.label(&self.message);
                            }
                        });
                    });
                });
            });
        });
    }
}

fn save_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("XDG_DATA_HOME").map(PathBuf::from))
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    base.join("ChessChaosLab").join("autosave.txt")
}

fn save_game(game: &Game, clock: Option<&Clock>) -> std::io::Result<()> {
    let path = save_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension("tmp");
    let mut text = game.save_text();
    if let Some(clock) = clock {
        text.push_str(&format!("{}\n", clock.save_text()));
    }
    fs::write(&temp, text)?;
    fs::rename(temp, path)
}

fn load_game() -> Option<(Game, Option<Clock>)> {
    let path = save_path();
    if fs::metadata(&path).ok()?.len() > 1_000_000 {
        return None;
    }
    let text = fs::read_to_string(path).ok()?;
    let game = Game::from_save_text(&text).ok()?;
    let clock = text
        .lines()
        .nth(3)
        .and_then(|line| Clock::from_save_text(line).ok())
        .filter(|clock| clock.active() == game.position().turn());
    Some((game, clock))
}

fn format_clock(milliseconds: u64) -> String {
    let seconds = milliseconds.div_ceil(1_000);
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 750.0])
            .with_min_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Chess Chaos Lab",
        options,
        Box::new(|_| Ok(Box::new(App::default()))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn internal_ai_match_plays_legal_moves() {
        let mut game = Game::default();
        for ply in 0..100 {
            if game.is_over() {
                break;
            }
            let style = if ply % 2 == 0 {
                Style::Random
            } else {
                Style::CaptureHunter
            };
            let selected = choose(game.position(), style, ply).unwrap();
            game.play(selected).unwrap();
        }
        assert!(game.ply() > 20);
    }
}
