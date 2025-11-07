// Gameplay modules removed from compilation to keep a minimal networking skeleton.

use bevy_renet::renet::{ConnectionConfig, DefaultChannel, RenetServer, ServerEvent};
use renet_netcode::{NetcodeServerTransport, ServerAuthentication, ServerConfig};
use local_ip_address::local_ip;

use bevy_ecs::prelude::{EventReader, Res, ResMut, Resource};
use std::collections::{HashMap, HashSet};
use rand::Rng;

/// Server plugin that configures Renet UDP transport and registers systems
pub struct MazeWarsServerPlugin {
    pub bind_addr: String,
    pub max_clients: usize,
}

impl Default for MazeWarsServerPlugin {
    fn default() -> Self {
        Self { bind_addr: "0.0.0.0:5000".to_string(), max_clients: 32 }
    }
}

impl bevy_app::Plugin for MazeWarsServerPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        // Renet core server
        let connection_config = ConnectionConfig::default();
        let server = RenetServer::new(connection_config);

        // UDP netcode transport
        let public_addr = self.bind_addr.parse().expect("Invalid bind address");
        let socket = std::net::UdpSocket::bind(public_addr).expect("Failed to bind UDP socket");
        socket.set_nonblocking(true).expect("Failed to set socket nonblocking");

        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("time");
        let server_config = ServerConfig {
            current_time: now,
            max_clients: self.max_clients,
            protocol_id: 0,
            public_addresses: vec![public_addr],
            authentication: ServerAuthentication::Unsecure,
        };
        let transport = NetcodeServerTransport::new(server_config, socket).expect("Create netcode transport");

        app.insert_resource(server)
            .insert_resource(transport)
            .insert_resource(MaxClients(self.max_clients))
            .insert_resource(init_world())
            .insert_resource(StatsTimer(bevy_time::Timer::from_seconds(1.0, bevy_time::TimerMode::Repeating)))
            .insert_resource(LastClientCount(usize::MAX))
            .add_systems(bevy_app::Startup, print_bind_info)
            .add_systems(bevy_app::Update, (handle_server_events, receive_client_messages, tick_rounds, log_server_stats));
    }
}

#[derive(Resource)]
pub struct MaxClients(pub usize);

fn print_bind_info(bind_info: Option<Res<MaxClients>>) {
    let ip = local_ip().map(|ip| ip.to_string()).unwrap_or_else(|_| "unknown".into());
    let max = bind_info.map(|m| m.0).unwrap_or(32);
    println!("MazeWars server starting on UDP 0.0.0.0:5000 (local IP {ip}) | max_clients={max}");
}

fn handle_server_events(mut server_events: EventReader<ServerEvent>) {
    for event in server_events.read() {
        match event {
            ServerEvent::ClientConnected { client_id } => {
                println!("client {client_id} connected");
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                println!("client {client_id} disconnected: {reason}");
            }
        }
    }
}

fn receive_client_messages(mut server: ResMut<RenetServer>, mut world: ResMut<WorldState>) {
    let client_ids: Vec<u64> = server.clients_id();
    for client_id in client_ids.into_iter() {
        while let Some(bytes) = server.receive_message(client_id, DefaultChannel::ReliableOrdered) {
            let parsed: Result<serde_json::Value, _> = serde_json::from_slice(&bytes);
            let Ok(value) = parsed else { continue };
            let event = value.get("event").and_then(|v| v.as_str()).unwrap_or("");
            let body = value.get("body").cloned().unwrap_or(serde_json::Value::Null);

            match event {
                "Hello" | "hello" => {
                    // Reply with a welcome message so client can confirm connectivity
                    let name = body.get("name").and_then(|v| v.as_str()).unwrap_or("player");
                    let reply = serde_json::json!({
                        "event": "Welcome",
                        "body": { "message": format!("Welcome, {}!", name), "client_id": client_id },
                    });
                    if let Ok(bytes) = serde_json::to_vec(&reply) {
                        server.send_message(client_id, DefaultChannel::ReliableOrdered, bytes);
                    }

                    // Register player if new and spawn at a unique empty cell
                    let (sx, sy) = find_unused_spawn(&world);
                    let newp = Player { id: client_id, x: sx, y: sy };
                    world.players.entry(client_id).or_insert(newp);

                    // Send map snapshot
                    let map_msg = serde_json::json!({
                        "event": "Map",
                        "body": {
                            "width": world.map_width,
                            "height": world.map_height,
                            "cells": world.map_cells,
                        }
                    });
                    if let Ok(bytes) = serde_json::to_vec(&map_msg) {
                        server.send_message(client_id, DefaultChannel::ReliableOrdered, bytes);
                    }

                    // Send player's initial position
                    if let Some(p) = world.players.get(&client_id) {
                        let init_msg = serde_json::json!({
                            "event": "PlayerInit",
                            "body": {"id": p.id, "x": p.x, "y": p.y},
                        });
                        if let Ok(bytes) = serde_json::to_vec(&init_msg) {
                            server.send_message(client_id, DefaultChannel::ReliableOrdered, bytes);
                        }
                    }

                    // Broadcast players snapshot to everyone
                    broadcast_players(&mut server, &world);
                }
                "Input" => {
                    let dx = body.get("dx").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let dy = body.get("dy").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let mw = world.map_width; let mh = world.map_height; let cells_ptr: *const u8 = world.map_cells.as_ptr();
                    if let Some(p) = world.players.get_mut(&client_id) {
                        let nx = (p.x as i32 + dx).max(0) as usize;
                        let ny = (p.y as i32 + dy).max(0) as usize;
                        if nx < mw && ny < mh {
                            let idx = ny * mw + nx;
                            // SAFETY: idx bounds checked; read-only
                            let passable = unsafe { *cells_ptr.add(idx) } == 0;
                            if passable {
                                p.x = nx;
                                p.y = ny;
                                broadcast_players(&mut server, &world);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

#[derive(Resource)]
struct StatsTimer(pub bevy_time::Timer);

#[derive(Resource)]
struct LastClientCount(pub usize);

fn log_server_stats(
    time: Res<bevy_time::Time>,
    mut timer: ResMut<StatsTimer>,
    server: Res<RenetServer>,
    max_clients: Option<Res<MaxClients>>,
    mut last: ResMut<LastClientCount>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        let connected = server.clients_id().len();
        if last.0 != connected {
            last.0 = connected;
            let max = max_clients.map(|m| m.0).unwrap_or(32);
            println!("clients: {connected}/{max}");
        }
    }
}

pub fn run_server(bind_addr: Option<String>, max_clients: Option<usize>) {
    let plugin = MazeWarsServerPlugin { bind_addr: bind_addr.unwrap_or_else(|| "0.0.0.0:5000".into()), max_clients: max_clients.unwrap_or(32) };

    bevy_app::App::new()
        .add_plugins(bevy_app::ScheduleRunnerPlugin::default())
        .add_plugins(bevy_time::TimePlugin::default())
        .add_plugins(bevy_renet::RenetServerPlugin)
        .add_plugins(plugin)
        .run();
}

// ---------------- World / Map -----------------

#[derive(Resource)]
struct WorldState {
    map_width: usize,
    map_height: usize,
    map_cells: Vec<u8>, // 0 passage, 1 wall
    spawn_x: usize,
    spawn_y: usize,
    players: HashMap<u64, Player>,
    round_state: RoundState,
    round_seconds: u32,
    difficulty_idx: usize,
}

struct Player {
    id: u64,
    x: usize,
    y: usize,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum RoundState { Lobby, InRound, Intermission }

fn init_world() -> WorldState {
    let env_diff = std::env::var("MAZE_DIFFICULTY").unwrap_or_else(|_| "medium".to_string());
    let (w, h) = (41usize, 31usize); // odd dims for nice grid
    let mut cells = generate_maze(w, h);
    apply_difficulty(&mut cells, w, h, &env_diff);
    let (sx, sy) = find_spawn(&cells, w, h);
    let idx = match env_diff.as_str() { "easy" => 0, "medium" => 1, _ => 2 };
    WorldState { map_width: w, map_height: h, map_cells: cells, spawn_x: sx, spawn_y: sy, players: HashMap::new(), round_state: RoundState::Lobby, round_seconds: 5, difficulty_idx: idx }
}

fn broadcast_players(server: &mut RenetServer, world: &WorldState) {
    let list: Vec<serde_json::Value> = world
        .players
        .values()
        .map(|p| serde_json::json!({"id": p.id, "x": p.x, "y": p.y}))
        .collect();
    let msg = serde_json::json!({"event": "Players", "body": list});
    if let Ok(bytes) = serde_json::to_vec(&msg) {
        for id in server.clients_id() {
            server.send_message(id, DefaultChannel::ReliableOrdered, bytes.clone());
        }
    }
}

fn broadcast_map(server: &mut RenetServer, world: &WorldState) {
    let map_msg = serde_json::json!({
        "event": "Map",
        "body": {
            "width": world.map_width,
            "height": world.map_height,
            "cells": world.map_cells,
        }
    });
    if let Ok(bytes) = serde_json::to_vec(&map_msg) {
        for id in server.clients_id() {
            server.send_message(id, DefaultChannel::ReliableOrdered, bytes.clone());
        }
    }
}

fn find_spawn(cells: &Vec<u8>, w: usize, h: usize) -> (usize, usize) {
    // pick first passage near center
    let cx = w / 2; let cy = h / 2;
    for r in 0..(w.max(h)) {
        for dy in -(r as isize)..=(r as isize) {
            for dx in -(r as isize)..=(r as isize) {
                let x = cx as isize + dx; let y = cy as isize + dy;
                if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                    let idx = y as usize * w + x as usize;
                    if cells[idx] == 0 { return (x as usize, y as usize); }
                }
            }
        }
    }
    (1,1)
}

fn cell_is_open(cells: &Vec<u8>, w: usize, x: usize, y: usize) -> bool {
    cells[y * w + x] == 0
}

fn is_occupied(players: &HashMap<u64, Player>, x: usize, y: usize) -> bool {
    players.values().any(|p| p.x == x && p.y == y)
}

// Find a spawn near the center that is open and not currently used by any player
fn find_unused_spawn(world: &WorldState) -> (usize, usize) {
    let w = world.map_width; let h = world.map_height;
    let cx = w / 2; let cy = h / 2;
    for r in 0..(w.max(h)) {
        for dy in -(r as isize)..=(r as isize) {
            for dx in -(r as isize)..=(r as isize) {
                let x = cx as isize + dx; let y = cy as isize + dy;
                if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                    let (ux, uy) = (x as usize, y as usize);
                    if cell_is_open(&world.map_cells, w, ux, uy) && !is_occupied(&world.players, ux, uy) {
                        return (ux, uy);
                    }
                }
            }
        }
    }
    (world.spawn_x, world.spawn_y)
}

// Assign unique spawns to all players for a fresh maze
fn allocate_unique_spawns_for_all(players: &mut HashMap<u64, Player>, cells: &Vec<u8>, w: usize, h: usize) {
    let mut used: HashSet<(usize, usize)> = HashSet::new();
    let cx = w / 2; let cy = h / 2;
    // helper to get next available open cell not in `used`
    let next_open = |used: &HashSet<(usize, usize)>| -> (usize, usize) {
        for r in 0..(w.max(h)) {
            for dy in -(r as isize)..=(r as isize) {
                for dx in -(r as isize)..=(r as isize) {
                    let x = cx as isize + dx; let y = cy as isize + dy;
                    if x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h {
                        let (ux, uy) = (x as usize, y as usize);
                        if cells[uy * w + ux] == 0 && !used.contains(&(ux, uy)) {
                            return (ux, uy);
                        }
                    }
                }
            }
        }
        (1,1)
    };

    // Allocate deterministically by sorted client ids
    let mut ids: Vec<u64> = players.keys().copied().collect();
    ids.sort_unstable();
    for id in ids {
        if let Some(p) = players.get_mut(&id) {
            let pos = next_open(&used);
            p.x = pos.0; p.y = pos.1;
            used.insert(pos);
        }
    }
}

fn generate_maze(w: usize, h: usize) -> Vec<u8> {
    // Perfect maze via recursive backtracker on cell grid (odd-sized grid)
    assert!(w % 2 == 1 && h % 2 == 1);
    let cw = (w - 1) / 2; // cells in x
    let ch = (h - 1) / 2; // cells in y
    let mut grid = vec![1u8; w * h]; // 1=wall, 0=passage
    let mut visited = vec![false; cw * ch];
    let mut stack: Vec<(usize, usize)> = Vec::new();
    let mut rng = rand::rng();

    push_cell_fn(&mut grid, &mut visited, w, cw, 0, 0, &mut stack);
    while let Some((cx, cy)) = stack.pop() {
        // collect neighbors
        let mut neighbors = Vec::new();
        if cx > 0 && !visited[cy * cw + (cx - 1)] { neighbors.push((cx - 1, cy, (2*cy+1, 2*cx))); }
        if cy > 0 && !visited[(cy - 1) * cw + cx] { neighbors.push((cx, cy - 1, (2*cy, 2*cx+1))); }
        if cx + 1 < cw && !visited[cy * cw + (cx + 1)] { neighbors.push((cx + 1, cy, (2*cy+1, 2*cx+2))); }
        if cy + 1 < ch && !visited[(cy + 1) * cw + cx] { neighbors.push((cx, cy + 1, (2*cy+2, 2*cx+1))); }

        if !neighbors.is_empty() {
            // push current back to continue later
            stack.push((cx, cy));
            let i: usize = rng.random_range(0..neighbors.len());
            let (nx, ny, (wy, wx)) = neighbors[i];
            // knock down wall between (cx,cy) and (nx,ny)
            grid[wy * w + wx] = 0;
            push_cell_fn(&mut grid, &mut visited, w, cw, nx, ny, &mut stack);
        }
    }

    grid
}

fn push_cell_fn(
    grid: &mut [u8],
    visited: &mut [bool],
    w: usize,
    cw: usize,
    cx: usize,
    cy: usize,
    stack: &mut Vec<(usize, usize)>,
) {
    visited[cy * cw + cx] = true;
    grid[(2 * cy + 1) * w + (2 * cx + 1)] = 0;
    stack.push((cx, cy));
}

fn reduce_dead_ends(cells: &mut Vec<u8>, w: usize, h: usize, ratio: f32) {
    // Connect some dead ends to create loops, reducing dead-end count.
    let mut rng = rand::rng();
    let mut dead_ends: Vec<(usize, usize)> = Vec::new();
    for y in 1..(h - 1) {
        for x in 1..(w - 1) {
            if cells[y * w + x] == 0 {
                let mut walls = 0;
                if cells[(y - 1) * w + x] == 1 { walls += 1; }
                if cells[(y + 1) * w + x] == 1 { walls += 1; }
                if cells[y * w + (x - 1)] == 1 { walls += 1; }
                if cells[y * w + (x + 1)] == 1 { walls += 1; }
                if walls == 3 { dead_ends.push((x, y)); }
            }
        }
    }
    let target = ((dead_ends.len() as f32) * ratio) as usize;
    for _ in 0..target {
        if dead_ends.is_empty() { break; }
    let idx: usize = rng.random_range(0..dead_ends.len());
        let (x, y) = dead_ends.remove(idx);
        // carve a random adjacent wall to open a loop
        let mut candidates = Vec::new();
        if cells[(y - 1) * w + x] == 1 { candidates.push((x, y - 1)); }
        if cells[(y + 1) * w + x] == 1 { candidates.push((x, y + 1)); }
        if cells[y * w + (x - 1)] == 1 { candidates.push((x - 1, y)); }
        if cells[y * w + (x + 1)] == 1 { candidates.push((x + 1, y)); }
        if !candidates.is_empty() {
            let j: usize = rng.random_range(0..candidates.len());
            let (wx, wy) = candidates[j];
            cells[wy * w + wx] = 0;
        }
    }
}

fn apply_difficulty(cells: &mut Vec<u8>, w: usize, h: usize, difficulty: &str) {
    match difficulty {
        "easy" => reduce_dead_ends(cells, w, h, 0.5),
        "medium" => reduce_dead_ends(cells, w, h, 0.2),
        _ => {} // hard = perfect
    }
}

fn tick_rounds(time: Res<bevy_time::Time>, mut server: ResMut<RenetServer>, mut world: ResMut<WorldState>) {
    // Simple state machine:
    // Lobby (5s) -> InRound (30s) -> Intermission (5s) -> next difficulty -> Lobby
    let dt = time.delta_secs();
    if dt <= 0.0 { return; }

    // coarse countdown in whole seconds
    let sub = (dt.ceil() as u32).max(1);
    if world.round_seconds > 0 { world.round_seconds = world.round_seconds.saturating_sub(sub); }

    if world.round_seconds == 0 {
        match world.round_state {
            RoundState::Lobby => {
                world.round_state = RoundState::InRound;
                world.round_seconds = 30;
                broadcast_round(&mut server, &world);
            }
            RoundState::InRound => {
                world.round_state = RoundState::Intermission;
                world.round_seconds = 5;
                broadcast_round(&mut server, &world);
            }
            RoundState::Intermission => {
                // Cycle difficulty and regenerate maze
                world.difficulty_idx = (world.difficulty_idx + 1) % 3;
                let difficulty = match world.difficulty_idx { 0 => "easy", 1 => "medium", _ => "hard" };
                let (w, h) = (world.map_width, world.map_height);
                let mut cells = generate_maze(w, h);
                apply_difficulty(&mut cells, w, h, difficulty);
                world.map_cells = cells;
                (world.spawn_x, world.spawn_y) = find_spawn(&world.map_cells, w, h);
                // Allocate unique spawns for all players on the new maze
                let cells_snapshot = world.map_cells.clone();
                let mut players_tmp = std::mem::take(&mut world.players);
                allocate_unique_spawns_for_all(&mut players_tmp, &cells_snapshot, w, h);
                world.players = players_tmp;
                broadcast_map(&mut server, &world);
                broadcast_players(&mut server, &world);
                world.round_state = RoundState::Lobby;
                world.round_seconds = 5;
                broadcast_round(&mut server, &world);
            }
        }
    }
}

fn broadcast_round(server: &mut RenetServer, world: &WorldState) {
    let difficulty = match world.difficulty_idx { 0 => "easy", 1 => "medium", _ => "hard" };
    let state = match world.round_state { RoundState::Lobby => "Lobby", RoundState::InRound => "InRound", RoundState::Intermission => "Intermission" };
    let msg = serde_json::json!({
        "event": "Round",
        "body": {"state": state, "difficulty": difficulty, "remaining": world.round_seconds}
    });
    if let Ok(bytes) = serde_json::to_vec(&msg) {
        for id in server.clients_id() {
            server.send_message(id, DefaultChannel::ReliableOrdered, bytes.clone());
        }
    }
}
