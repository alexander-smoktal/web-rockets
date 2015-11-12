//use string::String;

pub fn slice_to_string(string: &[u8]) -> String { String::from_utf8_lossy(string).into_owned() }
