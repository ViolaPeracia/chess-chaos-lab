use std::{
    io::{BufRead, BufReader, Write},
    path::Path,
    process::{Command, Stdio},
    sync::mpsc::{self, Receiver},
    thread,
    time::Duration,
};

#[derive(Debug, Clone)]
pub struct Analysis {
    pub engine: String,
    pub best_move: String,
    pub depth: Option<u32>,
    pub score_cp: Option<i32>,
    pub nodes: Option<u64>,
    pub pv: Vec<String>,
}

pub fn analyze(path: &Path, fen: &str, move_time_ms: u64) -> Result<Analysis, String> {
    if !path.is_file() {
        return Err("Engine executable does not exist".into());
    }
    if move_time_ms == 0 || move_time_ms > 60_000 {
        return Err("Move time must be 1–60000 ms".into());
    }
    if fen.len() > 256 || fen.contains('\n') || fen.contains('\r') {
        return Err("FEN is too long or contains a newline".into());
    }
    let mut child = Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("Could not start engine: {e}"))?;
    let stdout = child.stdout.take().ok_or("Missing engine stdout")?;
    let (tx, rx) = mpsc::sync_channel(256);
    thread::spawn(move || {
        for line in BufReader::new(stdout).lines() {
            match line {
                Ok(line) => {
                    if tx.send(line).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    let result = (|| {
        let mut stdin = child.stdin.take().ok_or("Missing engine stdin")?;
        writeln!(stdin, "uci").map_err(|e| e.to_string())?;
        stdin.flush().map_err(|e| e.to_string())?;
        let mut engine = path.file_name().map_or("UCI engine".to_string(), |n| {
            n.to_string_lossy().into_owned()
        });
        wait_for(&rx, "uciok", Duration::from_secs(5), |line| {
            if let Some(name) = line.strip_prefix("id name ") {
                engine = name.to_string();
            }
        })?;
        writeln!(stdin, "isready").map_err(|e| e.to_string())?;
        stdin.flush().map_err(|e| e.to_string())?;
        wait_for(&rx, "readyok", Duration::from_secs(5), |_| {})?;
        writeln!(stdin, "position fen {fen}").map_err(|e| e.to_string())?;
        writeln!(stdin, "go movetime {move_time_ms}").map_err(|e| e.to_string())?;
        stdin.flush().map_err(|e| e.to_string())?;
        let mut analysis = Analysis {
            engine,
            best_move: String::new(),
            depth: None,
            score_cp: None,
            nodes: None,
            pv: Vec::new(),
        };
        wait_for(
            &rx,
            "bestmove ",
            Duration::from_millis(move_time_ms + 5_000),
            |line| {
                if line.starts_with("info ") {
                    parse_info(line, &mut analysis);
                }
                if let Some(rest) = line.strip_prefix("bestmove ") {
                    analysis.best_move = rest.split_whitespace().next().unwrap_or("").to_string();
                }
            },
        )?;
        if analysis.best_move.is_empty() || analysis.best_move == "(none)" {
            return Err("Engine returned no move".into());
        }
        Ok(analysis)
    })();
    let _ = child.kill();
    let _ = child.wait();
    result
}

fn wait_for(
    rx: &Receiver<String>,
    marker: &str,
    timeout: Duration,
    mut observe: impl FnMut(&str),
) -> Result<(), String> {
    let deadline = std::time::Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Err(format!("Engine timed out waiting for {marker}"));
        }
        let line = rx
            .recv_timeout(remaining)
            .map_err(|_| format!("Engine stopped or timed out waiting for {marker}"))?;
        observe(&line);
        if line.starts_with(marker) {
            return Ok(());
        }
    }
}

fn parse_info(line: &str, result: &mut Analysis) {
    let parts: Vec<&str> = line.split_whitespace().collect();
    for pair in parts.windows(2) {
        match pair[0] {
            "depth" => result.depth = pair[1].parse().ok(),
            "nodes" => result.nodes = pair[1].parse().ok(),
            "pv" => {
                result.pv = parts
                    .iter()
                    .skip_while(|p| **p != "pv")
                    .skip(1)
                    .map(|s| (*s).to_string())
                    .collect()
            }
            _ => {}
        }
    }
    for triple in parts.windows(3) {
        if triple[0] == "score" && triple[1] == "cp" {
            result.score_cp = triple[2].parse().ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_info() {
        let mut a = Analysis {
            engine: String::new(),
            best_move: String::new(),
            depth: None,
            score_cp: None,
            nodes: None,
            pv: Vec::new(),
        };
        parse_info("info depth 12 score cp -34 nodes 900 pv e2e4 e7e5", &mut a);
        assert_eq!(
            (a.depth, a.score_cp, a.nodes),
            (Some(12), Some(-34), Some(900))
        );
        assert_eq!(a.pv, ["e2e4", "e7e5"]);
    }
}
