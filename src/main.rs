#![feature(convert)]

extern crate mio;
extern crate crypto;
extern crate rustc_serialize;

mod server;

use server::WebSocketHandler;

#[derive(Debug)]
struct Client (i64);

struct Server;

impl server::WebSocketHandler<Client> for Server {

    fn on_connect(&self, addr: String) -> Client {
        println!("Client connected {}", addr);
        return Client(42)
    }

    fn on_message(&self, message: String, client: &mut Client) {
        println!("Got a message from the client {:?} {}", client, message)
    }

    fn on_disconnect(&self, client: Client) {
        println!("Client disconnected {:?}", client)
    }
}

fn main() {
    let x = Server;
    match x.listen("127.0.0.1:8081") {
        Err(e) => println!("Failed to host websocket server: {:?}", e),
        _ => ()
    }
}
