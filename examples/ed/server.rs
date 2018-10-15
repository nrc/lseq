use std::net::{TcpStream, TcpListener};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

// The server assigns node ids and broadcasts messages to all clients.
pub fn run_server() {
    let mut server = Server::new();
    server.handle_requests();
}

const PORT: &str = "7878";

struct Server {
    streams: Arc<Mutex<Vec<TcpStream>>>,
    next_node_id: u32,
}

impl Server {
    fn new() -> Server {
        Server {
            streams: Arc::new(Mutex::new(Vec::new())),
            next_node_id: 1,
        }
    }

    fn handle_requests(&mut self) {
        let addr = format!("127.0.0.1:{}", PORT);
        let listener = TcpListener::bind(&addr).expect("Could not listen for connections");
        println!("Listening on `{}`", addr);

        for stream in listener.incoming() {
            let mut stream = stream.expect("bad stream");
            // Send the node id.
            stream.write(&self.next_node_id.to_be_bytes());
            // Save the stream.
            self.next_node_id += 1;
            {
                let mut streams = self.streams.lock().unwrap();
                streams.push(stream.try_clone().expect("Couldn't clone stream"));
            }
            self.handle_client(stream);
        }
    }

    fn handle_client(&mut self, mut stream: TcpStream) {
        let all_streams = self.streams.clone();
        thread::spawn(move || {
            // listen to client, re-broadcast any changes.
            // unimplemented!();
        });
    }
}
