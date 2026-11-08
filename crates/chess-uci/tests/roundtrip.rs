use chess_chaos_uci::analyze;
use std::path::Path;

#[test]
fn subprocess_handshake_and_analysis() {
    let mock = Path::new(env!("CARGO_BIN_EXE_mock-uci"));
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let result = analyze(mock, fen, 100).unwrap();
    assert_eq!(result.engine, "Chess Chaos Test Engine");
    assert_eq!(result.best_move, "e2e4");
    assert_eq!(result.score_cp, Some(25));
    assert_eq!(result.nodes, Some(100));
    let black_fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1";
    assert_eq!(analyze(mock, black_fen, 100).unwrap().best_move, "e7e5");
}
