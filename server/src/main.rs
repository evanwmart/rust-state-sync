// server/src/main.rs
use std::net::UdpSocket;
use std::collections::HashMap;

const SERVER_ADDR: &str = "127.0.0.1:8080";

fn main() {
    let socket = UdpSocket::bind(SERVER_ADDR).expect("Could not bind to address");
    println!("Server listening on {}", SERVER_ADDR);

    let mut buffer = [0u8; 1024];
    let mut last_sequence: HashMap<u16, u32> = HashMap::new(); // Maps port to last received sequence number

    loop {
        match socket.recv_from(&mut buffer) {
            Ok((size, src)) => {
                let port = src.port();
                let message = String::from_utf8_lossy(&buffer[..size]);

                if message == "connect" {
                    println!("{} connected", port);
                    continue;
                }

                let parts: Vec<&str> = message.split(':').collect();

                if parts.len() != 2 {
                    println!("Malformed message from {}: '{}'", port, message);
                    continue;
                }

                let sequence: u32 = match parts[0].parse() {
                    Ok(seq) => seq,
                    Err(_) => {
                        println!("Invalid sequence number from {}: '{}'", port, parts[0]);
                        continue;
                    }
                };

                let content = parts[1];
                let last_seq = last_sequence.entry(port).or_insert(0);

                if sequence <= *last_seq {
                    println!("Out-of-order or duplicate packet from {}: sequence {}", port, sequence);
                } else {
                    *last_seq = sequence;
                    println!("{}:{}", port, content);

                    // Send an acknowledgment back to the client
                    let ack_message = format!("ACK:{}", sequence);
                    if let Err(e) = socket.send_to(ack_message.as_bytes(), src) {
                        eprintln!("Failed to send ACK to {}: {}", src, e);
                    }
                }
            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
            }
        }
    }
}
