use std::{ net, marker, io, collections,  };
use std::str::FromStr;
use std::io::{ Read, Write };

use mio::*;

use crypto::sha1;
use crypto::digest::Digest;

use rustc_serialize::base64;
use rustc_serialize::base64::ToBase64;

use utils::*;

const SERVER: Token = Token(0);
const WEBSOCKET_GUID: &'static str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

struct TokenFactory(usize);
impl TokenFactory {
    fn next_token(&mut self) -> Token { self.0 += 1; Token(self.0) }
}

pub struct WebSocketServer<T, C> where T: WebSocketHandler<C> {
    handler: T,                                         // User handler which implements callbacks
    phantom: marker::PhantomData<C>,                    // I need to use this to calm down the compiler
    listener: tcp::TcpListener,                         // Listens all incoming connections

    // List af all clients:
    // C - User object returned for a new connection
    // TcpStream - Stream we need to delete in case of disconnect
    // bool - If client was handshaked
    clients: collections::HashMap<Token, (C, tcp::TcpStream, bool)>,
    tokens: TokenFactory                                // Counter to assign tokens to clients
}

impl<T, C> Handler for WebSocketServer<T, C> where T: WebSocketHandler<C> {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, main_loop: &mut EventLoop<Self>, token: Token, events: EventSet) {
        match token {
            // Got new connection. Lets add it to the list of clients and notify a user
            SERVER => {
                match self.listener.accept() {
                    Ok(client) => {
                        if let Err(e) = self.register_client(client, main_loop) {
                            println!("Error registering a client: {}", e)
                        }
                    },
                    Err(e) => { println!("Error accepting a client: {}", e) }
                }
            },
            ref token => {
                // Client has been disconnected
                // TODO: Handle error
                if events.is_error() || events.is_hup() {
                    if let Err(e) = self.disconnect_client(*token, main_loop) {
                        println!("Error disconnecting a client {}", e)
                    }
                } else if events.is_readable() {
                    // Got a message from the client. Lets get our client data
                    match self.clients.get_mut(token) {
                        Some(&mut (ref mut client,
                                   ref mut stream,
                                   ref mut handshaked)) => {
                            let ref mut buffer = [0; 1024];

                            // Read a message from the client socket
                            match stream.read(buffer) {
                                Ok(size) if *handshaked => { self.handler.on_message(slice_to_string(&buffer[..size]), client) }
                                Ok(size) => {
                                    match Self::create_handshake_response(slice_to_string(&buffer[..size])) {
                                        Ok(response) => {
                                            match stream.write_all(response.as_bytes()) {
                                                Ok(_) => { *handshaked = true }
                                                Err(e) => { println!("Can't send a handshake response to the client: {}", e) }
                                            }
                                        },
                                        Err(e) => { println!("Failed to handshake a client: {}", e) }
                                    }
                                },
                                Err(e) => { println!("An error occured while reading client socket {}", e) }
                            }
                        },
                        None => { println!("Failed to find a client, which sends a message {:?}", token); }
                    }
                }
            }
        }
    }
}

impl<T, C> WebSocketServer<T, C> where T: WebSocketHandler<C> + Sync {
    fn new(handler: T, addr: &'static str) -> io::Result<()> {
        let formated_addr = net::SocketAddr::from_str(addr).unwrap();
        let listener = try!(tcp::TcpListener::bind(&formated_addr));

        let mut result = WebSocketServer {
            handler: handler,
            phantom: marker::PhantomData,
            listener:listener,
            clients: collections::HashMap::new(),
            tokens: TokenFactory(1)
        };

        let mut main_loop = try!(EventLoop::<Self>::new());
        try!(main_loop.register(&result.listener, SERVER));
        try!(main_loop.run(&mut result));

        return Ok(())
    }

    fn register_client(&mut self, client: Option<tcp::TcpStream>, main_loop: &mut EventLoop<Self>) -> Result<(), String> {
        if let Some(client) = client {
            let token = self.tokens.next_token();

            match main_loop.register(&client, token) {
                Err(e) => { return Err(format!("Error registering a client: {}", e)) },
                _ => {
                    let addr = format!("{}", client.peer_addr().unwrap());

                    let _ = self.clients.insert(token, (self.handler.on_connect(addr), client, false));
                }
            }
        }
        return Ok(())
    }

    pub fn create_handshake_response(message: String) -> Result<String, String> {
        // Check if we got valid handshake message from a client
        match message.find("Upgrade: websocket") {
            Some(_) => {
                // Check if message contains a security key
                let key = message.lines().
                    find(|s| s.starts_with("Sec-WebSocket-Key")).
                    and_then(|s| s.split(":").last());

                match key {
                    // Calculate response security key
                    Some(key) => {
                        let ref mut sha_object = sha1::Sha1::new();
                        sha_object.input_str(format!("{}{}", key.trim(), WEBSOCKET_GUID).as_str());

                        // Seriously `crypto`? Buffer as a parameter?
                        let ref mut buffer = [0; 20]; sha_object.result(buffer);

                        // Response
                        return Ok(format!("{}\r\n{}\r\n{}\r\n{}\r\n\r\n",
                                          "HTTP/1.1 101 Switching Protocols",
                                          "Upgrade: websocket",
                                          "Connection: Upgrade",
                                          format!("Sec-WebSocket-Accept: {}", buffer.to_base64(base64::MIME))));
                    },
                    None => { Err(format!("Can't find a key in a handshake message: \n{}", message)) }
                }
            },
            None => { Err(format!("Client sent invalid handshake: \n{}", message)) }
        }
    }

    fn disconnect_client(&mut self, token: Token, main_loop: &mut EventLoop<Self>) -> Result<(), String> {
        match self.clients.remove(&token) {
            Some((client, stream, _)) => {
                match main_loop.deregister(&stream) {
                    Ok(_) => { self.handler.on_disconnect(client) },
                    Err(e) => { return Err(format!("Failed do unregister a client {}", e)) }
                }
            },
            None => { return Err(format!("Failed to disconnect a client")) }
        }
        return Ok(())
    }
}

pub trait WebSocketHandler<C>: Sized + marker::Sync {
    fn listen(self, addr: &'static str) -> io::Result<()> {
        return WebSocketServer::new(self, addr)
    }

    fn on_connect(&self, addr: String) -> C;
    fn on_message(&self, message: String, client: &mut C);
    fn on_disconnect(&self, client: C);
}
