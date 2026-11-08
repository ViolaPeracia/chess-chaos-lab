use shakmaty::{
    CastlingMode, Chess, Color, EnPassantMode, Move, Position, Role, Square,
    fen::Fen,
    san::{San, SanPlus},
    uci::UciMove,
};

pub mod clock;

#[derive(Clone)]
pub struct Game {
    start: Chess,
    positions: Vec<Chess>,
    moves: Vec<Move>,
    san: Vec<String>,
    cursor: usize,
    resigned: Option<Color>,
    draw_claimed: bool,
    time_forfeit: Option<(Color, bool)>,
}

impl Default for Game {
    fn default() -> Self {
        let start = Chess::default();
        Self {
            start: start.clone(),
            positions: vec![start],
            moves: Vec::new(),
            san: Vec::new(),
            cursor: 0,
            resigned: None,
            draw_claimed: false,
            time_forfeit: None,
        }
    }
}

impl Game {
    pub fn from_fen(input: &str) -> Result<Self, String> {
        if input.len() > 256 {
            return Err("FEN exceeds 256-byte limit".into());
        }
        let fen: Fen = input
            .trim()
            .parse()
            .map_err(|e| format!("Invalid FEN: {e}"))?;
        let start: Chess = fen
            .into_position(CastlingMode::Standard)
            .map_err(|e| format!("Illegal position: {e}"))?;
        Ok(Self {
            start: start.clone(),
            positions: vec![start],
            moves: Vec::new(),
            san: Vec::new(),
            cursor: 0,
            resigned: None,
            draw_claimed: false,
            time_forfeit: None,
        })
    }

    pub fn position(&self) -> &Chess {
        &self.positions[self.cursor]
    }

    pub fn fen(&self) -> String {
        Fen::from_position(self.position(), EnPassantMode::Legal).to_string()
    }

    pub fn legal_moves(&self) -> Vec<Move> {
        if self.is_over() {
            Vec::new()
        } else {
            self.position().legal_moves().into_iter().collect()
        }
    }

    pub fn play(&mut self, m: Move) -> Result<(), String> {
        if self.is_over() || !self.position().is_legal(m) {
            return Err("Illegal move or game already finished".into());
        }
        let san = SanPlus::from_move(self.position().clone(), m).to_string();
        let mut next = self.position().clone();
        next.play_unchecked(m);
        self.moves.truncate(self.cursor);
        self.san.truncate(self.cursor);
        self.positions.truncate(self.cursor + 1);
        self.moves.push(m);
        self.san.push(san);
        self.positions.push(next);
        self.cursor += 1;
        Ok(())
    }

    pub fn play_uci(&mut self, input: &str) -> Result<(), String> {
        let uci: UciMove = input
            .parse()
            .map_err(|e| format!("Invalid UCI move: {e}"))?;
        let m = uci
            .to_move(self.position())
            .map_err(|e| format!("Illegal UCI move: {e}"))?;
        self.play(m)
    }

    pub fn move_from_to(&self, from: Square, to: Square, promotion: Option<Role>) -> Option<Move> {
        self.legal_moves().into_iter().find(|m| {
            m.from() == Some(from) && display_target(*m) == to && m.promotion() == promotion
        })
    }

    pub fn undo(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        self.resigned = None;
        self.draw_claimed = false;
        self.time_forfeit = None;
        true
    }

    pub fn redo(&mut self) -> bool {
        if self.cursor == self.moves.len() {
            return false;
        }
        self.cursor += 1;
        true
    }

    pub fn ply(&self) -> usize {
        self.cursor
    }
    pub fn san_history(&self) -> &[String] {
        &self.san[..self.cursor]
    }
    pub fn save_text(&self) -> String {
        let start = Fen::from_position(&self.start, EnPassantMode::Legal);
        let moves = self.moves[..self.cursor]
            .iter()
            .map(|m| m.to_uci(CastlingMode::Standard).to_string())
            .collect::<Vec<_>>()
            .join(" ");
        let outcome = if let Some(color) = self.resigned {
            format!("resign:{}", color_name(color))
        } else if self.draw_claimed {
            "draw".into()
        } else if let Some((color, draw)) = self.time_forfeit {
            format!("time:{}:{}", color_name(color), u8::from(draw))
        } else {
            "active".into()
        };
        format!("{start}\n{moves}\n{outcome}\n")
    }

    pub fn from_save_text(input: &str) -> Result<Self, String> {
        if input.len() > 1_000_000 {
            return Err("Save exceeds 1 MB limit".into());
        }
        let mut lines = input.lines();
        let mut game = Self::from_fen(lines.next().ok_or("Missing FEN")?)?;
        for uci in lines.next().unwrap_or("").split_whitespace() {
            game.play_uci(uci)?;
        }
        match lines.next().unwrap_or("active") {
            "resign:White" => game.resigned = Some(Color::White),
            "resign:Black" => game.resigned = Some(Color::Black),
            "draw" => game.draw_claimed = true,
            "time:White:0" => game.time_forfeit = Some((Color::White, false)),
            "time:Black:0" => game.time_forfeit = Some((Color::Black, false)),
            "time:White:1" => game.time_forfeit = Some((Color::White, true)),
            "time:Black:1" => game.time_forfeit = Some((Color::Black, true)),
            "active" => {}
            _ => return Err("Invalid save outcome".into()),
        }
        Ok(game)
    }

    pub fn from_pgn(input: &str) -> Result<Self, String> {
        if input.len() > 1_000_000 {
            return Err("PGN exceeds 1 MB limit".into());
        }
        let mut fen = None;
        let mut body = String::new();
        for line in input.lines() {
            let line = line.trim();
            if line.starts_with('[') {
                if line.starts_with("[FEN \"") && line.ends_with("\"]") {
                    fen = Some(&line[6..line.len() - 2]);
                } else if !line.ends_with(']') {
                    return Err("Malformed PGN header".into());
                }
            } else {
                body.push_str(line);
                body.push(' ');
            }
        }
        if body.contains(['{', '}', '(', ')', ';', '$']) {
            return Err(
                "Only a single plain mainline without comments, variations or NAGs is supported"
                    .into(),
            );
        }
        let mut game = if let Some(fen) = fen {
            Self::from_fen(fen)?
        } else {
            Self::default()
        };
        for token in body.split_whitespace() {
            if ["1-0", "0-1", "1/2-1/2", "*"].contains(&token) {
                break;
            }
            let token = token.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
            let token = token.trim_end_matches(['!', '?']);
            if token.is_empty() {
                continue;
            }
            let san = San::from_ascii(token.as_bytes())
                .map_err(|e| format!("Invalid SAN {token}: {e}"))?;
            let m = san
                .to_move(game.position())
                .map_err(|e| format!("Illegal SAN {token}: {e}"))?;
            game.play(m)?;
        }
        Ok(game)
    }

    pub fn resign(&mut self, color: Color) {
        if !self.is_over() {
            self.resigned = Some(color);
        }
    }

    pub fn forfeit_on_time(&mut self, color: Color) {
        if !self.is_over() {
            self.time_forfeit = Some((color, self.position().has_insufficient_material(!color)));
        }
    }

    pub fn can_claim_draw(&self) -> bool {
        !self.is_over() && (self.position().halfmoves() >= 100 || self.repetition_count() >= 3)
    }

    pub fn claim_draw(&mut self) -> bool {
        if !self.can_claim_draw() {
            return false;
        }
        self.draw_claimed = true;
        true
    }

    pub fn repetition_count(&self) -> usize {
        let current = repetition_key(self.position());
        self.positions[..=self.cursor]
            .iter()
            .filter(|p| repetition_key(p) == current)
            .count()
    }

    pub fn status(&self) -> String {
        if let Some(color) = self.resigned {
            return format!("{} resigned", color_name(color));
        }
        if self.draw_claimed {
            return "Draw claimed".into();
        }
        if let Some((color, draw)) = self.time_forfeit {
            return if draw {
                "Draw on time: opponent cannot checkmate".into()
            } else {
                format!("{} lost on time", color_name(color))
            };
        }
        let p = self.position();
        if p.is_checkmate() {
            return format!("Checkmate — {} wins", color_name(!p.turn()));
        }
        if p.is_stalemate() {
            return "Draw by stalemate".into();
        }
        if p.is_insufficient_material() {
            return "Draw by insufficient material".into();
        }
        if p.halfmoves() >= 150 {
            return "Draw by 75-move rule".into();
        }
        if self.repetition_count() >= 5 {
            return "Draw by fivefold repetition".into();
        }
        if p.is_check() {
            return format!("{} to move — check", color_name(p.turn()));
        }
        if self.can_claim_draw() {
            return format!("{} to move — draw claim available", color_name(p.turn()));
        }
        format!("{} to move", color_name(p.turn()))
    }

    pub fn is_over(&self) -> bool {
        self.resigned.is_some()
            || self.draw_claimed
            || self.time_forfeit.is_some()
            || self.position().is_checkmate()
            || self.position().is_stalemate()
            || self.position().is_insufficient_material()
            || self.position().halfmoves() >= 150
            || self.repetition_count() >= 5
    }

    pub fn pgn(&self) -> String {
        let mut result = String::from("[Event \"Chess Chaos Lab\"]\n[Site \"Local\"]\n");
        if self.start != Chess::default() {
            result.push_str(&format!(
                "[SetUp \"1\"]\n[FEN \"{}\"]\n",
                Fen::from_position(&self.start, EnPassantMode::Legal)
            ));
        }
        result.push('\n');
        let first_ply = (self.start.fullmoves().get() as usize - 1) * 2
            + usize::from(self.start.turn() == Color::Black);
        for (i, san) in self.san_history().iter().enumerate() {
            let ply = first_ply + i;
            if ply.is_multiple_of(2) {
                result.push_str(&format!("{}. ", ply / 2 + 1));
            } else if i == 0 {
                result.push_str(&format!("{}... ", ply / 2 + 1));
            }
            result.push_str(san);
            result.push(' ');
        }
        result.push_str(
            if self.resigned.is_some()
                || self.position().is_checkmate()
                || self.time_forfeit.is_some_and(|(_, draw)| !draw)
            {
                let losing_color = self
                    .resigned
                    .or_else(|| self.time_forfeit.map(|(color, _)| color))
                    .unwrap_or(self.position().turn());
                if losing_color == Color::White {
                    "0-1"
                } else {
                    "1-0"
                }
            } else if self.is_over() {
                "1/2-1/2"
            } else {
                "*"
            },
        );
        result
    }
}

pub fn display_target(m: Move) -> Square {
    if let Move::Castle { king, rook } = m {
        let king_file = king.to_u32() % 8;
        let rook_file = rook.to_u32() % 8;
        let destination_file = if rook_file > king_file { 6 } else { 2 };
        Square::new((king.to_u32() / 8) * 8 + destination_file)
    } else {
        m.to()
    }
}

fn repetition_key(p: &Chess) -> String {
    Fen::from_position(p, EnPassantMode::Legal)
        .to_string()
        .split_whitespace()
        .take(4)
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn color_name(color: Color) -> &'static str {
    if color == Color::White {
        "White"
    } else {
        "Black"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perft(pos: &Chess, depth: u8) -> u64 {
        if depth == 0 {
            return 1;
        }
        pos.legal_moves()
            .into_iter()
            .map(|m| {
                let mut next = pos.clone();
                next.play_unchecked(m);
                perft(&next, depth - 1)
            })
            .sum()
    }

    #[test]
    fn initial_perft() {
        let p = Chess::default();
        assert_eq!(perft(&p, 1), 20);
        assert_eq!(perft(&p, 2), 400);
        assert_eq!(perft(&p, 3), 8902);
    }

    #[test]
    fn scholar_mate_undo_redo() {
        let mut g = Game::default();
        for m in ["e2e4", "e7e5", "d1h5", "b8c6", "f1c4", "g8f6", "h5f7"] {
            g.play_uci(m).unwrap();
        }
        assert!(g.position().is_checkmate());
        assert!(g.undo());
        assert!(!g.is_over());
        assert!(g.redo());
        assert!(g.is_over());
    }

    #[test]
    fn stalemate_and_insufficient_material_end_games() {
        let stalemate = Game::from_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap();
        assert!(stalemate.is_over());
        assert_eq!(stalemate.status(), "Draw by stalemate");
        assert!(stalemate.pgn().ends_with("1/2-1/2"));

        let kings_only = Game::from_fen("4k3/8/8/8/8/8/8/4K3 w - - 0 1").unwrap();
        assert!(kings_only.is_over());
        assert_eq!(kings_only.status(), "Draw by insufficient material");
        assert!(kings_only.pgn().ends_with("1/2-1/2"));
    }

    #[test]
    fn checkmate_precedes_seventy_five_move_draw() {
        let mate = Game::from_fen("7k/6Q1/5K2/8/8/8/8/8 b - - 150 1").unwrap();
        assert!(mate.is_over());
        assert_eq!(mate.status(), "Checkmate — White wins");
        assert!(mate.pgn().ends_with("1-0"));
    }

    #[test]
    fn fen_rejects_illegal_position() {
        assert!(Game::from_fen("8/8/8/8/8/8/8/8 w - - 0 1").is_err());
        assert!(Game::from_fen(&"x".repeat(257)).is_err());
    }

    #[test]
    fn castling_target_and_repetition_claim() {
        let mut g = Game::default();
        for m in [
            "g1f3", "g8f6", "f3g1", "f6g8", "g1f3", "g8f6", "f3g1", "f6g8",
        ] {
            g.play_uci(m).unwrap();
        }
        assert_eq!(g.repetition_count(), 3);
        assert!(g.claim_draw());
        assert!(g.is_over());
        let restored = Game::from_save_text(&g.save_text()).unwrap();
        assert!(restored.is_over());

        let mut castle = Game::default();
        for m in ["e2e4", "e7e5", "g1f3", "b8c6", "f1e2", "g8f6"] {
            castle.play_uci(m).unwrap();
        }
        assert!(castle.move_from_to(Square::E1, Square::G1, None).is_some());
    }

    #[test]
    fn pgn_roundtrip_plain_mainline() {
        let mut g = Game::default();
        for m in ["e2e4", "e7e5", "g1f3", "b8c6"] {
            g.play_uci(m).unwrap();
        }
        let restored = Game::from_pgn(&g.pgn()).unwrap();
        assert_eq!(restored.fen(), g.fen());
        assert!(Game::from_pgn("1. e4 {comment} e5").is_err());
        assert!(Game::from_pgn("1. e4 e5 1-0").is_ok());

        let mut black_start = Game::default();
        black_start.play_uci("e2e4").unwrap();
        let mut black_start = Game::from_fen(&black_start.fen()).unwrap();
        black_start.play_uci("e7e5").unwrap();
        assert!(black_start.pgn().contains("1... e5"));
        assert_eq!(
            Game::from_pgn(&black_start.pgn()).unwrap().fen(),
            black_start.fen()
        );
    }

    #[test]
    fn resignation_names_the_resigning_side() {
        let mut game = Game::default();
        game.play_uci("e2e4").unwrap();
        game.resign(Color::White);
        assert_eq!(game.status(), "White resigned");
        assert!(game.pgn().ends_with("0-1"));
    }

    #[test]
    fn time_forfeit_is_saved() {
        let mut game = Game::default();
        game.forfeit_on_time(Color::White);
        assert_eq!(game.status(), "White lost on time");
        assert!(Game::from_save_text(&game.save_text()).unwrap().is_over());

        let mut lone_king = Game::from_fen("7k/8/8/8/8/8/8/KR6 w - - 0 1").unwrap();
        lone_king.forfeit_on_time(Color::White);
        assert_eq!(
            lone_king.status(),
            "Draw on time: opponent cannot checkmate"
        );
        assert!(lone_king.pgn().ends_with("1/2-1/2"));
    }

    #[test]
    fn en_passant_and_underpromotion() {
        let mut ep = Game::from_fen("8/8/8/3pP3/8/8/8/K6k w - d6 0 1").unwrap();
        ep.play_uci("e5d6").unwrap();
        assert_eq!(ep.position().board().piece_at(Square::D5), None);
        assert_eq!(
            ep.position().board().piece_at(Square::D6),
            Some(Color::White.pawn())
        );

        let mut promotion = Game::from_fen("8/P7/8/8/8/8/8/K6k w - - 0 1").unwrap();
        promotion.play_uci("a7a8n").unwrap();
        assert_eq!(
            promotion.position().board().piece_at(Square::A8),
            Some(Color::White.knight())
        );
    }

    #[test]
    fn standard_castling_moves_rook() {
        let mut game = Game::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
        let castle = game.move_from_to(Square::E1, Square::G1, None).unwrap();
        game.play(castle).unwrap();
        assert_eq!(
            game.position().board().piece_at(Square::G1),
            Some(Color::White.king())
        );
        assert_eq!(
            game.position().board().piece_at(Square::F1),
            Some(Color::White.rook())
        );
    }

    #[test]
    fn fifty_and_seventy_five_move_thresholds() {
        let position = "7k/8/8/8/8/8/8/K5R1 w - - {} 1";
        let mut claim = Game::from_fen(&position.replace("{}", "99")).unwrap();
        claim.play_uci("g1g2").unwrap();
        assert!(claim.can_claim_draw());
        assert!(!claim.is_over());

        let mut automatic = Game::from_fen(&position.replace("{}", "149")).unwrap();
        automatic.play_uci("g1g2").unwrap();
        assert!(automatic.is_over());
        assert_eq!(automatic.status(), "Draw by 75-move rule");
    }

    #[test]
    fn fifth_repetition_ends_game() {
        let mut game = Game::default();
        for _ in 0..4 {
            for uci in ["g1f3", "g8f6", "f3g1", "f6g8"] {
                game.play_uci(uci).unwrap();
            }
        }
        assert_eq!(game.repetition_count(), 5);
        assert_eq!(game.status(), "Draw by fivefold repetition");
    }
}
