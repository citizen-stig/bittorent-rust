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
        let piece_length = self.piece_length as u64;
        let block_size = SIXTEEN_KIBIBYTES;

        let num_pieces = (self.length as u64 + piece_length - 1) / piece_length;

        let mut requests = Vec::new();

        for piece_idx in 0..num_pieces {
            let piece_size = if piece_idx == num_pieces - 1 {
                self.length as u64 - (piece_idx * piece_length)
            } else {
                piece_length
            };

            let num_blocks = (piece_size + block_size - 1) / block_size;

            for block_idx in 0..num_blocks {
                let begin_offset = block_idx * block_size;
                let block_length = if block_idx == num_blocks - 1 {
                    piece_size - (block_idx * block_size)
                } else {
                    block_size
                };

                requests.push(PieceInfo {
                    index: piece_idx as u32,
                    begin_bytes_offset: begin_offset as u32,
                    length_bytes: block_length as u32,
                });
            }
        }

        requests.into_iter()
    }
}
