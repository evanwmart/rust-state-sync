// client/src/main.rs
use std::env;
use std::net::UdpSocket;
use std::time::{Duration, Instant};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};

// Constants as const instead of static
const SERVER_ADDR: &str = "127.0.0.1:8080";
const RETRY_LIMIT: u8 = 3;
const RETRY_TIMEOUT: Duration = Duration::from_millis(500);
const POLL_TIMEOUT: Duration = Duration::from_millis(100);
const BUFFER_SIZE: usize = 1024;

// Custom error type for better error handling
#[derive(Debug)]
enum ClientError {
    NetworkError(std::io::Error),
    InvalidPlayer(String),
    ParseError(std::num::ParseIntError),
}

// Implement std::error::Error for ClientError
impl std::error::Error for ClientError {}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::NetworkError(e) => write!(f, "Network error: {}", e),
            ClientError::InvalidPlayer(msg) => write!(f, "Invalid player: {}", msg),
            ClientError::ParseError(e) => write!(f, "Parse error: {}", e),
        }
    }
}

impl From<std::io::Error> for ClientError {
    fn from(err: std::io::Error) -> Self {
        ClientError::NetworkError(err)
    }
}

impl From<std::num::ParseIntError> for ClientError {
    fn from(err: std::num::ParseIntError) -> Self {
        ClientError::ParseError(err)
    }
}

struct GameClient {
    socket: UdpSocket,
    sequence_number: u32,
    player_number: u16,
}

impl GameClient {
    fn new(player_number: u16) -> Result<Self, ClientError> {
        if !(1..=3).contains(&player_number) {
            return Err(ClientError::InvalidPlayer(
                "Player number must be 1, 2, or 3".to_string(),
            ));
        }

        let client_port = 8081 + player_number - 1;
        let client_addr = format!("127.0.0.1:{}", client_port);
        let socket = UdpSocket::bind(&client_addr)?;
        socket.set_read_timeout(Some(RETRY_TIMEOUT))?;

        println!("Player {} bound to {}\n", player_number, client_addr);

        Ok(Self {
            socket,
            sequence_number: 0,
            player_number,
        })
    }

    fn send_with_retries(&mut self, message_type: &str) -> Result<(), std::io::Error> {
        self.sequence_number = self.sequence_number.wrapping_add(1);
        let message = format!("{}:{}", self.sequence_number, message_type);
        let message_bytes = message.as_bytes();
        let start = Instant::now();
        let mut retries = 0;

        while retries < RETRY_LIMIT {
            self.socket.send_to(message_bytes, SERVER_ADDR)?;

            // Use stack-allocated buffer
            let mut buffer = [0u8; BUFFER_SIZE];
            match self.socket.recv_from(&mut buffer) {
                Ok((size, _)) => {
                    if let Ok(response) = std::str::from_utf8(&buffer[..size]) {
                        if response.starts_with("ACK:") {
                            println!("Acknowledgment received for '{}'", message);
                            return Ok(());
                        }
                    }
                }
                Err(_) => {
                    retries += 1;
                    if retries < RETRY_LIMIT {
                        println!("Retrying '{}'... ({}/{})", message, retries, RETRY_LIMIT);
                    }
                }
            }

            if start.elapsed() > RETRY_TIMEOUT * RETRY_LIMIT as u32 {
                eprintln!("Message '{}' failed after {} retries", message, retries);
                break;
            }
        }
        Ok(())
    }

    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Send initial connect message
        self.send_with_retries("connect")?;
        println!("Press W, A, S, or D to move. Press 'Q' to quit.\n");

        enable_raw_mode()?;

        'game_loop: loop {
            if event::poll(POLL_TIMEOUT)? {
                if let Event::Key(key_event) = event::read()? {
                    match key_event.code {
                        KeyCode::Char('q') => {
                            println!("Quitting client...");
                            break 'game_loop;
                        }
                        KeyCode::Char(c) if "wasd".contains(c) => {
                            if let Err(e) = self.send_with_retries(&c.to_uppercase().to_string()) {
                                eprintln!("Failed to send movement: {}", e);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        disable_raw_mode()?;
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