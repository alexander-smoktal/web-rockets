use std::net;
use std::marker;

struct WebSocketServer<T, C> where T: WebSocketHandler<C> {
	handler: T,
	phantom: marker::PhantomData<C>
}

trait WebSocketHandler<C>: Sized {
	fn listen(self, addr: &'static str) -> WebSocketServer<Self, C> {
		return WebSocketServer { handler: self, phantom: marker::PhantomData }
	}
	
	fn on_connect() -> C;
	fn on_message(message: usize);
	fn on_disconnect(C);
}

