use crate::bencode::{to_bencode, BencodeDeserializationError, BencodeDeserializer};
use crate::torrent::network::PieceInfo;
use crate::torrent::SIXTEEN_KIBIBYTES;
use serde::Deserialize;
use sha1::Digest;
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TorrentFile {
    // TODO: How to do `& str`
    pub(crate) announce: String,
    // comment: &'a str,
    pub(crate) info: MetaInfo,
}

#[derive(Debug, thiserror::Error)]
pub enum TorrentFileError {
    #[error(transparent)]
    DeserializeError(#[from] BencodeDeserializationError),

    #[error("failed to open torrent file")]
    OpenError(#[from] std::io::Error),
}

impl TorrentFile {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, TorrentFileError> {
        let mut file = File::open(path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;

        let mut deserializer = BencodeDeserializer::new(&bytes);
        Ok(Self::deserialize(&mut deserializer)?)
    }

    pub fn meta_hash(&self) -> [u8; 20] {
        // TODO: Change this unwrap to error
        let raw_meta = to_bencode(&self.info).unwrap();

        let mut hasher = sha1::Sha1::new();
        hasher.update(&raw_meta);
        let hash = hasher.finalize();
        hash.into()
    }
}

#[derive(Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct MetaInfo {
    pub length: usize,
    pub name: String,
    //
    #[serde(rename = "piece length")]
    pub piece_length: usize,
    // pieces: &'a [u8],
    #[serde(with = "serde_bytes")]
    pub pieces: Vec<u8>,
}

impl MetaInfo {
    pub fn as_piece_infos(&self) -> impl Iterator<Item = PieceInfo> + '_ {
        let pieces_total = self.length / SIXTEEN_KIBIBYTES as usize;
        println!("Pieces total: {}", pieces_total);

        (0..=pieces_total).map(move |piece_index| {

            let length = if piece_index < pieces_total {
                SIXTEEN_KIBIBYTES
            } else {

                self.length as u64 - ((pieces_total) as u64 * SIXTEEN_KIBIBYTES)
            };
            PieceInfo {
                index: piece_index as u32,
                begin_bytes_offset: piece_index as u32 * SIXTEEN_KIBIBYTES as u32,
                length_bytes: length as u32,
            }
        })
    }
}
