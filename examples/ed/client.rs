use lseq::{Id, Node, NodeId};

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::net::{TcpStream, TcpListener};
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;

// client handles editing and updating. Each client is an lseq node.
pub fn run_client(port: String) {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).expect("Could not connect to server");

    // Read our node id.
    let mut buf = [0u8; 4];
    assert_eq!(stream.read(&mut buf).expect("Failed to get node id"), 4);

    let client = Client {
        buffer: Arc::new(Mutex::new(Buffer::new(u32::from_be_bytes(buf)))),
        stream,
    };
    client.run();
}

struct Client {
    buffer: Arc<Mutex<Buffer>>,
    stream: TcpStream,
}

struct Buffer {
    node: Node,
    internal: Vec<(Id, char)>,
}

impl Client {
    fn run(&self) {
        let mut clone_buf = self.buffer.clone();
        thread::spawn(move || {
            loop {
                Client::listen_stdin(clone_buf.clone());
            }
        });

        loop {
            self.listen_server();
        }
    }

    // wait for user changes, update the buffer, and send them to the server
    fn listen_stdin(buf: Arc<Mutex<Buffer>>) {
        let mut buf = buf.lock().unwrap();
        unimplemented!();
    }

    // wait for broadcasts from the server and update the buffer
    fn listen_server(&self) {
        let mut buf = self.buffer.lock().unwrap();
        unimplemented!();
    }

}

impl Buffer {
    fn new(node_number: u32) -> Buffer {
        Buffer {
            node: Node::new(NodeId::new(node_number)),
            internal: Vec::new(),
        }
    }

    fn append(&mut self, s: &str) {
        let mut prev_id = match self.internal.last() {
            Some((id, _)) => id.clone(),
            None => self.node.begin(),
        };
        for c in s.chars() {
            let id = self.node.new_id_with_bounds(&prev_id, &prev_id);
            self.internal.push((id.clone(), c));
            prev_id = id;
        }
    }

    fn insert(&mut self, position: usize, s: &str) {
        if self.internal.len() <= position + 1 {
            self.append(s);
            return;
        }

        let next_id = self.internal[position].0.clone();
        let mut prev = position;
        let begin = self.node.begin();
        for c in s.chars() {
            let prev_id = if prev == 0 {
                &begin
            } else {
                &self.internal[prev - 1].0
            };
            let id = self.node.new_id_with_bounds(&prev_id, &next_id);
            self.internal.insert(prev, (id, c));
            prev += 1;
        }
    }

    fn delete(&mut self, position: usize, len: usize) {
        self.internal.drain(position..position + len);
    }

    fn to_string(&self) -> String {
        let mut result = String::new();
        self.internal.iter().for_each(|(_, c)| result.push(*c));

        result
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn assert_ordered_ids(buf: &Buffer) {
        let node = Node::new(NodeId::new(0));
        let mut prev = &node.begin();
        eprintln!("{:?}", buf.internal);
        for (id, _) in &buf.internal {
            assert!(prev < id);
            prev = id;
        }
    }

    #[test]
    fn test_append() {
        let mut buf = Buffer::new(0);
        buf.append("Hello");
        buf.append(", world!");
        assert_eq!(&buf.to_string(), "Hello, world!");
        assert_ordered_ids(&buf);
    }

    #[test]
    fn test_delete() {
        let mut buf = Buffer::new(0);
        buf.append("Hello, world!");
        buf.delete(5, 7);
        assert_eq!(&buf.to_string(), "Hello!");
        assert_ordered_ids(&buf);
    }

    #[test]
    fn test_insert() {
        let mut buf = Buffer::new(0);
        buf.append("Hello, world!");
        buf.insert(5, " there");
        assert_eq!(&buf.to_string(), "Hello there, world!");
        assert_ordered_ids(&buf);
    }

    #[test]
    fn test_insert_begin() {
        let mut buf = Buffer::new(0);
        buf.append("Hello, world!");
        buf.delete(0, 1);
        buf.insert(0, "Why h");
        assert_eq!(&buf.to_string(), "Why hello, world!");
        assert_ordered_ids(&buf);
    }
}
