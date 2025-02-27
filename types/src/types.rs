use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter, Result as FmtResult};
use std::io::Result;
use tokio::sync::mpsc::{Receiver, Sender};

#[inline]
pub fn new_io_error(s: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, s)
}

/// peer's network id.
#[derive(Copy, Clone, Default, Eq, PartialEq, Ord, PartialOrd, Hash, Deserialize, Serialize)]
pub struct PeerId(pub [u8; 32]);

pub const PEER_ID_LENGTH: usize = 32;

impl PeerId {
    pub fn short_show(&self) -> String {
        let mut hex = String::new();
        hex.extend(self.0.iter().map(|byte| format!("{:02x?}", byte)));
        let mut new_hex = String::new();
        new_hex.push_str("0x");
        new_hex.push_str(&hex[0..4]);
        new_hex.push_str("...");
        new_hex.push_str(&hex[hex.len() - 5..]);
        new_hex
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() != 32 {
            return Err(new_io_error("peer id bytes failure."));
        }
        let mut raw = [0u8; 32];
        raw.copy_from_slice(bytes);
        Ok(Self(raw))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    pub fn from_hex(s: impl ToString) -> Result<PeerId> {
        let s = s.to_string();
        if s.len() != 64 {
            return Err(new_io_error("peer bytes failure."));
        }

        let mut value = [0u8; 32];

        for i in 0..(s.len() / 2) {
            let res = u8::from_str_radix(&s[2 * i..2 * i + 2], 16)
                .map_err(|_e| new_io_error("peer hex failure."))?;
            value[i] = res;
        }

        Ok(PeerId(value))
    }

    pub fn to_hex(&self) -> String {
        let mut hex = String::new();
        hex.extend(self.0.iter().map(|byte| format!("{:02x?}", byte)));
        hex
    }
}

impl Debug for PeerId {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        let mut hex = String::new();
        hex.extend(self.0.iter().map(|byte| format!("{:02x?}", byte)));
        write!(f, "0x{}", hex)
    }
}

/// support some common broadcast algorithm.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum Broadcast {
    Gossip,
    StableAll,
}

/// Transports types support by Endpoint.
#[derive(Debug, Copy, Clone, Hash, Deserialize, Serialize, Eq, PartialEq)]
pub enum TransportType {
    QUIC, // 0u8
    TCP,  // 1u8
    RTP,  // 2u8
    UDT,  // 3u8
}

impl TransportType {
    /// transports from parse from str.
    pub fn from_str(s: &str) -> Self {
        match s {
            "quic" => TransportType::QUIC,
            "tcp" => TransportType::TCP,
            "rtp" => TransportType::RTP,
            "udt" => TransportType::UDT,
            _ => TransportType::QUIC,
        }
    }

    pub fn to_str<'a>(&self) -> &'a str {
        match self {
            TransportType::QUIC => "quic",
            TransportType::TCP => "tcp",
            TransportType::RTP => "rtp",
            TransportType::UDT => "udt",
        }
    }

    pub fn from_byte(b: u8) -> Result<Self> {
        match b {
            0u8 => Ok(TransportType::QUIC),
            1u8 => Ok(TransportType::TCP),
            2u8 => Ok(TransportType::RTP),
            3u8 => Ok(TransportType::UDT),
            _ => Err(new_io_error("transport bytes failure.")),
        }
    }

    pub fn to_byte(&self) -> u8 {
        match self {
            TransportType::QUIC => 0u8,
            TransportType::TCP => 1u8,
            TransportType::RTP => 2u8,
            TransportType::UDT => 3u8,
        }
    }
}

#[derive(Debug)]
pub struct TransportStream {
    transport: TransportType,
    sender: Sender<Vec<u8>>,
    receiver: Receiver<Vec<u8>>,
}

impl Eq for TransportStream {}

impl PartialEq for TransportStream {
    fn eq(&self, other: &TransportStream) -> bool {
        self.transport == other.transport
    }
}

impl TransportStream {
    pub fn new(
        transport: TransportType,
        sender: Sender<Vec<u8>>,
        receiver: Receiver<Vec<u8>>,
    ) -> Self {
        Self {
            transport,
            sender,
            receiver,
        }
    }

    pub fn channel(self) -> (Sender<Vec<u8>>, Receiver<Vec<u8>>) {
        (self.sender, self.receiver)
    }
}
