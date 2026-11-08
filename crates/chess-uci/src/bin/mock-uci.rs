use std::io::{self, BufRead, Write};

// Test fixture only: a deterministic UCI-speaking subprocess.
fn main() {
    let mut black_to_move = false;
    for line in io::stdin().lock().lines() {
        match line.as_deref() {
            Ok("uci") => {
                println!("id name Chess Chaos Test Engine");
                println!("uciok");
            }
            Ok("isready") => println!("readyok"),
            Ok(command) if command.starts_with("position fen ") => {
                black_to_move = command.contains(" b ");
            }
            Ok(command) if command.starts_with("go ") => {
                println!("info depth 4 score cp 25 nodes 100 pv e2e4 e7e5");
                println!("bestmove {}", if black_to_move { "e7e5" } else { "e2e4" });
                let _ = io::stdout().flush();
                break;
            }
            Err(_) => break,
            _ => {}
        }
        let _ = io::stdout().flush();
    }
}
