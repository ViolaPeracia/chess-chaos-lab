use pgn::PgnGame;
use shakmaty::{
    CastlingMode, Chess, Color, EnPassantMode, Move, Position, Role, Square, fen::Fen,
    san::SanPlus, uci::UciMove,
};

pub mod clock;
pub mod pgn;

/// A finished game result, separate from a player's right to claim a draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameOutcome {
    /// Play may continue; a draw claim can still be available.
    Ongoing,
    /// The named side won.
    Win(Color),
    /// The game ended in a draw.
    Draw,
}

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
    pgn_tree: PgnGame,
    pgn_path: Vec<usize>,
}

impl Default for Game {
    fn default() -> Self {
        let start = Chess::default();
        Self {
            start: start.clone(),
            positions: vec![start.clone()],
            moves: Vec::new(),
            san: Vec::new(),
            cursor: 0,
            resigned: None,
            draw_claimed: false,
            time_forfeit: None,
            pgn_tree: local_pgn_tree(&start),
            pgn_path: Vec::new(),
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
        let mut game = Self {
            start: start.clone(),
            positions: vec![start.clone()],
            moves: Vec::new(),
            san: Vec::new(),
            cursor: 0,
            resigned: None,
            draw_claimed: false,
            time_forfeit: None,
            pgn_tree: local_pgn_tree(&start),
            pgn_path: Vec::new(),
        };
        game.sync_pgn_result();
        Ok(game)
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
        let tree = &mut self.pgn_tree;
        let parent = if self.cursor == 0 {
            0
        } else {
            self.pgn_path[self.cursor - 1]
        };
        let node = if let Some(existing) = tree.nodes[parent]
            .children
            .iter()
            .copied()
            .find(|&index| tree.nodes[index].movement == Some(m))
        {
            existing
        } else {
            tree.add_move(parent, m, Vec::new())?
        };
        self.pgn_path.truncate(self.cursor);
        self.pgn_path.push(node);
        self.moves.truncate(self.cursor);
        self.san.truncate(self.cursor);
        self.positions.truncate(self.cursor + 1);
        self.moves.push(m);
        self.san.push(san);
        self.positions.push(next);
        self.cursor += 1;
        self.sync_pgn_result();
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
        let mut saved = format!("{start}\n{moves}\n{outcome}\n");
        let tree = &self.pgn_tree;
        let mut parent = 0;
        let mut branches = Vec::new();
        for &node in &self.pgn_path[..self.cursor] {
            let branch = tree.nodes[parent]
                .children
                .iter()
                .position(|&child| child == node)
                .expect("PGN path must belong to tree");
            branches.push(branch.to_string());
            parent = node;
        }
        saved.push_str("pgn:");
        saved.push_str(&branches.join(","));
        saved.push(':');
        for byte in self.pgn().bytes() {
            saved.push_str(&format!("{byte:02x}"));
        }
        saved.push('\n');
        saved
    }

    pub fn from_save_text(input: &str) -> Result<Self, String> {
        if input.len() > 4_000_000 {
            return Err("Save exceeds 4 MB limit".into());
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
        if let Some(line) = lines.next()
            && let Some(encoded) = line.strip_prefix("pgn:")
        {
            let (path, hex) = encoded.split_once(':').ok_or("Invalid saved PGN")?;
            if hex.len() % 2 != 0 {
                return Err("Invalid saved PGN encoding".into());
            }
            let bytes = hex
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| {
                    u8::from_str_radix(std::str::from_utf8(pair).map_err(|e| e.to_string())?, 16)
                        .map_err(|e| e.to_string())
                })
                .collect::<Result<Vec<_>, _>>()?;
            let pgn = String::from_utf8(bytes).map_err(|e| e.to_string())?;
            let mut parsed = Self::from_pgn(&pgn)?;
            let mut node = 0;
            if !path.is_empty() {
                for branch in path.split(',') {
                    let branch: usize = branch.parse().map_err(|_| "Invalid saved PGN path")?;
                    node = *parsed.pgn_tree.nodes[node]
                        .children
                        .get(branch)
                        .ok_or("Invalid saved PGN path")?;
                }
            }
            parsed.goto_pgn_node(node)?;
            if parsed.fen() != game.fen() {
                return Err("Saved PGN position disagrees with moves".into());
            }
            parsed.resigned = game.resigned;
            parsed.draw_claimed = game.draw_claimed;
            parsed.time_forfeit = game.time_forfeit;
            game = parsed;
        }
        Ok(game)
    }

    pub fn from_pgn(input: &str) -> Result<Self, String> {
        let tree = PgnGame::parse(input)?;
        let mut game = Self::from_fen(&tree.fen_at(0).ok_or("Missing PGN start")?)?;
        game.pgn_tree = tree;
        let mut node = 0;
        while let Some(&next) = game.pgn_tree.nodes[node].children.first() {
            game.pgn_path.push(next);
            let entry = &game.pgn_tree.nodes[next];
            game.moves.push(entry.movement.ok_or("Missing PGN move")?);
            game.san.push(entry.san.clone());
            game.positions.push(entry.position.clone());
            game.cursor += 1;
            node = next;
        }
        Ok(game)
    }

    /// The PGN tree for this game, including all mainline and variation nodes.
    pub fn pgn_tree(&self) -> &PgnGame {
        &self.pgn_tree
    }

    /// Navigate to any move in the PGN tree and make its position current.
    pub fn goto_pgn_node(&mut self, node: usize) -> Result<(), String> {
        let tree = &self.pgn_tree;
        let path = tree.path_to(node).ok_or("Invalid PGN node")?;
        self.moves.clear();
        self.san.clear();
        self.positions.truncate(1);
        for &index in &path {
            let entry = &tree.nodes[index];
            self.moves.push(entry.movement.ok_or("Missing PGN move")?);
            self.san.push(entry.san.clone());
            self.positions.push(entry.position.clone());
        }
        self.cursor = path.len();
        self.pgn_path = path;
        self.resigned = None;
        self.draw_claimed = false;
        self.time_forfeit = None;
        Ok(())
    }

    pub fn resign(&mut self, color: Color) {
        if !self.is_over() {
            self.resigned = Some(color);
            self.sync_pgn_result();
        }
    }

    pub fn forfeit_on_time(&mut self, color: Color) {
        if !self.is_over() {
            self.time_forfeit = Some((color, self.position().has_insufficient_material(!color)));
            self.sync_pgn_result();
        }
    }

    pub fn can_claim_draw(&self) -> bool {
        !self.is_over() && (self.position().halfmoves() >= 100 || self.repetition_count() >= 3)
    }

    /// Whether announcing this legal move would make a threefold or 50-move claim valid.
    /// The move is not played when the claim is made.
    pub fn can_claim_draw_after(&self, movement: Move) -> bool {
        if self.is_over() || !self.position().is_legal(movement) {
            return false;
        }
        let mut next = self.position().clone();
        next.play_unchecked(movement);
        if next.is_checkmate() {
            return false;
        }
        let key = repetition_key(&next);
        next.halfmoves() >= 100
            || self.positions[self.cursor.saturating_sub(next.halfmoves() as usize)..=self.cursor]
                .iter()
                .filter(|position| repetition_key(position) == key)
                .count()
                >= 2
    }

    /// Claim a draw by announcing a legal move that would meet the FIDE threshold.
    pub fn claim_draw_after(&mut self, movement: Move) -> bool {
        if !self.can_claim_draw_after(movement) {
            return false;
        }
        self.draw_claimed = true;
        self.sync_pgn_result();
        true
    }

    pub fn claim_draw(&mut self) -> bool {
        if !self.can_claim_draw() {
            return false;
        }
        self.draw_claimed = true;
        self.sync_pgn_result();
        true
    }

    pub fn repetition_count(&self) -> usize {
        let current = repetition_key(self.position());
        // A pawn move or capture cannot be undone, so earlier positions cannot repeat.
        self.positions[self
            .cursor
            .saturating_sub(self.position().halfmoves() as usize)
            ..=self.cursor]
            .iter()
            .filter(|p| repetition_key(p) == current)
            .count()
    }

    pub fn status(&self) -> String {
        if let Some(color) = self.resigned {
            if self.position().has_insufficient_material(!color) {
                return "Draw: opponent cannot checkmate after resignation".into();
            }
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

    /// Return a terminal result or `Ongoing`. A claimable draw remains ongoing.
    pub fn outcome(&self) -> GameOutcome {
        if !self.is_over() {
            return GameOutcome::Ongoing;
        }
        if self.draw_claimed
            || self.time_forfeit.is_some_and(|(_, draw)| draw)
            || self
                .resigned
                .is_some_and(|color| self.position().has_insufficient_material(!color))
            || self.position().is_stalemate()
            || self.position().is_insufficient_material()
            || (!self.position().is_checkmate()
                && (self.position().halfmoves() >= 150 || self.repetition_count() >= 5))
        {
            return GameOutcome::Draw;
        }
        let loser = self
            .resigned
            .or_else(|| self.time_forfeit.map(|(color, _)| color))
            .unwrap_or(self.position().turn());
        GameOutcome::Win(!loser)
    }

    pub fn pgn(&self) -> String {
        self.pgn_tree.export()
    }

    fn result_token(&self) -> &'static str {
        if self
            .resigned
            .is_some_and(|color| self.position().has_insufficient_material(!color))
        {
            return "1/2-1/2";
        }
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
        }
    }

    fn sync_pgn_result(&mut self) {
        let tree = &self.pgn_tree;
        let mut parent = 0;
        for &node in &self.pgn_path[..self.cursor] {
            if tree.nodes[parent].children.first() != Some(&node) {
                return;
            }
            parent = node;
        }
        if !tree.nodes[parent].children.is_empty() {
            return;
        }
        let result = self.result_token().to_owned();
        self.pgn_tree.result = result;
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

fn local_pgn_tree(start: &Chess) -> PgnGame {
    let mut tree = PgnGame::new(start.clone());
    tree.headers = vec![
        ("Event".into(), "Chess Chaos Lab".into()),
        ("Site".into(), "Local".into()),
    ];
    if start != &Chess::default() {
        tree.headers.push(("SetUp".into(), "1".into()));
        tree.headers.push((
            "FEN".into(),
            Fen::from_position(start, EnPassantMode::Legal).to_string(),
        ));
    }
    tree
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
    fn reference_perft_castling_en_passant_and_promotion() {
        // https://www.chessprogramming.org/Perft_Results, positions 2, 3, and 4.
        let cases = [
            (
                "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
                48,
                2039,
                97862,
            ),
            ("8/2p5/3p4/KP5r/1R3p1k/8/4P1P1/8 w - - 0 1", 14, 191, 2812),
            (
                "r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1",
                6,
                264,
                9467,
            ),
        ];
        for (fen, one, two, three) in cases {
            let game = Game::from_fen(fen).unwrap();
            assert_eq!(perft(game.position(), 1), one, "{fen}");
            assert_eq!(perft(game.position(), 2), two, "{fen}");
            assert_eq!(perft(game.position(), 3), three, "{fen}");
        }
    }

    #[test]
    fn generated_legal_games_round_trip_through_pgn() {
        for seed in 0..32usize {
            let mut game = Game::default();
            for ply in 0..48 {
                let legal = game.legal_moves();
                if legal.is_empty() {
                    break;
                }
                game.play(legal[(seed * 17 + ply * 13) % legal.len()])
                    .unwrap();
            }
            let restored = Game::from_pgn(&game.pgn()).unwrap();
            assert_eq!(restored.fen(), game.fen(), "seed {seed}");
        }
    }

    #[test]
    fn castling_through_check_and_discovered_en_passant_are_illegal() {
        let castle = Game::from_fen("k4r2/8/8/8/8/8/8/4K2R w K - 0 1").unwrap();
        assert!(castle.move_from_to(Square::E1, Square::G1, None).is_none());
        let mut ep = Game::from_fen("7k/8/8/4KPpr/8/8/8/8 w - g6 0 1").unwrap();
        assert!(ep.play_uci("f5g6").is_err());
    }

    #[test]
    fn claimable_draw_is_not_an_outcome_and_announced_move_can_be_claimed() {
        let mut game = Game::default();
        for _ in 0..2 {
            for uci in ["g1f3", "g8f6", "f3g1", "f6g8"] {
                game.play_uci(uci).unwrap();
            }
        }
        assert!(game.can_claim_draw());
        assert_eq!(game.outcome(), GameOutcome::Ongoing);
        assert!(!game.is_over());
        let mut prospective = Game::default();
        for uci in ["g1f3", "g8f6", "f3g1", "f6g8", "g1f3", "g8f6", "f3g1"] {
            prospective.play_uci(uci).unwrap();
        }
        let movement = "f6g8"
            .parse::<UciMove>()
            .unwrap()
            .to_move(prospective.position())
            .unwrap();
        assert!(prospective.can_claim_draw_after(movement));
        assert!(prospective.claim_draw_after(movement));
        assert_eq!(prospective.outcome(), GameOutcome::Draw);
        assert_eq!(prospective.ply(), 7);
    }

    #[test]
    fn mating_material_is_not_confused_with_forced_mate() {
        let two_knights = Game::from_fen("7k/8/8/8/8/8/8/KNN5 w - - 0 1").unwrap();
        assert!(!two_knights.is_over());
        let lone_bishop = Game::from_fen("7k/8/8/8/8/8/8/KB6 w - - 0 1").unwrap();
        assert_eq!(lone_bishop.outcome(), GameOutcome::Draw);
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
        let annotated = Game::from_pgn("1. e4 {comment} e5 *").unwrap();
        assert!(annotated.pgn().contains("{comment}"));
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

        let mut no_mate = Game::from_fen("7k/8/8/8/8/8/8/KR6 w - - 0 1").unwrap();
        no_mate.resign(Color::White);
        assert_eq!(no_mate.outcome(), GameOutcome::Draw);
        assert!(no_mate.pgn().ends_with("1/2-1/2"));
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

    #[test]
    fn imported_variation_navigation_and_autosave_preserve_tree() {
        let mut game =
            Game::from_pgn("[Event \"Test\"]\n\n1. e4 {main} (1. d4 $1 d5) e5 *").unwrap();
        let branch = game.pgn_tree().nodes[0].children[1];
        game.goto_pgn_node(branch).unwrap();
        assert_eq!(game.san_history(), &["d4"]);
        game.play_uci("g8f6").unwrap();
        let pgn = game.pgn();
        assert!(pgn.contains("1. d4 $1"));
        assert!(pgn.contains("( 1... Nf6 )"));
        assert!(pgn.contains("{main}"));
        let restored = Game::from_save_text(&game.save_text()).unwrap();
        assert_eq!(restored.fen(), game.fen());
        assert_eq!(restored.pgn(), pgn);
        assert_eq!(restored.pgn_tree().nodes[0].children.len(), 2);
    }

    #[test]
    fn new_game_keeps_old_continuation_as_variation_after_undo() {
        let mut game = Game::default();
        game.play_uci("e2e4").unwrap();
        game.play_uci("e7e5").unwrap();
        assert!(game.undo());
        game.play_uci("c7c5").unwrap();
        let tree = game.pgn_tree();
        let e4 = tree.nodes[0].children[0];
        assert_eq!(tree.nodes[e4].children.len(), 2);
        assert_eq!(tree.nodes[tree.nodes[e4].children[0]].san, "e5");
        assert_eq!(tree.nodes[tree.nodes[e4].children[1]].san, "c5");
        let restored = Game::from_save_text(&game.save_text()).unwrap();
        assert_eq!(restored.fen(), game.fen());
        assert_eq!(restored.pgn(), game.pgn());
    }

    #[test]
    fn legacy_three_line_save_still_loads() {
        let legacy = format!("{}\ne2e4 e7e5\nactive\n", Game::default().fen());
        let game = Game::from_save_text(&legacy).unwrap();
        assert_eq!(game.san_history(), &["e4", "e5"]);
        assert_eq!(game.pgn_tree().nodes.len(), 3);
        assert!(Game::from_pgn(&game.pgn()).is_ok());
    }

    #[test]
    fn visiting_terminal_variation_keeps_mainline_result() {
        let mut game =
            Game::from_pgn("[Result \"1-0\"]\n\n1. e4 (1. f3 e5 2. g4 Qh4#) e5 1-0").unwrap();
        let branch = game.pgn_tree().nodes[0].children[1];
        let mut node = branch;
        while let Some(&next) = game.pgn_tree().nodes[node].children.first() {
            node = next;
        }
        game.goto_pgn_node(node).unwrap();
        assert_eq!(game.outcome(), GameOutcome::Win(Color::Black));
        assert!(game.pgn().ends_with("1-0"));
        assert!(Game::from_pgn(&game.pgn()).is_ok());
    }
}
