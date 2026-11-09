use chess_chaos_ai::{Style, choose, material_balance, perft};
use chess_chaos_core::clock::{Clock, ClockMode};
use chess_chaos_core::{Game, display_target, pgn::PgnGame};
use chess_chaos_uci::{Analysis, analyze};
use eframe::egui::{self, Color32, RichText, Vec2};
use shakmaty::{Color, Move, Piece, Position, Role, Square, uci::UciMove};
use std::{
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
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
    pgn_file_path: String,
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
    autosave_blocked: Option<String>,
    focus_after_popup: Option<egui::Id>,
    open_combo: Option<egui::Id>,
}

impl Default for App {
    fn default() -> Self {
        let (game, mut clock, message, autosave_blocked) = match load_game(&save_path()) {
            Ok(Some((game, clock))) => (game, clock, String::new(), None),
            Ok(None) => (Game::default(), None, String::new(), None),
            Err(error) => match preserve_invalid_save(&save_path()) {
                Ok(backup) => (
                    Game::default(),
                    None,
                    format!(
                        "Could not restore autosave ({error}); copy preserved at {}",
                        backup.display()
                    ),
                    None,
                ),
                Err(backup_error) => {
                    let warning = format!(
                        "Could not restore autosave ({error}) or preserve it ({backup_error}); saving is disabled"
                    );
                    (Game::default(), None, warning.clone(), Some(warning))
                }
            },
        };
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
            message,
            thinking: None,
            generation: 0,
            ai_generation: 0,
            show_pgn: false,
            pgn_input: String::new(),
            pgn_file_path: String::new(),
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
            autosave_blocked,
            focus_after_popup: None,
            open_combo: None,
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
        if let Some(warning) = &self.autosave_blocked {
            self.message = warning.clone();
            return;
        }
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
                    let response = ui.add(button);
                    let selected = self.selected == Some(sq);
                    let name = square_accessible_name(sq, piece, selected, legal.contains(&sq));
                    response.widget_info(|| {
                        egui::WidgetInfo::selected(egui::Role::Button, true, selected, &name)
                    });
                    if response.on_hover_text(sq.to_string()).clicked() {
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
        if let Some(combo_id) = self.focus_after_popup.take() {
            ctx.memory_mut(|memory| memory.request_focus(combo_id));
        }
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.focus_after_popup = self.open_combo;
            if self.open_combo.is_some() {
                ctx.request_repaint();
            }
        }
        self.open_combo = None;
        self.poll_clock(&ctx);
        self.poll_ai(&ctx);
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            self.selected = None;
        }
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
                            let status = self.game.status();
                            let response = ui.label(&status);
                            response.widget_info(|| {
                                egui::WidgetInfo::labeled(egui::Role::Status, true, &status)
                            });
                            ctx.accesskit_node_builder(response.id, |node| {
                                node.set_live(egui::accesskit::Live::Polite);
                                node.set_live_atomic();
                            });
                            if let Some(clock) = &self.clock {
                                ui.label(format!(
                                    "White {}  ·  Black {}",
                                    format_clock(clock.remaining_ms(Color::White)),
                                    format_clock(clock.remaining_ms(Color::Black))
                                ));
                            }
                            ui.horizontal(|ui| {
                                let label = ui.label("Minutes");
                                ui.add(
                                    egui::DragValue::new(&mut self.clock_minutes).range(1..=180),
                                )
                                .labelled_by(label.id);
                                if self.clock_mode != ClockMode::Hourglass {
                                    let label =
                                        ui.label(if self.clock_mode == ClockMode::Fischer {
                                            "Increment (s)"
                                        } else {
                                            "Delay (s)"
                                        });
                                    ui.add(
                                        egui::DragValue::new(&mut self.increment_seconds)
                                            .range(0..=60),
                                    )
                                    .labelled_by(label.id);
                                }
                            });
                            ui.horizontal(|ui| {
                                let label = ui.label("Clock mode");
                                let combo_id = combo_box_id(ui, "clock_mode");
                                let combo = egui::ComboBox::from_id_salt("clock_mode")
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
                                combo.response.labelled_by(label.id);
                                if combo.inner.is_some() {
                                    self.open_combo = Some(combo_id);
                                }
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
                            ui.horizontal(|ui| {
                                let combo_id = combo_box_id(ui, "opponent");
                                let combo = egui::ComboBox::from_id_salt("opponent")
                                    .selected_text(match self.opponent {
                                        Opponent::Local => "Two players",
                                        Opponent::Computer(s) => s.name(),
                                        Opponent::External => "Local UCI engine",
                                        Opponent::ComputerVsComputer(_, _) => {
                                            "Computer vs computer"
                                        }
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
                                let label = ui.label("Opponent");
                                combo.response.labelled_by(label.id);
                                if combo.inner.is_some() {
                                    self.open_combo = Some(combo_id);
                                }
                            });
                            if self.opponent != previous_opponent {
                                self.thinking = None;
                                self.generation = self.generation.wrapping_add(1);
                            }
                            if matches!(self.opponent, Opponent::External) {
                                let label = ui.label("Engine executable path");
                                ui.text_edit_singleline(&mut self.engine_path)
                                    .labelled_by(label.id);
                            }
                            if matches!(self.opponent, Opponent::ComputerVsComputer(_, _)) {
                                ui.checkbox(&mut self.autoplay, "Autoplay");
                            }
                            ui.horizontal(|ui| {
                                ui.label("Promote to");
                                for (role, name, accessible_name) in [
                                    (Role::Queen, "Q", "Promote to queen"),
                                    (Role::Rook, "R", "Promote to rook"),
                                    (Role::Bishop, "B", "Promote to bishop"),
                                    (Role::Knight, "N", "Promote to knight"),
                                ] {
                                    let response =
                                        ui.selectable_value(&mut self.promotion, role, name);
                                    response.widget_info(|| {
                                        egui::WidgetInfo::selected(
                                            egui::Role::Button,
                                            true,
                                            self.promotion == role,
                                            accessible_name,
                                        )
                                    });
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
                            let rows = pgn_rows(self.game.pgn_tree());
                            let mut selected_node = None;
                            egui::ScrollArea::vertical().max_height(200.0).show_rows(
                                ui,
                                22.0,
                                rows.len(),
                                |ui, range| {
                                    for row in &rows[range] {
                                        if ui
                                            .add_enabled(
                                                self.clock.is_none(),
                                                egui::Button::new(&row.1),
                                            )
                                            .clicked()
                                        {
                                            selected_node = Some(row.0);
                                        }
                                    }
                                },
                            );
                            if let Some(node) = selected_node {
                                match self.game.goto_pgn_node(node) {
                                    Ok(()) => {
                                        self.opponent = Opponent::Local;
                                        self.autoplay = false;
                                        self.thinking = None;
                                        self.analysis_rx = None;
                                        self.after_move();
                                    }
                                    Err(error) => self.message = error,
                                }
                            }
                            ui.separator();
                            ui.heading("Position & export");
                            let label = ui.label("FEN");
                            ui.text_edit_multiline(&mut self.fen_input)
                                .labelled_by(label.id);
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
                            let label = ui.label("Paste PGN (one game, including variations)");
                            ui.text_edit_multiline(&mut self.pgn_input)
                                .labelled_by(label.id);
                            if ui.button("Load PGN").clicked() {
                                match Game::from_pgn(&self.pgn_input) {
                                    Ok(g) => {
                                        self.opponent = Opponent::Local;
                                        self.autoplay = false;
                                        self.reset(g);
                                    }
                                    Err(e) => self.message = e,
                                }
                            }
                            ui.horizontal(|ui| {
                                ui.label("PGN file path");
                                ui.text_edit_singleline(&mut self.pgn_file_path);
                            });
                            if ui.button("Load PGN file").clicked() {
                                match read_pgn_file(Path::new(self.pgn_file_path.trim()))
                                    .and_then(|text| Game::from_pgn(&text))
                                {
                                    Ok(game) => {
                                        self.opponent = Opponent::Local;
                                        self.autoplay = false;
                                        self.reset(game);
                                    }
                                    Err(error) => self.message = error,
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
                            let label = ui.label("Engine executable path");
                            ui.text_edit_singleline(&mut self.engine_path)
                                .labelled_by(label.id);
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

fn square_accessible_name(
    square: Square,
    piece: Option<Piece>,
    selected: bool,
    legal_destination: bool,
) -> String {
    let contents = piece.map_or("empty".to_owned(), |piece| {
        format!(
            "{} {}",
            if piece.color == Color::White {
                "White"
            } else {
                "Black"
            },
            match piece.role {
                Role::King => "king",
                Role::Queen => "queen",
                Role::Rook => "rook",
                Role::Bishop => "bishop",
                Role::Knight => "knight",
                Role::Pawn => "pawn",
            }
        )
    });
    let state = if selected {
        ", selected"
    } else if legal_destination {
        ", legal destination"
    } else {
        ""
    };
    format!("{square}, {contents}{state}")
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
    if text.len() > 4_000_000 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Save exceeds 4 MB limit",
        ));
    }
    fs::write(&temp, text)?;
    fs::rename(temp, path)
}

fn load_game(path: &Path) -> Result<Option<(Game, Option<Clock>)>, String> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.to_string()),
    };
    if metadata.len() > 4_000_000 {
        return Err("Save exceeds 4 MB limit".into());
    }
    let mut text = String::new();
    fs::File::open(path)
        .map_err(|error| error.to_string())?
        .take(4_000_001)
        .read_to_string(&mut text)
        .map_err(|error| error.to_string())?;
    if text.len() > 4_000_000 {
        return Err("Save exceeds 4 MB limit".into());
    }
    if !(3..=5).contains(&text.lines().count()) {
        return Err("Invalid autosave structure".into());
    }
    let game = Game::from_save_text(&text)?;
    let clock = text
        .lines()
        .skip(3)
        .find(|line| !line.starts_with("pgn:"))
        .map(Clock::from_save_text)
        .transpose()?
        .map(|clock| {
            if clock.active() == game.position().turn() {
                Ok(clock)
            } else {
                Err("Saved clock active side does not match game".to_owned())
            }
        })
        .transpose()?;
    Ok(Some((game, clock)))
}

fn preserve_invalid_save(path: &Path) -> io::Result<PathBuf> {
    if fs::metadata(path)?.len() > 4_000_000 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Autosave exceeds 4 MB; move it aside to resume saving",
        ));
    }
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |time| time.as_nanos());
    let backup = path.with_file_name(format!("autosave-corrupt-{timestamp}.txt"));
    let mut source = fs::File::open(path)?;
    let mut destination = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup)?;
    io::copy(&mut source, &mut destination)?;
    destination.sync_all()?;
    Ok(backup)
}

fn read_pgn_file(path: &Path) -> Result<String, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("PGN file: {error}"))?;
    if metadata.len() > 1_000_000 {
        return Err("PGN exceeds 1 MB limit".into());
    }
    let mut text = String::new();
    fs::File::open(path)
        .map_err(|error| format!("PGN file: {error}"))?
        .take(1_000_001)
        .read_to_string(&mut text)
        .map_err(|error| format!("PGN file: {error}"))?;
    if text.len() > 1_000_000 {
        return Err("PGN exceeds 1 MB limit".into());
    }
    Ok(text)
}

fn pgn_rows(tree: &PgnGame) -> Vec<(usize, String)> {
    let mut rows = Vec::with_capacity(tree.nodes().len().saturating_sub(1));
    let Some(&first) = tree.nodes()[0].children.first() else {
        return rows;
    };
    let mut stack = vec![(first, 0usize, true)];
    while let Some((node, depth, siblings)) = stack.pop() {
        let entry = &tree.nodes()[node];
        let (number, side) = tree.move_number_at(node).expect("move node has a parent");
        let prefix = if side == Color::White {
            format!("{number}.")
        } else {
            format!("{number}...")
        };
        let annotations = entry
            .nags
            .iter()
            .map(|nag| format!(" ${nag}"))
            .collect::<String>();
        rows.push((
            node,
            format!(
                "{}{} {}{} {} {}",
                "  ".repeat(depth.min(16)),
                prefix,
                entry.san,
                annotations,
                entry.before.join(" "),
                entry.after.join(" ")
            ),
        ));
        if let Some(&continuation) = entry.children.first() {
            stack.push((continuation, depth, true));
        }
        if siblings {
            let parent = entry.parent.expect("move node has a parent");
            for &variation in tree.nodes()[parent].children.iter().skip(1).rev() {
                stack.push((variation, depth + 1, false));
            }
        }
    }
    rows
}

fn format_clock(milliseconds: u64) -> String {
    let seconds = milliseconds.div_ceil(1_000);
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

fn combo_box_id(ui: &egui::Ui, salt: &'static str) -> egui::Id {
    ui.make_persistent_id(egui::IdSalt::new(salt))
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 450.0])
            .with_min_inner_size([700.0, 400.0]),
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
    fn popup_focus_targets_the_combo_box_widget() {
        egui::__run_test_ui(|ui| {
            for salt in ["clock_mode", "opponent"] {
                let widget = egui::ComboBox::from_id_salt(salt)
                    .selected_text("choice")
                    .show_ui(ui, |_| {})
                    .response;
                assert_eq!(combo_box_id(ui, salt), widget.id);
            }
        });
    }

    #[test]
    fn accessible_square_names_identify_position_piece_and_move_state() {
        let game = Game::default();
        let e2: Square = "e2".parse().unwrap();
        let e4: Square = "e4".parse().unwrap();
        assert_eq!(
            square_accessible_name(e2, game.position().board().piece_at(e2), true, false),
            "e2, White pawn, selected"
        );
        assert_eq!(
            square_accessible_name(e4, game.position().board().piece_at(e4), false, true),
            "e4, empty, legal destination"
        );
    }

    #[test]
    fn corrupt_clock_is_preserved_before_new_autosave() {
        let path = std::env::temp_dir().join(format!(
            "chess-chaos-m0-test-{}-{}.txt",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let invalid = format!("{}clock broken\n", Game::default().save_text());
        fs::write(&path, &invalid).unwrap();
        assert!(load_game(&path).is_err());
        let backup = preserve_invalid_save(&path).unwrap();
        assert_eq!(fs::read_to_string(&backup).unwrap(), invalid);
        fs::remove_file(path).unwrap();
        fs::remove_file(backup).unwrap();
    }

    #[test]
    fn truncated_or_oversized_autosave_is_not_loaded_or_copied() {
        let path = std::env::temp_dir().join(format!(
            "chess-chaos-m0-bounds-{}-{}.txt",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::write(&path, Game::default().fen()).unwrap();
        assert!(load_game(&path).is_err());
        fs::write(&path, vec![b'x'; 4_000_001]).unwrap();
        assert!(load_game(&path).is_err());
        assert!(preserve_invalid_save(&path).is_err());
        fs::remove_file(path).unwrap();
    }

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

    #[test]
    fn pgn_file_import_and_autosave_restore_variations() {
        let path = std::env::temp_dir().join(format!(
            "chess-chaos-pgn-test-{}-{}.txt",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let source = "[Event \"File\"]\n\n1. e4 {main} (1. d4 $1) e5 *";
        fs::write(&path, source).unwrap();
        let game = Game::from_pgn(&read_pgn_file(&path).unwrap()).unwrap();
        assert_eq!(game.pgn_tree().nodes()[0].children.len(), 2);
        fs::write(&path, game.save_text()).unwrap();
        let (restored, clock) = load_game(&path).unwrap().unwrap();
        assert!(clock.is_none());
        assert!(restored.pgn().contains("( 1. d4 $1 )"));
        let rows = pgn_rows(restored.pgn_tree());
        assert!(rows[0].1.contains("1. e4"));
        assert!(rows[1].1.contains("1. d4 $1"));
        assert!(rows[2].1.contains("1... e5"));
        fs::write(&path, vec![b'x'; 1_000_001]).unwrap();
        assert!(read_pgn_file(&path).is_err());
        fs::remove_file(path).unwrap();
    }
}
