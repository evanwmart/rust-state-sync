// client/src/main.rs
use std::collections::{HashMap, HashSet};
use std::env;
use std::net::UdpSocket;
use std::time::Duration;
use std::io::{stdout, Write};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
    terminal::{disable_raw_mode, enable_raw_mode, Clear, ClearType},
    cursor::{Hide, Show, MoveTo},
};


// Constants
const SERVER_ADDR: &str = "127.0.0.1:8080";
const RETRY_TIMEOUT: Duration = Duration::from_millis(500);
const POLL_TIMEOUT: Duration = Duration::from_millis(100);
const BUFFER_SIZE: usize = 1024;

// Struct to represent player state
#[derive(Debug, Clone)]
struct PlayerState {
    x: u8,
    y: u8,
    score: i32,
}

struct GameClient {
    socket: UdpSocket,
    sequence_number: u32,
    acked_messages: HashSet<u32>, // Track acknowledged messages
    grid_width: u8,
    grid_height: u8,
    player_state: PlayerState,
    players: HashMap<u16, PlayerState>,
    treasures: Vec<(u8, u8)>,
    traps: Vec<(u8, u8)>,
    time_remaining: u32,
}

impl GameClient {
    fn new(player_number: u16) -> Result<Self, Box<dyn std::error::Error>> {
        if !(1..=3).contains(&player_number) {
            return Err(format!("Player number must be 1, 2, or 3").into());
        }

        let client_port = 8081 + player_number - 1;
        let client_addr = format!("127.0.0.1:{}", client_port);
        let socket = UdpSocket::bind(&client_addr)?;
        socket.set_read_timeout(Some(RETRY_TIMEOUT))?;
        println!("Player {} bound to {}\n", player_number, client_addr);

        Ok(Self {
            socket,
            sequence_number: 0,
            acked_messages: HashSet::new(),
            grid_width: 10,
            grid_height: 10,
            player_state: PlayerState { x: 0, y: 0, score: 0 },
            players: HashMap::new(),
            treasures: vec![],
            traps: vec![],
            time_remaining: 60,
        })
    }

    fn send_message(&mut self, message: &str) -> Result<(), std::io::Error> {
        self.sequence_number = self.sequence_number.wrapping_add(1);
        let packet = format!("{}:{}", self.sequence_number, message);
        self.socket.send_to(packet.as_bytes(), SERVER_ADDR)?;
        Ok(())
    }

    fn receive_updates(&mut self) -> Result<(), std::io::Error> {
        let mut buffer = [0u8; BUFFER_SIZE];
        if let Ok((size, _)) = self.socket.recv_from(&mut buffer) {
            let response = std::str::from_utf8(&buffer[..size]).unwrap_or("");

            if response.starts_with("ACK:") {
                if let Ok(seq) = response[4..].parse::<u32>() {
                    self.acked_messages.insert(seq);
                }
            } else if response.starts_with("GAME_STATE|") {
                self.parse_game_state(response);
            }
        }
        Ok(())
    }

    fn parse_game_state(&mut self, state: &str) {
        let parts: Vec<&str> = state.split('|').collect();
        if parts.len() < 4 {
            return;
        }
    
        // Parse time remaining
        if let Ok(time_remaining) = parts[1].split(':').nth(1).unwrap_or("0").parse::<u32>() {
            self.time_remaining = time_remaining;
        }
    
        // Clear and rebuild players
        self.players.clear();
        for player_str in parts[2].split('|') {
            if player_str.starts_with("P") {
                let player_data: Vec<&str> = player_str[1..].split(':').collect();
                if let (Some(id), Some(pos)) = (player_data.get(0), player_data.get(1)) {
                    let id = id.parse::<u16>().unwrap_or(0);
                    let coords: Vec<&str> = pos[1..pos.len() - 1].split(',').collect();
                    if coords.len() == 3 {
                        let x = coords[0].trim().parse::<u8>().unwrap_or(0);
                        let y = coords[1].trim().parse::<u8>().unwrap_or(0);
                        let score = coords[2].trim().parse::<i32>().unwrap_or(0);
                        let player_state = PlayerState { x, y, score };
    
                        // Add player to players list
                        self.players.insert(id, player_state.clone());
    
                        // Update local player state if the ID matches
                        if id == self.socket.local_addr().unwrap().port() as u16 {
                            self.player_state = player_state;
                        }
                    }
                }
            }
        }
    
        // Parse treasures
        self.treasures = parts[3]
            .split(',')
            .filter_map(|t| {
                let coords: Vec<&str> = t[1..t.len() - 1].split(',').collect();
                if coords.len() == 2 {
                    Some((
                        coords[0].trim().parse::<u8>().unwrap_or(0),
                        coords[1].trim().parse::<u8>().unwrap_or(0),
                    ))
                } else {
                    None
                }
            })
            .collect();
    
        // Parse traps
        self.traps = parts[4]
            .split(',')
            .filter_map(|t| {
                let coords: Vec<&str> = t[1..t.len() - 1].split(',').collect();
                if coords.len() == 2 {
                    Some((
                        coords[0].trim().parse::<u8>().unwrap_or(0),
                        coords[1].trim().parse::<u8>().unwrap_or(0),
                    ))
                } else {
                    None
                }
            })
            .collect();
    }
    
    fn display_game_state(&self) {
        let mut stdout = stdout();
        execute!(stdout, MoveTo(0, 0), Clear(ClearType::All)).unwrap();
    
        // Display header
        writeln!(
            stdout,
            "Time Remaining: {:<5} Your Score: {}",
            self.time_remaining, self.player_state.score
        )
        .unwrap();
    
        // Center grid in terminal
        let terminal_width = crossterm::terminal::size().unwrap().0;
        let grid_width = self.grid_width as u16 * 2; // Each grid cell is 2 characters wide
        let grid_offset = (terminal_width.saturating_sub(grid_width)) / 2;
    
        // Render grid
        for y in 0..self.grid_height {
            execute!(stdout, MoveTo(grid_offset, (y + 2) as u16)).unwrap(); // Offset for header
            for x in 0..self.grid_width {
                let symbol = if self.player_state.x == x && self.player_state.y == y {
                    "P" // Local player
                } else if self
                    .players
                    .values()
                    .any(|player| player.x == x && player.y == y)
                {
                    "O" // Other players
                } else if self.treasures.contains(&(x, y)) {
                    "T" // Treasure
                } else if self.traps.contains(&(x, y)) {
                    "X" // Trap
                } else {
                    "." // Empty cell
                };
                write!(stdout, "{} ", symbol).unwrap(); // Add space for better readability
            }
        }
        stdout.flush().unwrap();
    }

    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.send_message("connect")?;
        println!("Press W, A, S, or D to move. Press 'Q' to quit.\n");

        let mut stdout = stdout();
        execute!(stdout, Hide).unwrap(); // Hide cursor
        enable_raw_mode()?;

        loop {
            self.receive_updates()?;

            if event::poll(POLL_TIMEOUT)? {
                if let Event::Key(key_event) = event::read()? {
                    match key_event.code {
                        KeyCode::Char('q') => {
                            println!("Quitting client...");
                            break;
                        }
                        KeyCode::Char(c) if "wasd".contains(c) => {
                            let direction = match c {
                                'w' => "MOVE:W",
                                'a' => "MOVE:A",
                                's' => "MOVE:S",
                                'd' => "MOVE:D",
                                _ => "",
                            };

                            if let Err(e) = self.send_message(direction) {
                                eprintln!("Failed to send movement: {}", e);
                            }
                        }
                        _ => {}
                    }
                }
            }

            self.display_game_state();
        }

        disable_raw_mode()?;
        execute!(stdout, Show).unwrap(); // Show cursor
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <player_number>", args[0]);
        std::process::exit(1);
    }

    let player_number: u16 = args[1].parse()?;
    let mut client = GameClient::new(player_number)?;
    client.run()?;

    Ok(())
}
