use std::{ net, marker, io, thread, string, time };
use std::sync::mpsc;
use std::collections::vec_deque;
use std::io::Read;

pub struct WebSocketServer<T, C> where T: WebSocketHandler<C> {
    handler: T,                                    // User handler which implements callbacks
    phantom: marker::PhantomData<C>,               // I need to use this to calm down the compiler

    clients: vec_deque::VecDeque<(net::TcpStream, C, bool)>,  // All connected clients
}

impl<T, C> WebSocketServer<T, C> where T: WebSocketHandler<C> + Sync {
    fn new(handler: T, addr: &'static str) ->
        io::Result<WebSocketServer<T, C>> {

        let mut result = WebSocketServer {
            handler: handler,
            phantom: marker::PhantomData,
            clients: vec_deque::VecDeque::with_capacity(64),
        };
        try!(result.listen(addr));

        return Ok(result)
    }

    fn listen(&mut self, addr: &'static str) -> io::Result<()> {
        // First start thread which accept incoming clients
        let (incoming_clients_tx, incoming_clients) = mpsc::channel();

        let incoming_thread = thread::Builder::new().name("Incoming thread".to_string()).spawn
        (
            move || {
                let listener = net::TcpListener::bind(addr).unwrap();
                for stream in listener.incoming() {
                    if let Err(mpsc::SendError(_))  = incoming_clients_tx.send(stream) {
                        println!("Error sending new incoming connection to the server. Is it alive?");
                    }
                }
            }
        );

        // Then start our mainloop
        loop {
            // Get all new connections
            match incoming_clients.try_recv() {
                Ok(incoming_client) => { self.handle_new_connection(incoming_client) },
                _ => { }
            }

            let mut size: isize = self.clients.len() as isize;
            // Process all new messages
            while size > 0 {
                size -= 1;

                let (mut stream, client, handshaked) = self.clients.pop_front().unwrap();

                let mut buffer = [0; 1024];
                match stream.read(&mut buffer) {
                    Ok(size) => {
                        println!("Size: {}", size);
                        if size > 0 {
                            println!("Got a fucking message of size {}: '{}'", size, string::String::from_utf8_lossy(&buffer));
                            self.clients.push_back((stream, client, handshaked))
                        } else {
                            self.handler.on_disconnect(client);
                        }
                    }
                    Err(e) => {
                        if e.kind() != io::ErrorKind::TimedOut {
                            println!("Error reading from the client socket: {}", e);
                            self.handler.on_disconnect(client);
                        }
                    }
                }
            }

            // To calm compiler
            if self.clients.len() > 50 { break }
        }

        incoming_thread.unwrap().join().unwrap();

        return Ok(())
    }

    fn handle_new_connection(&mut self, incoming_client: io::Result<net::TcpStream>) {
         match incoming_client {
            Ok(client) => {
                let addr = client.peer_addr().unwrap();
                match client.set_read_timeout(Some(time::Duration::new(1, 100))) {
                    Ok(_) => {
                        self.clients.push_back((client,
                                                self.handler.on_connect(format!("{}", addr)),
                                                false))
                        }
                    Err(e) => { println!("Can't set socket timeout: {}. Ignoring the client", e) }
                }
            }
            Err(e) => { println!("Error creating new connection {}", e) }
        }
    }
}

pub trait WebSocketHandler<C>: Sized + marker::Sync {
    fn listen(self, addr: &'static str) -> io::Result<WebSocketServer<Self, C>> {
        return WebSocketServer::new(self, addr)
    }

    fn on_connect(&self, addr: String) -> C;
    fn on_message(&self, message: usize, client: &mut C);
    fn on_disconnect(&self, client: C);
}
