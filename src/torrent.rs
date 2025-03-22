#![allow(dead_code)]

#[derive(Debug, serde::Deserialize)]
pub struct TorrentFile {
    // TODO: How to do `& str`
    announce: String,
    // comment: &'a str,
    info: MetaInfo,
}

#[derive(Debug, serde::Deserialize)]
pub struct MetaInfo {
    length: usize,
    name: String,
    //
    #[serde(rename = "piece length")]
    piece_length: usize,
    // pieces: &'a [u8],
    // TODO: Byte string
    pieces: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bencode::BencodeDeserializer;
    use serde::Deserialize;
    use std::path::Path;

    #[test]
    fn deserialize_sample_torrent_file() {
        use std::fs::File;
        use std::io::Read;
        let project_root =
            std::env::var("CARGO_MANIFEST_DIR").expect("Failed to get CARGO_MANIFEST_DIR");
        let torrent_path = Path::new(&project_root)
            .join("torrents")
            .join("ubuntu-22.04.5-live-server-amd64.iso.torrent");

        let mut file = File::open(torrent_path).expect("Failed to open torrent file");
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)
            .expect("Failed to read torrent file");

        let mut deserializer = BencodeDeserializer::new(&bytes);
        let result = TorrentFile::deserialize(&mut deserializer).unwrap();
        println!("{:#?}", result);
    }
}
