use chess_chaos_ai::perft;
use shakmaty::Chess;
use std::time::Instant;

fn main() {
    for (depth, expected) in [(4, 197_281), (5, 4_865_609)] {
        let start = Instant::now();
        let nodes = perft(&Chess::default(), depth);
        assert_eq!(nodes, expected);
        let elapsed = start.elapsed();
        println!(
            "perft depth {depth}: {nodes} nodes in {:.3} ms ({:.0} nodes/s)",
            elapsed.as_secs_f64() * 1_000.0,
            nodes as f64 / elapsed.as_secs_f64()
        );
    }
}
