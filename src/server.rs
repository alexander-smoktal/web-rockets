use std::{ net, marker, io, collections,  };
use std::str::FromStr;
use std::io::Read;

use mio::*;

const SERVER: Token = Token(0);

struct TokenFactory(usize);
impl TokenFactory {
    fn next_token(&mut self) -> Token { self.0 += 1; Token(self.0) }
}

pub struct WebSocketServer<T, C> where T: WebSocketHandler<C> {
    handler: T,                                         // User handler which implements callbacks
    phantom: marker::PhantomData<C>,                    // I need to use this to calm down the compiler
    listener: tcp::TcpListener,                         // Listens all incoming connections

    clients: collections::HashMap<Token, (C, tcp::TcpStream)>, // List af all clients
    tokens: TokenFactory                                       // Counter to assign tokens to clients
}

impl<T, C> Handler for WebSocketServer<T, C> where T: WebSocketHandler<C> {
    type Timeout = ();
    type Message = ();

    fn ready(&mut self, main_loop: &mut EventLoop<Self>, token: Token, events: EventSet) {
        match token {
            // Got new connection. Lets add it to the list of clients and notify a user
            SERVER => {
                match self.listener.accept() {
                    Ok(client) => { self.register_client(client, main_loop); },
                    Err(e) => { println!("Error accepting a client: {}", e) }
                }
            }
            ref token => {
                // Client has been disconnected
                // TODO: Handle error
                if events.is_error() || events.is_hup() {
                    self.disconnect_client(*token, main_loop);
                    return;
                }

                // Got a message from a client
                match self.clients.get_mut(token) {
                    Some(&mut (ref mut client, ref mut stream)) => {
                        let ref mut buffer = [0; 1024];

                        if let Ok(_) = stream.read(buffer) {
                            self.handler.on_message(String::from_utf8_lossy(buffer).into_owned(), client)
                        }
                    },
                    None => { println!("Failed to find a client, which sends a message {:?}", token); }
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

    fn register_client(&mut self, client: Option<tcp::TcpStream>, main_loop: &mut EventLoop<Self>) {
        if let Some(client) = client {
            let token = self.tokens.next_token();

            match main_loop.register(&client, token) {
                Err(e) => { println!("Error registering a client: {}", e) },
                _ => {
                    let addr = format!("{}", client.peer_addr().unwrap());

                    let _ = self.clients.insert(token,
                                                (self.handler.on_connect(addr), client));
                }
            }
        }
    }

    fn disconnect_client(&mut self, token: Token, main_loop: &mut EventLoop<Self>) {
        match self.clients.remove(&token) {
            Some((client, stream)) => {
                match main_loop.deregister(&stream) {
                    Ok(_) => { self.handler.on_disconnect(client) },
                    Err(e) => { println!("Failed do unregister a client {}", e) }
                }
            },
            None => { println!("Failed to disconnect a client"); }
        }
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
