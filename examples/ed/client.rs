extern crate serde_derive;
extern crate bincode;

use bincode::{serialize, deserialize};
use lseq::{Id, Node, NodeId};
use serde_derive::{Serialize, Deserialize};

use std::net::TcpStream;
use std::io::{Read, Write, stdin, stdout};
use std::process::exit;
use std::sync::{Arc, Mutex};
use std::thread;

// client handles editing and updating. Each client is an lseq node.
pub fn run_client(port: String) {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{}", port)).expect("Could not connect to server");

    // Read our node id.
    let mut buf = [0u8; 4];
    assert_eq!(stream.read(&mut buf).expect("Failed to get node id"), 4);

    let mut client = Client {
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
    fn run(&mut self) {
        let clone_buf = self.buffer.clone();
        let stream = self.stream.try_clone().expect("could not clone stream");
        thread::spawn(move || {
            Client::listen_stdin(clone_buf.clone(), stream);
        });

        self.listen_server();
    }

    // wait for user changes, update the buffer, and send them to the server
    fn listen_stdin(buf: Arc<Mutex<Buffer>>, mut stream: TcpStream) {
        loop {
            print!("{}\n> ", buf.lock().unwrap().to_string());
            stdout().flush().unwrap();

            let mut input = String::new();
            stdin().read_line(&mut input).unwrap();
            let op = if input.starts_with('.') {
                let mut chars = input.chars();
                chars.next(); // '.'
                match chars.next() {
                    Some('i') => {
                        assert_eq!(chars.next(), Some(' '));
                        let mut s = String::new();
                        while let Some(c) = chars.next() {
                            if c.is_whitespace() {
                                break;
                            }
                            s.push(c);
                        }
                        let index = s.parse().unwrap();

                        let mut s = String::new();
                        while let Some(c) = chars.next() {
                            if c == '\n' {
                                break;
                            }
                            s.push(c);
                        }

                        let mut buf = buf.lock().unwrap();
                        buf.insert(index, &s)
                    }
                    Some('d') => {
                        assert_eq!(chars.next(), Some(' '));
                        let mut s = String::new();
                        while let Some(c) = chars.next() {
                            if c.is_whitespace() {
                                break;
                            }
                            s.push(c);
                        }
                        //eprintln!("`{}`", s);
                        let index = s.parse().unwrap();

                        let mut s = String::new();
                        while let Some(c) = chars.next() {
                            if c.is_whitespace() {
                                break;
                            }
                            s.push(c);
                        }
                        //eprintln!("`{}`", s);
                        let len = s.parse().unwrap();

                        let mut buf = buf.lock().unwrap();
                        buf.delete(index, len)
                    }
                    Some('q') => exit(0),
                    c => {
                        println!("unknown command {:?}", c);
                        continue;
                    }
                }
            } else {
                let mut buf = buf.lock().unwrap();
                buf.append(input.trim_right())
            };
            if op.is_some() {
                let serialised = serialize(&op).expect("Could not serialize Op");
                stream.write(&(serialised.len() as u32).to_le_bytes()).expect("could not send size to server");
                stream.write(&serialised).expect("could not send to server");
            }
        }
    }

    // wait for broadcasts from the server and update the buffer
    fn listen_server(&mut self) {
        let mut buf: Vec<u8> = Vec::new();
        loop {
            // TODO - code dup with server
            let mut size_buf: [u8; 4] = [0; 4];
            assert_eq!(4, self.stream.read(&mut size_buf).expect("error reading size stream"));
            let size = u32::from_le_bytes(size_buf) as usize;
            let mut read = 0;
            loop {
                let mut cur = [0; 256];
                let count = self.stream.read(&mut cur).expect("error reading stream");
                read += count;
                buf.extend(&cur[..count]);
                if read >= size {
                    break;
                }
            }

            let op: Op = deserialize(&buf).expect("Could not deserialize Op");
            let mut buffer = self.buffer.lock().unwrap();
            buffer.apply(op);

            buf.clear();
        }
    }
}

impl Buffer {
    fn new(node_number: u32) -> Buffer {
        Buffer {
            node: Node::new(NodeId::new(node_number)),
            internal: Vec::new(),
        }
    }

    fn append(&mut self, s: &str) -> Op {
        let mut prev_id = match self.internal.last() {
            Some((id, _)) => id.clone(),
            None => self.node.begin(),
        };
        let mut added = Vec::with_capacity(s.len());
        for c in s.chars() {
            let id = self.node.new_id_with_bounds(&prev_id, &prev_id);
            self.internal.push((id.clone(), c));
            added.push((id.clone(), c));
            prev_id = id;
        }
        Op::Add(added)
    }

    fn insert(&mut self, position: usize, s: &str) -> Op {
        if self.internal.len() <= position + 1 {
            self.append(s);
            return Op::None;
        }

        let next_id = self.internal[position].0.clone();
        let mut prev = position;
        let begin = self.node.begin();
        let mut added = Vec::with_capacity(s.len());
        for c in s.chars() {
            let prev_id = if prev == 0 {
                &begin
            } else {
                &self.internal[prev - 1].0
            };
            let id = self.node.new_id_with_bounds(&prev_id, &next_id);
            self.internal.insert(prev, (id.clone(), c));
            added.push((id, c));
            prev += 1;
        }
        Op::Add(added)
    }

    fn delete(&mut self, position: usize, len: usize) -> Op {
        let deleted = self.internal.drain(position..position + len);
        Op::Remove(deleted.map(|(id, _)| id).collect())
    }

    fn apply(&mut self, op: Op) {
        let mut changed = false;
        match op {
            Op::None => {}
            Op::Add(id_chars) => {
                for (id, c) in id_chars {
                    if id.node == self.node.id {
                        continue;
                    }

                    if let Err(i) = self.internal.binary_search_by(|(pid, _)| pid.cmp(&id)) {
                        self.internal.insert(i, (id, c));
                        changed = true;
                    }
                }
            }
            Op::Remove(ids) => {
                for id in ids {
                    self.internal.retain(|(i, _)| i != &id);
                    changed = true;
                }
            }
        }

        if changed {
            print!("{}\n> ", self.to_string());
            stdout().flush().unwrap();
        }
    }

    // FIXME should be a Display impl
    fn to_string(&self) -> String {
        let mut result = String::new();
        self.internal.iter().for_each(|(_, c)| result.push(*c));

        result
    }
}

#[derive(Serialize, Deserialize)]
enum Op {
    None,
    Add(Vec<(Id, char)>),
    Remove(Vec<Id>),
}

impl Op {
    fn is_some(&self) -> bool {
        match self {
            Op::None => false,
            _ => true,
        }
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
