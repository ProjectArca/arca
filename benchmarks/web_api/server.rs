// Rust web server benchmark using std::net only
// Run: rustc -O -o server server.rs && ./server

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn handle_client(mut stream: TcpStream) {
    let mut buf = [0; 4096];
    if stream.read(&mut buf).is_ok() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 20\r\nConnection: close\r\n\r\n{\"message\": \"hello\"}";
        let _ = stream.write_all(response);
    }
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:3000").unwrap();
    println!("Rust server listening on 0.0.0.0:3000");
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => { thread::spawn(|| handle_client(stream)); }
            Err(_) => {}
        }
    }
}
