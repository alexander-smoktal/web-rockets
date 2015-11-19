extern crate mio;
extern crate crypto;
extern crate rustc_serialize;
extern crate num;

mod utils;
mod server;
mod socket_message;
mod user_message;

use server::WebSocketHandler;

#[derive(Debug)]
struct Client (i64);

struct Server;

impl server::WebSocketHandler<Client> for Server {

    fn on_connect(&self, addr: String) -> Client {
        println!("Client connected {}", addr);
        return Client(42)
    }

    fn on_message(&self, mut message: user_message::Message, client: &mut Client) {
        println!("Got a message from the client {:?} {}", client, message);

        let _ = message.reply("Hello my friend!".to_string());
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
