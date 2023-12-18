use serde::de::Expected;
use std::fmt::Formatter;

pub struct InvalidPubkey {
    addr: String,
}

impl InvalidPubkey {
    pub fn new(addr: String) -> Self {
        Self { addr: addr }
    }
}

impl Expected for InvalidPubkey {
    fn fmt(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str(&format!("{} is not a valid public key", &self.addr))
    }
}
