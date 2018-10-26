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
            stream.write(&self.next_node_id.to_be_bytes()).expect("could not send node id");
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
            let mut buf: Vec<u8> = Vec::new();
            loop {
                let mut size_buf: [u8; 4] = [0; 4];
                stream.read_exact(&mut size_buf).expect("error reading size stream");
                let size = u32::from_le_bytes(size_buf) as usize;
                let mut read = 0;
                loop {
                    let mut cur = [0; 256];
                    let count = stream.read(&mut cur).expect("error reading stream");
                    read += count;
                    buf.extend(&cur[..count]);
                    if read >= size {
                        break;
                    }
                }

                // eprintln!("rebroadcast {}", buf.len());

                let mut streams = all_streams.lock().unwrap();
                streams.iter_mut().for_each(|s| {
                    s.write_all(&size_buf).expect("could not write size to stream");
                    s.write_all(&buf).expect("could not write to stream");
                });
                buf.clear();
            }
        });
    }
}
