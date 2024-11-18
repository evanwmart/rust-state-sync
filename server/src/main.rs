// server/src/main.rs
use std::net::{SocketAddr, UdpSocket};
use std::collections::HashMap;
use std::time::Instant;

const SERVER_ADDR: &str = "127.0.0.1:8080";
const BUFFER_SIZE: usize = 1024;
const MAX_CLIENTS: usize = 3;

#[derive(Debug)]
struct Client {
    last_sequence: u32,
    last_seen: Instant,
    addr: SocketAddr,
}

struct GameState {
    grid_width: u8,
    grid_height: u8,
    players: HashMap<u16, PlayerState>,
    treasures: Vec<(u8, u8)>,
    traps: Vec<(u8, u8)>,
    time_remaining: u32,
}

struct PlayerState {
    x: u8,
    y: u8,
    score: i32,
}

struct GameServer {
    socket: UdpSocket,
    clients: HashMap<u16, Client>,
    game_state: GameState,
    buffer: [u8; BUFFER_SIZE],
}

impl GameServer {
    fn new() -> Self {
        let socket = UdpSocket::bind(SERVER_ADDR).unwrap();
        println!("Server listening on {}", SERVER_ADDR);

        let game_state = GameState {
            grid_width: 10,
            grid_height: 10,
            players: HashMap::new(),
            treasures: vec![(2, 3), (5, 5)],
            traps: vec![(3, 3), (6, 6)],
            time_remaining: 60,
        };

        Self {
            socket,
            clients: HashMap::with_capacity(MAX_CLIENTS),
            game_state,
            buffer: [0u8; BUFFER_SIZE],
        }
    }

    fn handle_connect(&mut self, addr: SocketAddr) {
        let port = addr.port();
        if self.clients.len() < MAX_CLIENTS {
            self.clients.insert(
                port,
                Client {
                    last_sequence: 0,
                    last_seen: Instant::now(),
                    addr,
                },
            );

            self.game_state.players.insert(
                port,
                PlayerState {
                    x: 0,
                    y: 0,
                    score: 0,
                },
            );

            println!("Client connected: {}", port);
            self.socket.send_to(b"ACK:connect", addr).unwrap();
        }
    }

    fn handle_message(&mut self, message: &str, addr: SocketAddr) {
        let port = addr.port();
        println!("Received message from {}: {}", port, message);

        if message.contains(":connect") {
            self.handle_connect(addr);
            return;
        }

        if let Some((sequence, content)) = self.parse_message(message) {
            // Respond with acknowledgment
            let ack_message = format!("ACK:{}", sequence);
            self.socket.send_to(ack_message.as_bytes(), addr).unwrap();

            // Handle move commands
            if content.starts_with("MOVE:") {
                if let Some(player) = self.game_state.players.get_mut(&port) {
                    let direction = content[5..].to_string();
                    match direction.as_str() {
                        "W" => if player.y > 0 { player.y -= 1 },
                        "A" => if player.x > 0 { player.x -= 1 },
                        "S" => if player.y < self.game_state.grid_height - 1 { player.y += 1 },
                        "D" => if player.x < self.game_state.grid_width - 1 { player.x += 1 },
                        _ => {}
                    }

                    if self.game_state.treasures.contains(&(player.x, player.y)) {
                        player.score += 10;
                        self.game_state.treasures.retain(|&pos| pos != (player.x, player.y));
                    }

                    if self.game_state.traps.contains(&(player.x, player.y)) {
                        player.score = 0;
                        self.game_state.traps.retain(|&pos| pos != (player.x, player.y));
                    }
                }
            }
        }
    }

    fn parse_message(&self, message: &str) -> Option<(u32, String)> {
        let parts: Vec<&str> = message.splitn(2, ':').collect();
        if parts.len() == 2 {
            if let Ok(sequence) = parts[0].parse::<u32>() {
                return Some((sequence, parts[1].to_string()));
            }
        }
        None
    }

    fn broadcast_game_state(&self) {
        let game_state = format!(
            "GAME_STATE|TIME:{}|{}|TREASURES:{}|TRAPS:{}",
            self.game_state.time_remaining,
            self.game_state
                .players
                .iter()
                .map(|(id, player)| format!("P{}:({}, {}, {})", id, player.x, player.y, player.score))
                .collect::<Vec<_>>()
                .join("|"),
            self.game_state
                .treasures
                .iter()
                .map(|(x, y)| format!("({}, {})", x, y))
                .collect::<Vec<_>>()
                .join(","),
            self.game_state
                .traps
                .iter()
                .map(|(x, y)| format!("({}, {})", x, y))
                .collect::<Vec<_>>()
                .join(",")
        );

        for client in self.clients.values() {
            self.socket.send_to(game_state.as_bytes(), client.addr).unwrap();
        }

        println!("Sent GS: {}", game_state);
    }

    fn run(&mut self) {
        loop {
            let (size, src) = self.socket.recv_from(&mut self.buffer).unwrap();
            let message = String::from_utf8_lossy(&self.buffer[..size]).to_string(); // Extract message here
            self.handle_message(&message, src);
            self.broadcast_game_state();
        }
    }
}

fn main() {
    let mut server = GameServer::new();
    server.run();
}
