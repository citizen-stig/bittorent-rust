#![allow(dead_code)]

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TorrentFile {
    // TODO: How to do `& str`
    announce: String,
    // comment: &'a str,
    info: MetaInfo,
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct MetaInfo {
    length: usize,
    name: String,
    //
    #[serde(rename = "piece length")]
    piece_length: usize,
    // pieces: &'a [u8],
    #[serde(with = "serde_bytes")]
    pieces: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bencode::{to_bencode, BencodeDeserializer};
    use serde::Deserialize;
    use sha1::Digest;
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
        let torrent_file = TorrentFile::deserialize(&mut deserializer).unwrap();
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

    }
}
