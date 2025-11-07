use server::run_server;

fn main() {
    // Start the MazeWars server on UDP 0.0.0.0:5000 with room for 32 clients.
    run_server(None, Some(32));
}

