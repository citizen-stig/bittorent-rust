#![allow(dead_code)]

pub mod meta;
pub mod network;



#[cfg(test)]
mod tests {
    use super::*;
    use crate::bencode::{to_bencode, BencodeDeserializer};
    use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
    use serde::Deserialize;
    use sha1::Digest;
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::path::Path;

    #[test]
    fn deserialize_sample_torrent_file() {
        use std::fs::File;
        use std::io::Read;
        let project_root =
            std::env::var("CARGO_MANIFEST_DIR").expect("Failed to get CARGO_MANIFEST_DIR");
        let torrent_path = Path::new(&project_root)
            .join("torrents")
            .join("sample.torrent");
        // .join("ubuntu-22.04.5-live-server-amd64.iso.torrent");

        let mut file = File::open(torrent_path).expect("Failed to open torrent file");
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .expect("Failed to read torrent file");

        let mut deserializer = BencodeDeserializer::new(&bytes);
        let torrent_file = RawTorrentFile::deserialize(&mut deserializer).unwrap();
        println!("Filename: {:#?}", torrent_file.info.name);
        println!("Tracker URL: {:#?}", torrent_file.announce);
        println!("Length: {}", torrent_file.info.length);
        let raw_meta = to_bencode(&torrent_file.info).unwrap();

        let mut hasher = sha1::Sha1::new();
        hasher.update(&raw_meta);
        let hash = hasher.finalize();
        println!("Info Hash: {}", hex::encode(&hash[..]));
        println!("Piece Length: {}", torrent_file.info.piece_length);

        println!("Piece Hashes: ");
        for piece in torrent_file.info.pieces.chunks(20) {
            println!("{}", hex::encode(piece));
        }

        // Perform a simple GET request to the `announce` URL with a `a=10` query parameter
        let client = reqwest::blocking::Client::new();
        let info_hash_encoded = percent_encode(&hash[..], NON_ALPHANUMERIC).to_string();
        println!("Info Hash: {}", info_hash_encoded);
        let peer_id = b"-GT0001-NGO456789012";
        let peer_id_encoded = percent_encode(peer_id, NON_ALPHANUMERIC).to_string();

        // let x = [("info_hash", &hash[..]), (&b"peer_id"[..], &peer_id[..])];

        let url = format!(
            "{}?info_hash={}&peer_id={}&port={}&uploaded=0&downloaded=0&left={}&compact=1",
            torrent_file.announce,
            info_hash_encoded,
            peer_id_encoded,
            6881,
            torrent_file.info.length
        );

        let response = client.get(&url).send().expect("Failed to send GET request");

        println!("Response Status: {}", response.status());
        // println!("Response Body: {}", response.text().unwrap());

        let response_bytes = response.bytes().unwrap();
        let mut deserializer = BencodeDeserializer::new(response_bytes.as_ref());
        let tracker_response = RawTrackerResponse::deserialize(&mut deserializer)
            .expect("Failed to deserialize tracker response");
        println!("Interval: {}", tracker_response.interval);
        println!("Peers: {}", tracker_response.peers.len());
        println!("Complete: {}", tracker_response.complete);
        println!("Incomplete: {}", tracker_response.incomplete);
        for chunk in tracker_response.peers.chunks(6) {
            let octets: [u8; 4] = chunk[0..4].try_into().unwrap();
            let port: [u8; 2] = chunk[4..6].try_into().unwrap();
            let port: u16 = u16::from_be_bytes(port);
            let ip_address = Ipv4Addr::from(octets);
            println!("IP: {}:{}", ip_address, port);
        }


    }
}
