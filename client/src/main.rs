// client/src/main.rs
use std::env;
use std::net::UdpSocket;
use std::time::{Duration, Instant};
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};

const SERVER_ADDR: &str = "127.0.0.1:8080";
const RETRY_LIMIT: u8 = 3; // Max number of retries for a message
const RETRY_TIMEOUT: Duration = Duration::from_millis(500); // Timeout for acknowledgment

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <player_number>", args[0]);
        return;
    }

    let player_number: u16 = args[1]
        .parse()
        .expect("Player number must be an integer (1, 2, or 3)");

    if !(1..=3).contains(&player_number) {
        eprintln!("Player number must be 1, 2, or 3");
        return;
    }

    let client_port = 8081 + player_number - 1;
    let client_addr = format!("127.0.0.1:{}", client_port);
    let socket = UdpSocket::bind(&client_addr).expect("Could not bind to address");
    socket.set_read_timeout(Some(RETRY_TIMEOUT)).expect("Failed to set read timeout");

    println!("Player {} bound to {}\n", player_number, client_addr);

    let mut sequence_number = 0;

    // Send a connect message to the server
    sequence_number += 1;
    send_with_retries(&socket, &format!("{}:connect", sequence_number));

    println!("Press W, A, S, or D to move. Press 'Q' to quit.\n");

    enable_raw_mode().expect("Failed to enable raw mode");

    loop {
        if event::poll(Duration::from_millis(100)).unwrap() {
            if let Event::Key(key_event) = event::read().unwrap() {
                match key_event.code {
                    KeyCode::Char('w') | KeyCode::Char('a') | KeyCode::Char('s') | KeyCode::Char('d') => {
                        sequence_number += 1;
                        let message = format!("{}:{}", sequence_number, key_event.code.to_string().to_uppercase());
                        send_with_retries(&socket, &message);
                    }
                    KeyCode::Char('q') => {
                        println!("Quitting client...");
                        break;
                    }
                    _ => {}
                }
            }
        }
    }

    disable_raw_mode().expect("Failed to disable raw mode");
}

/// Sends a message to the server and retries if no acknowledgment is received.
fn send_with_retries(socket: &UdpSocket, message: &str) {
    let mut retries = 0;
    let start = Instant::now();

    while retries < RETRY_LIMIT {
        if let Err(e) = socket.send_to(message.as_bytes(), SERVER_ADDR) {
            eprintln!("Failed to send message: {} (retry {})", e, retries);
        } else {
            // Wait for acknowledgment
            let mut buffer = [0u8; 1024];
            match socket.recv_from(&mut buffer) {
                Ok((size, _src)) => {
                    let response = String::from_utf8_lossy(&buffer[..size]);
                    if response.starts_with("ACK:") {
                        println!("Acknowledgment received for '{}'", message);
                        return;
                    }
                }
                Err(_) => {
                    retries += 1;
                    println!("Retrying '{}'... ({}/{})", message, retries, RETRY_LIMIT);
                }
            }
        }

        if start.elapsed() > RETRY_TIMEOUT * RETRY_LIMIT as u32 {
            eprintln!("Message '{}' failed after {} retries", message, retries);
            break;
        }
    }
}
