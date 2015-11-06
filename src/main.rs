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

    fn on_message(&self, message: usize) {
        println!("Got fucking message {}", message)
    }

    fn on_disconnect(&self, client: Client) {
        println!("Client disconnected {:?}", client)
    }
}

fn main() {
	let x = Server;
    match x.listen("127.0.0.1:8081") {
        Err(e) => println!("Failed to host websocket server: {}", e),
        _ => ()
    }

    println!("Hello, world!");
}
