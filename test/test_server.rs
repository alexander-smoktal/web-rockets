use web_rockets::server::{ WebSocketServer, WebSocketHandler, Message };

struct Client;
struct Server;

impl WebSocketHandler<Client> for Server {

    fn on_connect(&self, _: String) -> Client { return Client }
    fn on_message(&self, _: Message, _: &mut Client) {}
    fn on_disconnect(&self, _: Client) {}
}


#[test]
fn check_hashshake_creation() {
    let request = format!("{}\r\n{}",
                          "Upgrade: websocket",
                          "Sec-WebSocket-Key: x3JJHMbDL1EzLkh9GBhXDw==");

    let response = Ok(format!("{}\r\n{}\r\n{}\r\n{}\r\n\r\n",
                              "HTTP/1.1 101 Switching Protocols",
                              "Upgrade: websocket",
                              "Connection: Upgrade",
                              "Sec-WebSocket-Accept: HSmrc0sMlYUkAGmm5OPpG2HaGWk="));

    assert_eq!(WebSocketServer::<Server, Client>::create_handshake_response(request), response)
}
