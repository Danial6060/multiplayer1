//! Networking helpers for the client (UDP Renet/Netcode).
//! Provides a persistent NetClient for handshake, polling, and sending inputs.

use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};
use std::time::{SystemTime, UNIX_EPOCH};

use renet::RenetClient;
use renet_netcode::{ClientAuthentication, NetcodeClientTransport};

pub struct NetClient {
    client: RenetClient,
    transport: NetcodeClientTransport,
}

impl NetClient {
    pub fn connect(server_addr: &str, username: &str) -> Option<Self> {
        let client = RenetClient::new(renet::ConnectionConfig::default());
        let server_addr = match resolve_server_addr(server_addr) {
            Some(a) => a,
            None => { eprintln!("Invalid server address"); return None; }
        };

    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Failed to bind UDP socket: {}", e);
                return None;
        }
    };
    let _ = socket.set_nonblocking(true);

    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("time");
    let auth = ClientAuthentication::Unsecure { protocol_id: 0, client_id: 0, server_addr, user_data: None };
    let transport = match NetcodeClientTransport::new(now, auth, socket) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to create netcode client: {}", e);
                return None;
        }
    };

        let mut nc = NetClient { client, transport };
        // Send Hello
        let payload = serde_json::json!({
            "event": "Hello",
            "body": {"name": username},
        });
        let bytes = serde_json::to_vec(&payload).unwrap();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let _ = nc.transport.update(now, &mut nc.client);
        nc.client
            .send_message(renet::DefaultChannel::ReliableOrdered, bytes);

        Some(nc)
    }

    pub fn poll(&mut self) -> Vec<serde_json::Value> {
        let mut out = Vec::new();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
        let _ = self.transport.update(now, &mut self.client);
        while let Some(bytes) = self.client.receive_message(renet::DefaultChannel::ReliableOrdered) {
            if let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) {
                out.push(value);
            }
        }
        out
    }

    pub fn send_input(&mut self, dx: i32, dy: i32) {
        let msg = serde_json::json!({
            "event": "Input",
            "body": {"dx": dx, "dy": dy},
        });
        if let Ok(bytes) = serde_json::to_vec(&msg) {
            self.client.send_message(renet::DefaultChannel::ReliableOrdered, bytes);
        }
    }
}

fn resolve_server_addr(input: &str) -> Option<SocketAddr> {
    // Try direct SocketAddr parse first
    if let Ok(sa) = input.parse::<SocketAddr>() { return Some(sa); }
    // Try resolving hostnames like "localhost:5000"
    if let Ok(mut iter) = input.to_socket_addrs() { return iter.next(); }
    None
}
