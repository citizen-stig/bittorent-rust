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

        let mut deserializer_2 = BencodeDeserializer::new(&raw_meta[..]);

        let meta_info_2 = MetaInfo::deserialize(&mut deserializer_2).unwrap();
        assert_eq!(meta_info_2, torrent_file.info);


        let raw_torrent = to_bencode(&torrent_file).unwrap();
        // Write raw_meta to a file named "sample_2.torrent" in the same folder
        let output_path = Path::new(&project_root)
            .join("torrents")
            .join("sample_2.torrent");
        std::fs::write(output_path, &raw_torrent).expect("Failed to write raw_meta to file");
    }
}
