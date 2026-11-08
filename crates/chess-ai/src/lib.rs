use shakmaty::{Chess, Color, Move, Position, Role};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Random,
    CaptureHunter,
    Material,
    Search,
}

impl Style {
    pub const ALL: [Self; 4] = [
        Self::Random,
        Self::CaptureHunter,
        Self::Material,
        Self::Search,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Random => "Random legal moves",
            Self::CaptureHunter => "Capture hunter",
            Self::Material => "Material collector",
            Self::Search => "Alpha-beta (depth 3)",
        }
    }
}

pub fn choose(pos: &Chess, style: Style, seed: u64) -> Option<Move> {
    let moves = pos.legal_moves();
    if moves.is_empty() {
        return None;
    }
    let offset = (seed as usize) % moves.len();
    let mut ordered: Vec<Move> = moves.into_iter().collect();
    ordered.rotate_left(offset);
    if style == Style::Random {
        return ordered.first().copied();
    }
    let side = pos.turn();
    ordered.into_iter().max_by_key(|m| {
        let mut next = pos.clone();
        next.play_unchecked(*m);
        match style {
            Style::CaptureHunter => {
                value(m.capture().unwrap_or(Role::King)) * 100 + i32::from(m.is_capture())
            }
            Style::Material => evaluate(&next, side),
            Style::Search => -search(&next, 2, i32::MIN + 1, i32::MAX - 1),
            Style::Random => 0,
        }
    })
}

fn search(pos: &Chess, depth: u8, mut alpha: i32, beta: i32) -> i32 {
    if pos.is_checkmate() {
        return -100_000 - i32::from(depth);
    }
    if depth == 0 || pos.is_stalemate() || pos.is_insufficient_material() {
        return evaluate(pos, pos.turn());
    }
    let mut best = i32::MIN + 1;
    for m in pos.legal_moves() {
        let mut next = pos.clone();
        next.play_unchecked(m);
        let score = -search(&next, depth - 1, -beta, -alpha);
        best = best.max(score);
        alpha = alpha.max(score);
        if alpha >= beta {
            break;
        }
    }
    best
}

fn evaluate(pos: &Chess, side: Color) -> i32 {
    if pos.is_checkmate() {
        return if pos.turn() == side {
            -100_000
        } else {
            100_000
        };
    }
    pos.board()
        .iter()
        .map(|(sq, piece)| {
            let material = value(piece.role) * 100;
            let center = if piece.role == Role::Pawn || piece.role == Role::Knight {
                let file = (sq.to_u32() % 8) as i32;
                let rank = (sq.to_u32() / 8) as i32;
                6 - (file - 3).abs() - (rank - 3).abs()
            } else {
                0
            };
            (material + center * 3) * if piece.color == side { 1 } else { -1 }
        })
        .sum()
}

fn value(role: Role) -> i32 {
    match role {
        Role::Pawn => 1,
        Role::Knight | Role::Bishop => 3,
        Role::Rook => 5,
        Role::Queen => 9,
        Role::King => 0,
    }
}

pub fn perft(pos: &Chess, depth: u8) -> u64 {
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

pub fn material_balance(pos: &Chess) -> i32 {
    pos.board()
        .iter()
        .map(|(_, piece)| value(piece.role) * if piece.color == Color::White { 1 } else { -1 })
        .sum()
}

pub fn move_hint(pos: &Chess) -> Option<String> {
    choose(pos, Style::Search, 0).map(|m| m.to_uci(pos.castles().mode()).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_style_returns_legal_move() {
        let p = Chess::default();
        for style in Style::ALL {
            assert!(p.is_legal(choose(&p, style, 42).unwrap()));
        }
    }
    #[test]
    fn perft_initial() {
        assert_eq!(perft(&Chess::default(), 3), 8902);
    }

    #[test]
    fn starting_material_is_even() {
        assert_eq!(material_balance(&Chess::default()), 0);
    }
}
