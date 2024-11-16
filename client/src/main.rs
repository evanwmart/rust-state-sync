use std::env;
use std::net::UdpSocket;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode},
};

const SERVER_ADDR: &str = "127.0.0.1:8080";

fn main() {
    // Read the player number from command-line arguments
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

    // Bind the client to a port based on the player number
    let client_port = 8081 + player_number - 1;
    let client_addr = format!("127.0.0.1:{}", client_port);
    let socket = UdpSocket::bind(&client_addr).expect("Could not bind to address");

    println!("Player {} bound to {}\n", player_number, client_addr);

    // Send a connect message to the server
    socket
        .send_to("connect".as_bytes(), SERVER_ADDR)
        .expect("Failed to send connect message");

    println!("Press W, A, S, or D to move. Press 'Q' to quit.\n");

    // Enable raw mode to capture keypresses immediately
    enable_raw_mode().expect("Failed to enable raw mode");

    loop {
        if event::poll(std::time::Duration::from_millis(100)).unwrap() {
            if let Event::Key(key_event) = event::read().unwrap() {
                match key_event.code {
                    KeyCode::Char('w') | KeyCode::Char('a') | KeyCode::Char('s') | KeyCode::Char('d') => {
                        let message = key_event.code.to_string().to_uppercase();
                        if let Err(e) = socket.send_to(message.as_bytes(), SERVER_ADDR) {
                            eprintln!("Failed to send message: {}", e);
                        }
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

    // Restore terminal mode when exiting
    disable_raw_mode().expect("Failed to disable raw mode");
}
