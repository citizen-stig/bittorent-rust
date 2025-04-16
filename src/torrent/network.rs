use crate::bencode::BencodeDeserializer;
use crate::torrent::meta::TorrentFile;
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt};
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use serde::Deserialize;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4};

const PEER_ID: &'static [u8; 20] = b"-GT0001-NGO456789012";

pub struct TorrentTrackerClient {
    tracker_client: reqwest::blocking::Client,
}

impl TorrentTrackerClient {
    pub fn new() -> Self {
        Self {
            tracker_client: reqwest::blocking::Client::new(),
        }
    }

    pub fn get_peers(&self, torrent_file: &TorrentFile) -> Vec<std::net::SocketAddrV4> {
        let hash = torrent_file.meta_hash();
        let info_hash_encoded = percent_encode(&hash[..], NON_ALPHANUMERIC);
        let peer_id = b"-GT0001-NGO456789012";
        let peer_id_encoded = percent_encode(peer_id, NON_ALPHANUMERIC);

        let url = format!(
            "{}?info_hash={}&peer_id={}&port={}&uploaded=0&downloaded=0&left={}&compact=1",
            torrent_file.announce,
            info_hash_encoded,
            peer_id_encoded,
            6881,
            torrent_file.info.length
        );

        let response = self
            .tracker_client
            .get(&url)
            .send()
            .expect("Failed to send GET request");
        println!("Response Status: {}", response.status());

        let response_bytes = response.bytes().unwrap();

        let mut deserializer = BencodeDeserializer::new(response_bytes.as_ref());
        let tracker_response = RawTrackerResponse::deserialize(&mut deserializer)
            .expect("Failed to deserialize tracker response");

        let tracker_response = TrackerResponse::from(tracker_response);
        tracker_response.peers
    }
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct RawTrackerResponse {
    interval: u64,
    #[serde(with = "serde_bytes")]
    peers: Vec<u8>,
    complete: u64,
    incomplete: u64,
}

pub struct TrackerResponse {
    interval: std::time::Duration,
    peers: Vec<std::net::SocketAddrV4>,
    complete: u64,
    incomplete: u64,
}

// TODO: Convert to try_from with proper error
impl From<RawTrackerResponse> for TrackerResponse {
    fn from(tracker_response: RawTrackerResponse) -> Self {
        let mut peers = Vec::new();
        for chunk in tracker_response.peers.chunks(6) {
            let octets: [u8; 4] = chunk[0..4].try_into().unwrap();
            let port: [u8; 2] = chunk[4..6].try_into().unwrap();
            let port: u16 = u16::from_be_bytes(port);
            let ip_address = Ipv4Addr::from(octets);
            peers.push(SocketAddrV4::new(ip_address, port));
        }
        let interval = std::time::Duration::from_secs(tracker_response.interval);

        Self {
            interval,
            peers,
            complete: tracker_response.complete,
            incomplete: tracker_response.incomplete,
        }
    }
}

pub struct PeerClient {
    stream: std::net::TcpStream,
}

impl PeerClient {
    pub fn new(peer: SocketAddrV4, info_hash: [u8; 20]) -> Self {
        let mut stream = std::net::TcpStream::connect(peer).expect("Failed to connect to peer");
        let mut handshake = [0; 68];
        handshake[0] = 19;
        handshake[1..20].copy_from_slice(&b"BitTorrent protocol"[..]);
        handshake[28..48].copy_from_slice(&info_hash);
        handshake[48..68].copy_from_slice(&PEER_ID[..]);
        println!("Sending bytes : {}", hex::encode(&handshake));
        stream
            .write_all(&handshake)
            .expect("Failed to send data to peer");

        let mut buffer = [0; 68];
        stream
            .read_exact(&mut buffer)
            .expect("Failed to read data from peer");
        Self { stream }
    }

    pub fn read_message(&mut self) -> PeerMessage {
        PeerMessage::from_reader(&mut self.stream).unwrap()
    }

    pub fn send_message(&mut self, message: PeerMessage) {
        message.write_to_stream(&mut self.stream).unwrap();
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PeerMessageType {
    Choke = 0,
    Unchoke = 1,
    Interested = 2,
    NotInterested = 3,
    Have = 4,
    Bitfield = 5,
    Request = 6,
    Piece = 7,
    Cancel = 8,
}

#[derive(Debug, thiserror::Error)]
pub enum PeerMessageTypeError {
    #[error("Received unknown peer message type identifier: {0}")]
    Unknown(u8),
}

impl TryFrom<u8> for PeerMessageType {
    type Error = PeerMessageTypeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PeerMessageType::Choke),
            1 => Ok(PeerMessageType::Unchoke),
            2 => Ok(PeerMessageType::Interested),
            3 => Ok(PeerMessageType::NotInterested),
            4 => Ok(PeerMessageType::Have),
            5 => Ok(PeerMessageType::Bitfield),
            6 => Ok(PeerMessageType::Request),
            7 => Ok(PeerMessageType::Piece),
            8 => Ok(PeerMessageType::Cancel),
            unknown => Err(PeerMessageTypeError::Unknown(unknown)),
        }
    }
}

#[derive(Debug)]
pub struct PieceInfo {
    pub index: u32,
    pub begin_bytes_offset: u32,
    pub length_bytes: u32,
}

#[derive(Debug)]
pub enum PeerMessage {
    Choke,
    Unchoke,
    Interested,
    NotInterested,
    Have(u32),
    Bitfield(Vec<u8>),
    Request(PieceInfo),
    Piece(u32, u32, Vec<u8>),
    Cancel(u32, u32, u32),
}

impl PeerMessage {
    fn write_to_stream(self, mut stream: impl std::io::Write) -> std::io::Result<()> {
        match self {
            PeerMessage::Choke => {}
            PeerMessage::Unchoke => {}
            PeerMessage::Interested => {
                stream.write_u32::<BigEndian>(1)?;
                stream.write_u8(PeerMessageType::Interested as u8)?;
            }
            PeerMessage::NotInterested => {}
            PeerMessage::Have(_) => {}
            PeerMessage::Bitfield(_) => {}
            PeerMessage::Request(piece_info) => {
                stream.write_u32::<BigEndian>(13)?;
                stream.write_u8(PeerMessageType::Request as u8)?;
                stream.write_u32::<BigEndian>(piece_info.index)?;
                stream.write_u32::<BigEndian>(piece_info.begin_bytes_offset)?;
                stream.write_u32::<BigEndian>(piece_info.length_bytes)?;
            }
            PeerMessage::Piece(_, _, _) => {}
            PeerMessage::Cancel(_, _, _) => {}
        }
        Ok(())
    }

    pub fn from_reader(mut reader: impl Read) -> std::io::Result<Self> {
        println!("FROM READER START");
        let length = reader.read_u32::<BigEndian>()?;
        println!("FROM READER. LEN={}", length);

        if length == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Keep-alive not handled",
            ));
        }

        let message_type = PeerMessageType::try_from(reader.read_u8()?)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        println!("Message type: {:?}", message_type);
        // Subtract message ID byte
        let mut payload = vec![0u8; (length - 1) as usize];
        reader.read_exact(&mut payload)?;
        Self::build_from_type_and_payload(message_type, payload)
    }

    fn build_from_type_and_payload(
        message_type: PeerMessageType,
        payload: Vec<u8>,
    ) -> std::io::Result<Self> {
        Ok(match message_type {
            PeerMessageType::Choke => PeerMessage::Choke,
            PeerMessageType::Unchoke => PeerMessage::Unchoke,
            PeerMessageType::Interested => PeerMessage::Interested,
            PeerMessageType::NotInterested => PeerMessage::NotInterested,
            PeerMessageType::Have => unimplemented!("Have to be implemented"),
            PeerMessageType::Bitfield => {
                // TODO: smarter parsing into actual data
                PeerMessage::Bitfield(payload)
            }
            PeerMessageType::Request => unimplemented!("Request to be implemented"),
            PeerMessageType::Piece => {
                let mut header = std::io::Cursor::new(&payload[..8]);
                let index = header.read_u32::<BigEndian>()?;
                let begin_bytes_offset = header.read_u32::<BigEndian>()?;
                let piece_data = payload[13..64].to_vec();
                PeerMessage::Piece(index, begin_bytes_offset, piece_data)
            }
            PeerMessageType::Cancel => unimplemented!("Cancel to be implemented"),
        })
    }
}
