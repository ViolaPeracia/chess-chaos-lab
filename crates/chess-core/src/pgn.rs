//! A bounded PGN game tree. Node zero is the starting position; each node's
//! first child is its mainline continuation and later children are RAVs.

use shakmaty::{
    CastlingMode, Chess, Color, Move, Position,
    fen::Fen,
    san::{San, SanPlus},
};

const MAX_PGN_BYTES: usize = 1_000_000;
const MAX_NODES: usize = 50_000;
const MAX_RAV_DEPTH: usize = 64;
const MAX_HEADERS: usize = 256;
type Headers = Vec<(String, String)>;

struct NumberPrefix {
    number: u32,
    dots: usize,
}

/// One legal move and its annotations in a PGN game tree.
#[derive(Clone)]
pub struct PgnNode {
    /// Parent node index; absent for the starting position.
    pub parent: Option<usize>,
    /// Continuations, with the mainline first.
    pub children: Vec<usize>,
    /// SAN generated from the position before this move.
    pub san: String,
    /// Comments immediately before the move.
    pub before: Vec<String>,
    /// Comments immediately after the move.
    pub after: Vec<String>,
    /// Numeric annotation glyphs in their original order.
    pub nags: Vec<u16>,
    pub(crate) movement: Option<Move>,
    pub(crate) position: Chess,
}

/// One PGN game, including headers, result, and all move variations.
#[derive(Clone)]
pub struct PgnGame {
    /// Header pairs in input order. Export places the Seven Tag Roster first.
    pub(crate) headers: Vec<(String, String)>,
    /// Node zero is the starting position.
    pub(crate) nodes: Vec<PgnNode>,
    /// One of `1-0`, `0-1`, `1/2-1/2`, or `*`.
    pub(crate) result: String,
}

impl PgnGame {
    /// Header pairs in input order. Export writes roster tags first.
    pub fn headers(&self) -> &[(String, String)] {
        &self.headers
    }

    /// All move nodes, beginning with the starting position at index zero.
    pub fn nodes(&self) -> &[PgnNode] {
        &self.nodes
    }

    /// The recorded mainline result token.
    pub fn result(&self) -> &str {
        &self.result
    }

    /// Create an empty tree from a legal standard-chess position.
    pub fn new(start: Chess) -> Self {
        Self {
            headers: Vec::new(),
            nodes: vec![PgnNode {
                parent: None,
                children: Vec::new(),
                san: String::new(),
                before: Vec::new(),
                after: Vec::new(),
                nags: Vec::new(),
                movement: None,
                position: start,
            }],
            result: "*".into(),
        }
    }

    /// Parse exactly one game. A second game, malformed token, or illegal move is an error.
    pub fn parse(input: &str) -> Result<Self, String> {
        if input.len() > MAX_PGN_BYTES {
            return Err("PGN exceeds 1 MB limit".into());
        }
        let (headers, body) = parse_headers(input)?;
        let setup = headers
            .iter()
            .find(|(name, _)| name == "SetUp")
            .map(|(_, value)| value.as_str());
        let fen = headers
            .iter()
            .find(|(name, _)| name == "FEN")
            .map(|(_, value)| value.as_str());
        if setup.is_some_and(|value| value != "0" && value != "1") {
            return Err("SetUp must be 0 or 1".into());
        }
        if setup == Some("1") && fen.is_none() {
            return Err("SetUp 1 requires FEN".into());
        }
        if fen.is_some() && setup != Some("1") {
            return Err("FEN requires SetUp 1".into());
        }
        let start = if let Some(fen) = fen {
            let parsed: Fen = fen.parse().map_err(|e| format!("Invalid FEN: {e}"))?;
            parsed
                .into_position(CastlingMode::Standard)
                .map_err(|e| format!("Illegal FEN: {e}"))?
        } else {
            Chess::default()
        };
        let mut game = Self::new(start);
        game.headers = headers;
        let tokens = lex(body)?;
        let mut parser = Parser {
            tokens: &tokens,
            index: 0,
            game: &mut game,
            result_seen: false,
        };
        parser.sequence(0, 0, false)?;
        if parser.index != tokens.len() {
            return Err("Unexpected PGN content after result".into());
        }
        if let Some((_, header_result)) = game.headers.iter().find(|(name, _)| name == "Result")
            && header_result != &game.result
        {
            return Err("Result header and movetext disagree".into());
        }
        Ok(game)
    }

    /// Serialize the entire tree with normalized spacing and PGN escaping.
    pub fn export(&self) -> String {
        let mut output = String::new();
        const ROSTER: [&str; 7] = ["Event", "Site", "Date", "Round", "White", "Black", "Result"];
        for name in ROSTER {
            let value = self
                .headers
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, value)| value.as_str())
                .unwrap_or(if name == "Date" { "????.??.??" } else { "?" });
            write_header(
                &mut output,
                name,
                if name == "Result" {
                    &self.result
                } else {
                    value
                },
            );
        }
        for (name, value) in &self.headers {
            if !ROSTER.contains(&name.as_str()) {
                write_header(&mut output, name, value);
            }
        }
        if !output.is_empty() {
            output.push('\n');
        }
        if !self.nodes[0].children.is_empty() {
            self.write_line(0, 0, &mut output);
        }
        output.push_str(&self.result);
        output
    }

    /// Return node indices from the starting position to `node`.
    pub fn path_to(&self, node: usize) -> Option<Vec<usize>> {
        if node >= self.nodes.len() {
            return None;
        }
        let mut path = Vec::new();
        let mut current = node;
        while current != 0 {
            path.push(current);
            current = self.nodes[current].parent?;
        }
        path.reverse();
        Some(path)
    }

    /// FEN of a node, useful for navigating a variation without losing the tree.
    pub fn fen_at(&self, node: usize) -> Option<String> {
        self.nodes
            .get(node)
            .map(|n| Fen::from_position(&n.position, shakmaty::EnPassantMode::Legal).to_string())
    }

    /// Move number and side to move immediately before this node's move.
    pub fn move_number_at(&self, node: usize) -> Option<(u32, Color)> {
        let parent = self.nodes.get(node)?.parent?;
        let position = &self.nodes[parent].position;
        Some((position.fullmoves().get(), position.turn()))
    }

    pub(crate) fn add_move(
        &mut self,
        parent: usize,
        movement: Move,
        before: Vec<String>,
    ) -> Result<usize, String> {
        if self.nodes.len() >= MAX_NODES {
            return Err("PGN exceeds 50,000 moves".into());
        }
        let position = &self.nodes[parent].position;
        if position.is_checkmate()
            || position.is_stalemate()
            || position.is_insufficient_material()
            || position.halfmoves() >= 150
        {
            return Err("Move after automatic game termination".into());
        }
        if !position.is_legal(movement) {
            return Err("Illegal PGN move".into());
        }
        let san = SanPlus::from_move(position.clone(), movement).to_string();
        let mut next = position.clone();
        next.play_unchecked(movement);
        let index = self.nodes.len();
        self.nodes.push(PgnNode {
            parent: Some(parent),
            children: Vec::new(),
            san,
            before,
            after: Vec::new(),
            nags: Vec::new(),
            movement: Some(movement),
            position: next,
        });
        self.nodes[parent].children.push(index);
        Ok(index)
    }

    fn write_line(&self, mut parent: usize, mut branch: usize, out: &mut String) {
        let mut first = true;
        loop {
            let node_index = self.nodes[parent].children[branch];
            let node = &self.nodes[node_index];
            let position = &self.nodes[parent].position;
            if position.turn() == Color::White {
                out.push_str(&format!("{}. ", position.fullmoves()));
            } else if first || !node.before.is_empty() {
                out.push_str(&format!("{}... ", position.fullmoves()));
            }
            for comment in &node.before {
                write_comment(out, comment);
            }
            out.push_str(&node.san);
            out.push(' ');
            for nag in &node.nags {
                out.push_str(&format!("${nag} "));
            }
            for comment in &node.after {
                write_comment(out, comment);
            }
            if branch == 0 {
                for sibling in 1..self.nodes[parent].children.len() {
                    out.push_str("( ");
                    self.write_line(parent, sibling, out);
                    out.push_str(") ");
                }
            }
            if node.children.is_empty() {
                break;
            }
            parent = node_index;
            branch = 0;
            first = false;
        }
    }
}

fn write_header(out: &mut String, name: &str, value: &str) {
    out.push('[');
    out.push_str(name);
    out.push_str(" \"");
    for character in value.chars() {
        if character == '"' || character == '\\' {
            out.push('\\');
        }
        out.push(character);
    }
    out.push_str("\"]\n");
}

fn write_comment(out: &mut String, comment: &str) {
    if comment.contains('}') {
        out.push(';');
        out.push_str(comment);
        out.push('\n');
    } else {
        out.push('{');
        out.push_str(comment);
        out.push_str("} ");
    }
}

fn parse_headers(input: &str) -> Result<(Headers, &str), String> {
    let mut rest = input.trim_start_matches(char::is_whitespace);
    let mut headers = Vec::new();
    while rest.starts_with('%') {
        rest = rest
            .find('\n')
            .map_or("", |end| &rest[end + 1..])
            .trim_start_matches(char::is_whitespace);
    }
    while let Some(after_open) = rest.strip_prefix('[') {
        if headers.len() >= MAX_HEADERS {
            return Err("PGN exceeds 256 headers".into());
        }
        let name_end = after_open
            .find(char::is_whitespace)
            .ok_or("Unclosed PGN header")?;
        let name = &after_open[..name_end];
        if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_') {
            return Err(format!("Invalid PGN header name: {name}"));
        }
        let after_name = after_open[name_end..].trim_start_matches(char::is_whitespace);
        let Some(mut value_input) = after_name.strip_prefix('"') else {
            return Err(format!("Missing quoted value for {name}"));
        };
        let mut value = String::new();
        loop {
            let Some(character) = value_input.chars().next() else {
                return Err(format!("Unclosed PGN header {name}"));
            };
            value_input = &value_input[character.len_utf8()..];
            match character {
                '"' => break,
                '\\' => {
                    let Some(escaped) = value_input.chars().next() else {
                        return Err(format!("Unclosed escape in {name}"));
                    };
                    if escaped != '"' && escaped != '\\' {
                        return Err(format!("Invalid escape in {name}"));
                    }
                    value.push(escaped);
                    value_input = &value_input[escaped.len_utf8()..];
                }
                '\r' | '\n' | '\0' => {
                    return Err(format!("Control character in PGN header {name}"));
                }
                _ => value.push(character),
            }
        }
        let after_value = value_input.trim_start_matches([' ', '\t']);
        let Some(after_close) = after_value.strip_prefix(']') else {
            return Err(format!("Missing ] in {name}"));
        };
        if headers
            .iter()
            .any(|(existing, _): &(String, String)| existing == name)
        {
            return Err(format!("Duplicate PGN header {name}"));
        }
        headers.push((name.into(), value));
        rest = after_close.trim_start_matches(char::is_whitespace);
        while rest.starts_with('%') {
            rest = rest
                .find('\n')
                .map_or("", |end| &rest[end + 1..])
                .trim_start_matches(char::is_whitespace);
        }
    }
    Ok((headers, rest))
}

#[derive(Clone)]
enum Token {
    Word(String),
    Comment(String),
    Nag(u16),
    Open,
    Close,
}

fn lex(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut cursor = 0;
    let bytes = input.as_bytes();
    while cursor < input.len() {
        let character = input[cursor..].chars().next().unwrap();
        if character.is_whitespace() {
            cursor += character.len_utf8();
            continue;
        }
        if character == '%' && (cursor == 0 || bytes[cursor - 1] == b'\n') {
            cursor = input[cursor..]
                .find('\n')
                .map_or(input.len(), |offset| cursor + offset + 1);
            continue;
        }
        match character {
            '{' => {
                let rest = &input[cursor + 1..];
                let end = rest
                    .find('}')
                    .ok_or_else(|| format!("Unclosed comment at byte {cursor}"))?;
                tokens.push(Token::Comment(rest[..end].to_owned()));
                cursor += end + 2;
            }
            ';' => {
                let rest = &input[cursor + 1..];
                let end = rest.find('\n').unwrap_or(rest.len());
                tokens.push(Token::Comment(
                    rest[..end].trim_end_matches('\r').to_owned(),
                ));
                cursor += end + 1;
            }
            '(' => {
                tokens.push(Token::Open);
                cursor += 1;
            }
            ')' => {
                tokens.push(Token::Close);
                cursor += 1;
            }
            '$' => {
                let start = cursor + 1;
                cursor = start;
                while cursor < input.len() && bytes[cursor].is_ascii_digit() {
                    cursor += 1;
                }
                if cursor == start {
                    return Err(format!("NAG needs digits at byte {start}"));
                }
                let nag = input[start..cursor]
                    .parse()
                    .map_err(|_| format!("Invalid NAG at byte {start}"))?;
                if nag > 255 {
                    return Err(format!("NAG out of range at byte {start}"));
                }
                tokens.push(Token::Nag(nag));
            }
            '}' | ']' | '[' => return Err(format!("Unexpected {character} at byte {cursor}")),
            _ => {
                let start = cursor;
                while cursor < input.len() {
                    let c = input[cursor..].chars().next().unwrap();
                    if c.is_whitespace() || "{}();$[]".contains(c) {
                        break;
                    }
                    cursor += c.len_utf8();
                }
                tokens.push(Token::Word(input[start..cursor].to_owned()));
            }
        }
        if tokens.len() > MAX_NODES * 8 {
            return Err("PGN has too many tokens".into());
        }
    }
    Ok(tokens)
}

struct Parser<'a> {
    tokens: &'a [Token],
    index: usize,
    game: &'a mut PgnGame,
    result_seen: bool,
}

impl Parser<'_> {
    fn sequence(&mut self, mut parent: usize, depth: usize, variation: bool) -> Result<(), String> {
        if depth > MAX_RAV_DEPTH {
            return Err("PGN variation depth exceeds 64".into());
        }
        let mut last: Option<usize> = None;
        let mut before = Vec::new();
        let mut moves = 0;
        let mut number_pending = false;
        while let Some(token) = self.tokens.get(self.index).cloned() {
            self.index += 1;
            match token {
                Token::Comment(comment) => {
                    if let Some(node) = last.filter(|_| !number_pending) {
                        self.game.nodes[node].after.push(comment);
                    } else {
                        before.push(comment);
                    }
                }
                Token::Nag(nag) => {
                    if number_pending {
                        return Err("NAG before a numbered move".into());
                    }
                    let node = last.ok_or("NAG without a move")?;
                    self.game.nodes[node].nags.push(nag);
                }
                Token::Open => {
                    if number_pending {
                        return Err("Variation before a numbered move".into());
                    }
                    let node = last.ok_or("Variation without a preceding move")?;
                    let branch_parent = self.game.nodes[node].parent.unwrap();
                    self.sequence(branch_parent, depth + 1, true)?;
                }
                Token::Close => {
                    if number_pending {
                        return Err("Move number without move".into());
                    }
                    if !variation {
                        return Err("Unmatched ) in PGN".into());
                    }
                    if moves == 0 {
                        return Err("Empty PGN variation".into());
                    }
                    return Ok(());
                }
                Token::Word(word) => {
                    if is_result(&word) {
                        if number_pending {
                            return Err("Move number without move".into());
                        }
                        if variation {
                            return Err("Result inside variation".into());
                        }
                        self.game.result = word;
                        self.result_seen = true;
                        if self.index != self.tokens.len() {
                            return Err("Content after PGN result".into());
                        }
                        return Ok(());
                    }
                    if self.result_seen {
                        return Err("Content after PGN result".into());
                    }
                    let (prefix, san_word) = split_number(&word)?;
                    if let Some(NumberPrefix { number, dots }) = prefix {
                        let position = &self.game.nodes[parent].position;
                        if number != position.fullmoves().get()
                            || (dots == 3) != (position.turn() == Color::Black)
                        {
                            return Err(format!("Move number {word} does not match position"));
                        }
                    }
                    if san_word.is_empty() {
                        number_pending = true;
                        continue;
                    }
                    if let Some(nag) = glyph_nag(san_word) {
                        if number_pending {
                            return Err("Annotation before a numbered move".into());
                        }
                        let node = last.ok_or("Annotation without a move")?;
                        self.game.nodes[node].nags.push(nag);
                        continue;
                    }
                    let (san_text, glyph) = split_glyph(san_word);
                    let san = San::from_ascii(san_text.as_bytes())
                        .map_err(|e| format!("Invalid SAN {san_text}: {e}"))?;
                    let movement = san
                        .to_move(&self.game.nodes[parent].position)
                        .map_err(|e| format!("Illegal SAN {san_text}: {e}"))?;
                    let node = self
                        .game
                        .add_move(parent, movement, std::mem::take(&mut before))?;
                    if let Some(nag) = glyph {
                        self.game.nodes[node].nags.push(nag);
                    }
                    parent = node;
                    last = Some(node);
                    moves += 1;
                    number_pending = false;
                }
            }
        }
        if variation {
            return Err("Unclosed PGN variation".into());
        }
        if number_pending {
            return Err("Move number without move".into());
        }
        if !before.is_empty() {
            return Err("Comment without following move".into());
        }
        Ok(())
    }
}

fn is_result(word: &str) -> bool {
    matches!(word, "1-0" | "0-1" | "1/2-1/2" | "*")
}

fn split_number(word: &str) -> Result<(Option<NumberPrefix>, &str), String> {
    let digits = word.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return Ok((None, word));
    }
    let dots = word[digits..].bytes().take_while(|b| *b == b'.').count();
    if dots == 0 {
        return Ok((None, word));
    }
    if dots != 1 && dots != 3 {
        return Err(format!("Invalid move number {word}"));
    }
    let number = word[..digits]
        .parse()
        .map_err(|_| format!("Invalid move number {word}"))?;
    Ok((Some(NumberPrefix { number, dots }), &word[digits + dots..]))
}

fn glyph_nag(word: &str) -> Option<u16> {
    match word {
        "!" => Some(1),
        "?" => Some(2),
        "!!" => Some(3),
        "??" => Some(4),
        "!?" => Some(5),
        "?!" => Some(6),
        _ => None,
    }
}

fn split_glyph(word: &str) -> (&str, Option<u16>) {
    for glyph in ["!!", "??", "!?", "?!", "!", "?"] {
        if let Some(san) = word.strip_suffix(glyph) {
            return (san, glyph_nag(glyph));
        }
    }
    (word, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nested_variations_round_trip() {
        let source = "[Event \"Café \\\"night\\\"\"]\n[Result \"*\"]\n[Annotator \"A\\\\B\"]\n\n{start} 1. e4! {king pawn} (1. d4 $2 d5 (1... Nf6 {Indian}) 2. c4) 1... e5 $1 2. Nf3 *";
        let parsed = PgnGame::parse(source).unwrap();
        assert_eq!(parsed.nodes[0].children.len(), 2);
        assert_eq!(parsed.nodes[parsed.nodes[0].children[1]].san, "d4");
        let exported = parsed.export();
        let restored = PgnGame::parse(&exported).unwrap();
        assert_eq!(restored.export(), exported);
        assert_eq!(restored.nodes.len(), parsed.nodes.len());
        for (before, after) in parsed.nodes.iter().zip(&restored.nodes) {
            assert_eq!(before.san, after.san);
            assert_eq!(before.before, after.before);
            assert_eq!(before.after, after.after);
            assert_eq!(before.nags, after.nags);
            assert_eq!(before.children.len(), after.children.len());
        }
        for header in &parsed.headers {
            assert!(restored.headers.contains(header));
        }
        assert!(exported.starts_with("[Event \"Café"));
        assert!(exported.contains("[White \"?\"]"));
    }

    #[test]
    fn black_start_and_invalid_input() {
        let source = "[SetUp \"1\"]\n[FEN \"rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1\"]\n\n1... e5 (1... c5) *";
        let parsed = PgnGame::parse(source).unwrap();
        assert_eq!(parsed.nodes[0].children.len(), 2);
        assert!(PgnGame::parse(&parsed.export()).is_ok());
        for invalid in [
            "1. e5 *",
            "1. e4 (1. e5) *",
            "1. e4 {oops",
            "1. e4 ) *",
            "1. e4 $ *",
            "[FEN \"x\"]\n*",
            "1. e4 *\n1. d4 *",
            "1. e4 $256 *",
            "[Result \"1-0\"]\n\n1. e4 0-1",
        ] {
            assert!(PgnGame::parse(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn semicolon_comment_with_brace_survives_export() {
        let first = PgnGame::parse("1. e4 ; brace } and Unicode ♞\n e5 *").unwrap();
        let exported = first.export();
        assert_eq!(PgnGame::parse(&exported).unwrap().export(), exported);
        assert!(exported.contains("; brace } and Unicode ♞"));
    }

    #[test]
    fn comment_after_move_number_belongs_to_following_move() {
        let game = PgnGame::parse("1. e4 1... {Black considers the reply} e5 *").unwrap();
        let white = game.nodes[0].children[0];
        let black = game.nodes[white].children[0];
        assert!(game.nodes[white].after.is_empty());
        assert_eq!(game.nodes[black].before, ["Black considers the reply"]);
        let exported = game.export();
        assert_eq!(
            PgnGame::parse(&exported).unwrap().nodes[black].before,
            ["Black considers the reply"]
        );
        assert!(PgnGame::parse("1. e4 1... *").is_err());
    }

    #[test]
    fn arbitrary_small_inputs_never_panic_and_successful_parses_round_trip() {
        let alphabet = [
            '1', '.', 'e', '4', ' ', '(', ')', '{', '}', '$', '0', '*', '\n', '♞',
        ];
        let mut state = 0x1234_5678_u64;
        for _ in 0..2_000 {
            let mut input = String::new();
            for _ in 0..48 {
                state = state
                    .wrapping_mul(6_364_136_223_846_793_005)
                    .wrapping_add(1);
                input.push(alphabet[(state as usize) % alphabet.len()]);
            }
            if let Ok(parsed) = PgnGame::parse(&input) {
                assert!(PgnGame::parse(&parsed.export()).is_ok());
            }
        }
    }

    #[test]
    fn header_count_and_control_characters_are_bounded() {
        let headers = (0..257)
            .map(|number| format!("[Tag{number} \"x\"]\n"))
            .collect::<String>();
        assert!(
            PgnGame::parse(&headers)
                .err()
                .unwrap()
                .contains("256 headers")
        );
        assert!(PgnGame::parse("[Event \"bad\nvalue\"]\n*").is_err());
    }

    #[test]
    fn crlf_and_attached_move_numbers_are_accepted() {
        let input = "[Event \"CRLF\"]\r\n[Result \"*\"]\r\n\r\n1.e4 e5\r\n2.Nf3 Nc6 *\r\n";
        let game = PgnGame::parse(input).unwrap();
        assert_eq!(game.nodes.len(), 5);
        assert_eq!(PgnGame::parse(&game.export()).unwrap().nodes.len(), 5);
    }
}
