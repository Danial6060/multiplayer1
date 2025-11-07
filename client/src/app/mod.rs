//! Client application skeleton.
//! Extend this module with input handling, state transitions, and render triggers.

#[allow(dead_code)]
pub struct App {
    pub connected: bool,
    pub username: String,
    pub server_addr: String,
}

impl App {
    #[allow(dead_code)]
    pub fn new(server_addr: String, username: String, connected: bool) -> Self {
        Self { connected, username, server_addr }
    }

    #[allow(dead_code)]
    pub fn update(&mut self, _dt: f32) {
        // Update app state, poll network, queue draws, etc.
    }
}
