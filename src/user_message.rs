use mio::tcp::TcpStream;
use std::borrow::Borrow;
use std::vec::Vec;
use std::io::Write;
use std::io;
use std::fmt;

use utils::*;
use socket_message;

#[allow(dead_code)]
pub struct Message <'a> {
    pub data: Vec<u8>,
    client: &'a mut TcpStream,
    message_type: u8
}

impl<'a> fmt::Display for Message<'a> {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", slice_to_string(self.data.borrow()))
    }
}

#[allow(dead_code)]
impl<'a> Message<'a> {
    pub fn new(data: Vec<u8>, client: &'a mut TcpStream, message_type: u8) -> Message {
        return Message { data: data, client: client, message_type: message_type }
    }

    pub fn is_text(&self) -> bool { return self.message_type == socket_message::OPCODE_TEXT }
    pub fn is_binary(&self) -> bool { return self.message_type == socket_message::OPCODE_BINARY }

    pub fn reply(&mut self, message: String) -> io::Result<()> {
        let reply = socket_message::SocketMessage::from_string(message).into_vector();

        return self.client.write_all(reply.borrow());
    }
}
