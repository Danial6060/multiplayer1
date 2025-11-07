use std::io::{self, Write};
use std::time::{Duration, Instant};

use minifb::{Key, Window, WindowOptions};
use client::net::NetClient;
use client::ui::fps::{FpsCounter, draw_text};

fn main() {
    // Prompt for server and username
    let server_addr = prompt("Enter Server IP:PORT (e.g., 127.0.0.1:5000): ");
    let username = prompt("Enter Name: ");

    println!("Starting (window)… connecting to {} as {}", server_addr, username);
    let mut net = match NetClient::connect(&server_addr, &username) {
        Some(nc) => nc,
        None => {
            println!("Could not connect. Press ENTER to exit…");
            let _ = io::stdin().read_line(&mut String::new());
            return;
        }
    };

    // Wait briefly for Welcome and Map/Init messages
    let mut got_welcome = false;
    let mut map: Option<(usize, usize, Vec<u8>)> = None;
    let mut my_id: Option<u64> = None;
    let mut players: std::collections::HashMap<u64, (i32, i32)> = std::collections::HashMap::new();

    let start_wait = std::time::Instant::now();
    while start_wait.elapsed().as_millis() < 5000 {
        for msg in net.poll() {
            if let Some(ev) = msg.get("event").and_then(|v| v.as_str()) {
                match ev {
                    "Welcome" | "welcome" => { got_welcome = true; }
                    "Map" => {
                        if let Some(body) = msg.get("body") {
                            let w = body.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                            let h = body.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                            if let Some(arr) = body.get("cells").and_then(|v| v.as_array()) {
                                let cells: Vec<u8> = arr.iter().map(|x| x.as_u64().unwrap_or(0) as u8).collect();
                                map = Some((w, h, cells));
                            }
                        }
                    }
                    "PlayerInit" => {
                        if let Some(body) = msg.get("body") {
                            if let Some(id) = body.get("id").and_then(|v| v.as_u64()) { my_id = Some(id); }
                            if let (Some(x), Some(y)) = (body.get("x").and_then(|v| v.as_i64()), body.get("y").and_then(|v| v.as_i64())) {
                                if let Some(id) = my_id { players.insert(id, (x as i32, y as i32)); }
                            }
                        }
                    }
                    "Players" => {
                        if let Some(body) = msg.get("body").and_then(|v| v.as_array()) {
                            for p in body {
                                if let (Some(id), Some(x), Some(y)) = (
                                    p.get("id").and_then(|v| v.as_u64()),
                                    p.get("x").and_then(|v| v.as_i64()),
                                    p.get("y").and_then(|v| v.as_i64()),
                                ) {
                                    players.insert(id, (x as i32, y as i32));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // Open a tiny window to show the maze and FPS in the title
    run_window(&server_addr, &username, got_welcome, &mut net, map, my_id, players);
}

fn prompt(label: &str) -> String {
    print!("{}", label);
    let _ = io::stdout().flush();
    let mut s = String::new();
    io::stdin().read_line(&mut s).expect("failed to read line");
    s.trim().to_string()
}

// Handshake moved to crate::net::handshake

fn run_window(
    server_addr: &str,
    username: &str,
    mut connected: bool,
    net: &mut NetClient,
    mut map: Option<(usize, usize, Vec<u8>)>,
    mut my_id: Option<u64>,
    mut players: std::collections::HashMap<u64, (i32, i32)>,
) {
    const WIDTH: usize = 400;
    const HEIGHT: usize = 300;

    let mut window = Window::new(
        &format!(
            "MazeWars Client — {} | {} | FPS: --",
            if connected { "connected" } else { "no welcome" },
            username
        ),
        WIDTH as usize,
        HEIGHT as usize,
        WindowOptions::default(),
    )
    .expect("Unable to open window");

    let mut buffer = vec![0u32; WIDTH * HEIGHT];
    // Initial draw
    if let Some((mw, mh, cells)) = map.as_ref() {
        draw_maze_into(&mut buffer, WIDTH, HEIGHT, *mw, *mh, cells);
    } else {
        draw_background(&mut buffer, WIDTH, HEIGHT);
    }

    let mut last_title_update = Instant::now();
    let mut fps_counter = FpsCounter::new();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        // Input: WASD -> send Input to server (grid steps)
        let mut dx = 0;
        let mut dy = 0;
        if window.is_key_down(Key::W) { dy -= 1; }
        if window.is_key_down(Key::S) { dy += 1; }
        if window.is_key_down(Key::A) { dx -= 1; }
        if window.is_key_down(Key::D) { dx += 1; }
        if dx != 0 || dy != 0 { net.send_input(dx, dy); }

        // Poll network updates
        for msg in net.poll() {
            if let Some(ev) = msg.get("event").and_then(|v| v.as_str()) {
                match ev {
                    "Welcome" | "welcome" => { connected = true; }
                    "Round" => { /* could show round/difficulty in title next tick */ }
                    "PlayerInit" => {
                        if let Some(body) = msg.get("body") {
                            if let Some(id) = body.get("id").and_then(|v| v.as_u64()) { my_id = Some(id); }
                            if let (Some(x), Some(y)) = (body.get("x").and_then(|v| v.as_i64()), body.get("y").and_then(|v| v.as_i64())) {
                                if let Some(id) = my_id { players.insert(id, (x as i32, y as i32)); }
                            }
                        }
                    }
                    "Players" => {
                        if let Some(body) = msg.get("body").and_then(|v| v.as_array()) {
                            for p in body {
                                if let (Some(id), Some(x), Some(y)) = (
                                    p.get("id").and_then(|v| v.as_u64()),
                                    p.get("x").and_then(|v| v.as_i64()),
                                    p.get("y").and_then(|v| v.as_i64()),
                                ) {
                                    players.insert(id, (x as i32, y as i32));
                                }
                            }
                        }
                    }
                    "Map" => {
                        if let Some(body) = msg.get("body") {
                            let mw = body.get("width").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                            let mh = body.get("height").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                            if let Some(arr) = body.get("cells").and_then(|v| v.as_array()) {
                                let cells: Vec<u8> = arr.iter().map(|x| x.as_u64().unwrap_or(0) as u8).collect();
                                // Redraw base layer immediately
                                draw_maze_into(&mut buffer, WIDTH, HEIGHT, mw, mh, &cells);
                                // Store latest map
                                map = Some((mw, mh, cells));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        // Update FPS counter
        fps_counter.tick();

        // Occasionally update the window title with FPS
        if last_title_update.elapsed() >= Duration::from_millis(250) {
            window.set_title(&format!(
                "MazeWars Client — {} | user: {} | server: {} | FPS: {}",
                if connected { "connected" } else { "no welcome" },
                username,
                server_addr,
                fps_counter.fps
            ));
            last_title_update = Instant::now();
        }

        // Redraw player dots on a copy of the background
        let mut frame = buffer.clone();
        if let Some((mw, mh, _)) = map.as_ref() {
            draw_players(&mut frame, WIDTH, HEIGHT, *mw, *mh, &players, my_id);
        } else {
            draw_players(&mut frame, WIDTH, HEIGHT, 1, 1, &players, my_id);
        }
        // On-screen overlays
    let fg = rgb(0, 255, 128);
    let bg = Some(rgb(0, 0, 0));
    let mut frame = frame; // mutable binding
    draw_text(&mut frame, WIDTH, 6, 6, &format!("FPS:{}", fps_counter.fps), fg, bg);
        if !connected {
            draw_text(&mut frame, WIDTH, 6, 18, "waiting for server...", rgb(220,220,220), Some(rgb(30,30,30)));
        }

    window.update_with_buffer(&frame, WIDTH, HEIGHT).unwrap();
        std::thread::sleep(Duration::from_millis(10)); // ~100 FPS cap, enough to show >50
    }
}

fn draw_background(buf: &mut [u32], w: usize, h: usize) {
    // Fill with dark background
    for px in buf.iter_mut() {
        *px = rgb(16, 16, 20);
    }

    // Draw a simple minimap-like grid
    let cell = 20usize;
    // Light gray lines
    let line = rgb(60, 60, 70);

    for y in (0..h).step_by(cell) {
        draw_hline(buf, w, y, 0, w - 1, line);
    }
    for x in (0..w).step_by(cell) {
        draw_vline(buf, w, x, 0, h - 1, line);
    }

    // No fake player dot here to avoid confusion before connection
}

fn draw_maze_into(buf: &mut [u32], w: usize, h: usize, mw: usize, mh: usize, cells: &Vec<u8>) {
    // Dark background
    for px in buf.iter_mut() { *px = rgb(16, 16, 20); }
    // Compute cell size to fit
    let cw = (w as f32 / mw as f32).floor().max(1.0) as usize;
    let ch = (h as f32 / mh as f32).floor().max(1.0) as usize;

    let wall = rgb(200, 200, 200);
    for y in 0..mh {
        for x in 0..mw {
            if cells[y * mw + x] != 0 {
                // draw filled cell
                let px0 = x * cw;
                let py0 = y * ch;
                for py in py0..(py0 + ch).min(h) {
                    for px in px0..(px0 + cw).min(w) {
                        buf[py * w + px] = wall;
                    }
                }
            }
        }
    }
}

fn draw_players(buf: &mut [u32], w: usize, h: usize, mw: usize, mh: usize, players: &std::collections::HashMap<u64, (i32, i32)>, my_id: Option<u64>) {
    let me_color = rgb(0, 200, 255);
    let cw = (w as f32 / mw as f32).floor().max(1.0) as usize;
    let ch = (h as f32 / mh as f32).floor().max(1.0) as usize;
    for (id, (x, y)) in players {
        let px = (*x as usize) * cw + cw / 2;
        let py = (*y as usize) * ch + ch / 2;
        let color = if Some(*id) == my_id { me_color } else { color_from_id(*id) };
        draw_disc(buf, w, px as isize, py as isize, (cw.min(ch).max(3) / 3) as isize, color);
    }
}

fn color_from_id(id: u64) -> u32 {
    // Deterministic bright color from id using HSV
    let hue = (id % 360) as f32; // 0..360
    let (r, g, b) = hsv_to_rgb(hue, 0.8, 0.95);
    rgb(r, g, b)
}

fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (u8, u8, u8) {
    let c = v * s;
    let hh = (h / 60.0) % 6.0;
    let x = c * (1.0 - ((hh % 2.0) - 1.0).abs());
    let (r1, g1, b1) = if hh < 1.0 {
        (c, x, 0.0)
    } else if hh < 2.0 {
        (x, c, 0.0)
    } else if hh < 3.0 {
        (0.0, c, x)
    } else if hh < 4.0 {
        (0.0, x, c)
    } else if hh < 5.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    let m = v - c;
    let (r, g, b) = (r1 + m, g1 + m, b1 + m);
    ((r * 255.0) as u8, (g * 255.0) as u8, (b * 255.0) as u8)
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
}

fn draw_hline(buf: &mut [u32], w: usize, y: usize, x0: usize, x1: usize, color: u32) {
    if y >= buf.len() / w { return; }
    let row = y * w;
    let x1 = x1.min(w - 1);
    for x in x0..=x1 {
        buf[row + x] = color;
    }
}

fn draw_vline(buf: &mut [u32], w: usize, x: usize, y0: usize, y1: usize, color: u32) {
    if x >= w { return; }
    let y1 = y1.min(buf.len() / w - 1);
    for y in y0..=y1 {
        buf[y * w + x] = color;
    }
}

fn draw_disc(buf: &mut [u32], w: usize, cx: isize, cy: isize, r: isize, color: u32) {
    let r2 = r * r;
    let (w_i, h_i) = (w as isize, (buf.len() / w) as isize);
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy <= r2 {
                let x = cx + dx;
                let y = cy + dy;
                if x >= 0 && y >= 0 && x < w_i && y < h_i {
                    buf[(y as usize) * w + x as usize] = color;
                }
            }
        }
    }
}
