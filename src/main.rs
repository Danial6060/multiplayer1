use std::io::{self, Write};
use std::process::{Command, Stdio};

fn main() {
    println!("MazeWars launcher\n1) Server\n2) Client\n");
    let choice = prompt("Select [1/2]: ");
    match choice.as_str() {
        "1" => run_server(),
        "2" => run_client(),
        _ => {
            eprintln!("Unknown selection. Enter 1 for Server or 2 for Client.");
        }
    }
}

fn run_server() {
    let addr = prompt("Bind address (default 0.0.0.0:5000): ");
    let max = prompt("Max clients (default 32): ");
    let bind = if addr.trim().is_empty() { None } else { Some(addr.trim().to_string()) };
    let maxn = if max.trim().is_empty() { None } else { max.trim().parse::<usize>().ok() };
    server::run_server(bind, maxn);
}

fn run_client() {
    // Spawn `cargo run -p client` as a child process, inheriting stdio.
    // This keeps client logic in the client crate (no duplication here).
    let status = Command::new("cargo")
        .args(["run", "-p", "client"])
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status();
    match status {
        Ok(s) if s.success() => {}
        Ok(s) => eprintln!("client exited with status: {s}"),
        Err(e) => eprintln!("failed to launch client: {e}"),
    }
}

fn prompt(label: &str) -> String {
    print!("{}", label);
    let _ = io::stdout().flush();
    let mut s = String::new();
    io::stdin().read_line(&mut s).ok();
    s.trim().to_string()
}
