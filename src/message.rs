use std::{ fmt, ops };
use std::vec::Vec;
use std::collections::HashMap;
use num;

use mio::*;

use utils::*;

#[allow(dead_code)]
pub const OPCODE_CONTINUATION: u8 = 0x0;
#[allow(dead_code)]
pub const OPCODE_TEXT: u8 = 0x1;
#[allow(dead_code)]
pub const OPCODE_BINARY: u8 = 0x2;
#[allow(dead_code)]
pub const OPCODE_CONNECTION_CLOSE: u8 = 0x8;
#[allow(dead_code)]
pub const OPCODE_PING: u8 = 0x9;
#[allow(dead_code)]
pub const OPCODE_PONG: u8 = 0xA;

pub struct Message {
    pub opcode: u8,
    is_final: bool,
    was_masked: bool,
    payload: Vec<u8>,
}

impl Message {
    pub fn get_data(&self) -> Vec<u8> {
        let mut result = vec![128 & self.opcode];

        if self.payload.len() < 126 { let _ = result.push(self.payload.len() as u8); }
        // Let's assume we'll not have messages longer then 4gb
        else if self.payload.len() < num::pow(2, 32) {
            // Push 126 as indicator and next 2 bytes of exetended len
            result.push_all(&[126, 0, 0]);
            unsafe { *(result.as_mut_ptr().offset(2) as *mut u16) = u16::to_be(self.payload.len() as u16); }
        }
        else { panic!("Message is longer then 4gb. Are realy want to do this?"); }

        //We have no mask, just append a payload
        result.push_all(self.payload.as_slice());

        return result;
    }
}

impl fmt::Display for Message {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "{}", slice_to_string(self.payload.as_slice()))
    }
}

impl ops::Add for Message {
    type Output = Message;

    fn add(mut self, rhs: Message) -> Message {
        self.payload.push_all(rhs.payload.as_slice());

        return Message {
            opcode: self.opcode,
            is_final: self.is_final | rhs.is_final,
            was_masked: self.was_masked,
            payload: self.payload
        }
    }
}

pub struct MessageFactory {
    fragments: HashMap<Token, Vec<u8>>,      // Incomplete data from a reader (if fragment is longer then receiver buffer)
    messages: HashMap<Token, Message>        // Message without final fragments (unfinished)
}

impl MessageFactory {
    pub fn new() -> MessageFactory {
        return MessageFactory { fragments: HashMap::new(), messages: HashMap::new() }
    }

    // Get message len. Return (payload len, header len)
    // Lets assume we'll not get more than 2^64 bytes data %)
    fn get_message_len(data: & Vec<u8>) -> (u64, usize) {
        let mut payload_len: u64 = (data[1] & 127) as u64;

        if payload_len < 126 { return (payload_len, 2) }
        else if payload_len == 126
        {
            unsafe { payload_len = u16::from_be(*(data.as_ptr().offset(2) as *const u16)) as u64 }
            return (payload_len, 4)
        }
        else
        {
            unsafe { payload_len =  u64::from_be(*(data.as_ptr().offset(2) as *const u64)) }
            return (payload_len, 10)
        }
    }

    // Haskell style (``Deal with`` it gif is playing)
    fn unmask_data(data: Vec<u8>, mask: [u8; 4]) -> Vec<u8> {
        return mask.into_iter().
                    cycle().
                    zip(data.into_iter()).
                    map(|(x, y)| x ^ y).
                    collect::<Vec<u8>>()
    }

    #[allow(dead_code)]
    pub fn create_ping_message(&self) -> Message {
        return Message {
            opcode: OPCODE_PING,
            is_final: true,
            was_masked: false,
            payload: vec![]
        }
    }

    pub fn create_pong_message(&self) -> Message {
        return Message {
            opcode: OPCODE_PONG,
            is_final: true,
            was_masked: false,
            payload: vec![]
        }
    }

    pub fn parse(&mut self, data: &[u8], client: &Token) -> Option<Message> {
        // First let's check if we have unfinished fragments for current user
        // If true, append current data to the previous segment
        let mut local_data = self.fragments.remove(client).unwrap_or(vec![]);
        local_data.push_all(data);

        // Lets check if we have complete fragment in our data if not
        // push it back to the fragment list
        let (payload_len, header_len) = Self::get_message_len(&local_data);

        if (payload_len + header_len as u64) as usize > local_data.len() {
            let _ = self.fragments.insert(*client, local_data);
        }

        // We've got complete message. Lets process it
        else {
            let is_masked = (data[1] & 128).count_ones() > 0; // This is amazing %)
            if is_masked {
                let mask  = unsafe { *(local_data.as_ptr().offset(header_len as isize) as *const [u8; 4]) };

                // Drop header and unmask
                local_data = Self::unmask_data(local_data.split_off(header_len + 4), mask);
            } else {
                // Just drop header
                local_data = local_data.split_off(header_len);
            }

            let message = Message {
                opcode: data[0] & 15,
                is_final: (data[0] & 128).count_ones() > 0,
                was_masked: is_masked,
                payload: local_data
            };

            // Final message. Lets check if we have previous fragments and
            // return single or merged result to the user.
            if message.is_final {
                match self.messages.remove(client) {
                    Some(framed_message) => { return Some(framed_message + message) },
                    None => { return Some(message) }
                }
            }
            // Message fragment.
            // Add it to the list of fragments (merge into previous fragment if exists)
            else {
                match self.messages.remove(client) {
                    Some(framed_message) => { let _ = self.messages.insert(*client, framed_message + message); },
                    None => { let _ = self.messages.insert(*client, message); }
                }
            }
        }

        return None
    }
}
