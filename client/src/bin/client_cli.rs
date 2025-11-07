use std::io::{self, Write};
use std::net::UdpSocket;

use renet::RenetClient;
use renet_netcode::{ClientAuthentication, NetcodeClientTransport};

fn main() {
    // Prompt for server and username
    let server_addr = prompt("Enter Server IP:PORT (e.g., 127.0.0.1:5000): ");
    let username = prompt("Enter Name: ");

    println!("\nClient Menu:\n  [1] Start Game\n  [2] Exit");
    print!("Select: ");
    let _ = io::stdout().flush();

    let mut choice = String::new();
    let _ = io::stdin().read_line(&mut choice);

    match choice.trim() {
        "1" => run_client(&server_addr, &username),
        _ => {
            println!("Bye");
        }
    }
}

fn prompt(label: &str) -> String {
    print!("{}", label);
    let _ = io::stdout().flush();
    let mut s = String::new();
    io::stdin().read_line(&mut s).expect("failed to read line");
    s.trim().to_string()
}

fn run_client(server_addr: &str, username: &str) {
    println!("Starting (CLI)… connecting to {} as {}", server_addr, username);

    // Create Renet client + UDP transport (nonblocking)
    let client = RenetClient::new(renet::ConnectionConfig::default());
    let server_addr = server_addr.parse().expect("Invalid server address");

    let socket = UdpSocket::bind("0.0.0.0:0").expect("bind UDP");
    socket.set_nonblocking(true).expect("nonblocking");

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time");

    let auth = ClientAuthentication::Unsecure { protocol_id: 0, client_id: 0, server_addr, user_data: None };
    let transport = NetcodeClientTransport::new(now, auth, socket).expect("netcode client");

    // Send hello right away
    let payload = serde_json::json!({
        "event": "Hello",
        "body": {"name": username},
    });
    let bytes = serde_json::to_vec(&payload).unwrap();
    let mut client = client;
    let mut transport = transport;

    // First update to kick off connection
    let mut now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let _ = transport.update(now, &mut client);
    client.send_message(renet::DefaultChannel::ReliableOrdered, bytes);

    println!("Hello sent. Waiting up to 5s for Welcome…");

    let start = std::time::Instant::now();
    let mut got_welcome = false;

    loop {
        now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap();
        let _ = transport.update(now, &mut client);

        while let Some(bytes) = client.receive_message(renet::DefaultChannel::ReliableOrdered) {
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                if let Some(event) = value.get("event").and_then(|v| v.as_str()) {
                    println!("[client] received event: {} -> {}", event, value);
                    if event.eq_ignore_ascii_case("welcome") { got_welcome = true; }
                }
            }
        }

        if got_welcome || start.elapsed().as_secs_f32() > 5.0 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(16));
    }

    if !got_welcome {
        println!("No Welcome received. The server may be unreachable or busy. You can still proceed to implement gameplay.");
    }

    println!("Press ENTER to exit…");
    let _ = io::stdin().read_line(&mut String::new());
}
