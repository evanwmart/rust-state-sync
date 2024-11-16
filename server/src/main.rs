use std::net::UdpSocket;
use std::collections::HashMap;

const SERVER_ADDR: &str = "127.0.0.1:8080";

fn main() {
    let socket = UdpSocket::bind(SERVER_ADDR).expect("Could not bind to address");
    println!("Server listening on {}", SERVER_ADDR);

    let mut buffer = [0u8; 1024];
    let mut player_map: HashMap<u16, u8> = HashMap::new(); // Maps port to player number

    loop {
        match socket.recv_from(&mut buffer) {
            Ok((size, src)) => {
                let port = src.port();
                let message = String::from_utf8_lossy(&buffer[..size]);

                // Determine or assign the player number
                let player_number = *player_map.entry(port).or_insert_with(|| {
                    match port {
                        8081 => 1,
                        8082 => 2,
                        8083 => 3,
                        _ => {
                            println!("Unknown player port: {}", port);
                            0 // Unknown player
                        }
                    }
                });

                if player_number == 0 {
                    println!("Received message '{}' from unknown port: {}", message, port);
                    continue;
                }

                if message == "connect" {
                    println!("P{} connected from {}", player_number, port);
                } else {
                    println!("P{}: {}", player_number, message);
                }
            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
            }
        }
    }
}
