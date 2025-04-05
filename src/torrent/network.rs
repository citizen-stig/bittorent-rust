use crate::bencode::BencodeDeserializer;
use crate::torrent::meta::TorrentFile;
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
        stream
            .write_all(&handshake)
            .expect("Failed to send data to peer");


        let mut buffer = [0; 68];
        stream
            .read_exact(&mut buffer)
            .expect("Failed to read data from peer");
        println!("Received 32 bytes: {}", hex::encode(&buffer[48..68]));
        
        Self { stream }
    }
}
