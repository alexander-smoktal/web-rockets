use std::net;
use std::marker;
use std::io;
use std::{ collections, hash };

struct TcpClient {
    id: u64,
    handle: net::TcpStream
}

impl PartialEq for TcpClient {
    fn eq(&self, other: &Self) -> bool { return self.id == other.id}
}
impl Eq for TcpClient {}

impl hash::Hash for TcpClient {
    fn hash<H>(&self, state: &mut H) where H: hash::Hasher { u64::hash(&self.id, state); }
}

pub struct WebSocketServer<T, C> where T: WebSocketHandler<C> {
    handler: T,
    listener: net::TcpListener,
    phantom: marker::PhantomData<C>,
    // We get TcpStream RawFd to store client as a key
    clients: collections::HashMap<TcpClient, C>,
    // Identifiers counter for our clients
    id_counter: u64
}

impl<T, C> WebSocketServer<T, C> where T: WebSocketHandler<C> {
    fn new(handler: T, listener: net::TcpListener) ->
        io::Result<WebSocketServer<T, C>> {

        let mut result = WebSocketServer {
            handler: handler,
            phantom: marker::PhantomData,
            listener: listener,
            clients: collections::HashMap::new(),
            id_counter: 0
        };
        try!(result.listen());

        return Ok(result)
    }

    fn listen(&mut self) -> io::Result<()> {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    //let (stream, _) = try!(self.listener.accept());
                    self.id_counter += 1;

                    let new_client = self.handler.on_connect(format!("{}", stream.peer_addr().unwrap()));

                    self.clients.insert(TcpClient { id: self.id_counter, handle: stream },
                                        new_client);
                    ()
                },
                Err(e) => { return Err(e) }
            }
        }

        return Ok(())
    }
}

pub trait WebSocketHandler<C>: Sized {
    fn listen(self, addr: &'static str) -> io::Result<WebSocketServer<Self, C>> {
        let listener = try!(net::TcpListener::bind(addr));

        return WebSocketServer::new(self, listener)
    }

    fn on_connect(&self, addr: String) -> C;
    fn on_message(&self, message: usize);
    fn on_disconnect(&self, C);
}
