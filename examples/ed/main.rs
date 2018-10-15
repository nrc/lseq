#![feature(int_to_from_bytes)]

extern crate lseq;

mod client;
mod server;

fn main() {
    let mut args = ::std::env::args();
    args.next().unwrap();
    let arg = args.next();
    match arg {
        Some(a) => client::run_client(a),
        None => server::run_server(),
    }
}
