use crate::torrent::network::{PeerClient, PeerMessage};
use sha1::{Digest, Sha1};
use std::fs::File;
use std::io::{Seek, Write};

mod bencode;
mod torrent;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let Some(torrent_path) = args.get(1) else {
        println!("No parameters were provided.");
        std::process::exit(1);
    };

    let torrent_file = torrent::meta::TorrentFile::open(torrent_path).unwrap();
    println!("Filename: {:#?}", torrent_file.info.name);
    println!("Tracker URL: {:#?}", torrent_file.announce);
    println!("Length: {}", torrent_file.info.length);

    let client = torrent::network::TorrentTrackerClient::new();

    let peers = client.get_peers(&torrent_file);

    let hash = torrent_file.meta_hash();
    // for peer_addr in peers {
    //     println!("{:?}", peer_addr);
    //     let _peer_client = PeerClient::new(peer_addr, hash);
    // }

    let first_peer = peers.first().unwrap();
    let mut peer_client = PeerClient::new(*first_peer, hash);
    // TODO: State machine
    let msg_1 = peer_client.read_message();
    println!("RECEIVED MESSAGE 1: {:?}", msg_1);
    peer_client.send_message(PeerMessage::Interested);
    println!("SENT MESSAGE 1: {:?}", PeerMessage::Interested);
    let msg_2 = peer_client.read_message();
    println!("RECEIVED MESSAGE 2: {:?}", msg_2);
    let mut pieces: Vec<Vec<u8>> = vec![Vec::new()];
    let mut current_index = 0;
    for piece_info in torrent_file.info.as_piece_infos() {
        println!("Piece info: {:?}", piece_info);
        if piece_info.index != current_index {
            current_index += 1;
            pieces.push(Vec::new());
        }
        peer_client.send_message(PeerMessage::Request(piece_info));
        let msg_l = peer_client.read_message();
        if let PeerMessage::Piece(i, j, data) = msg_l {
            println!("Piece {} {}: {}", i, j, data.len());
            pieces[current_index as usize].extend(data);
        }
    }

    let mut output_file = File::create(torrent_file.info.name).unwrap();
    output_file
        .set_len(torrent_file.info.length as u64)
        .unwrap();

    for (piece_index, (info_hash_piece, piece)) in torrent_file
        .info
        .pieces
        .chunks(20)
        .zip(pieces.iter())
        .enumerate()
    {
        let mut hasher = Sha1::new();
        hasher.update(piece);
        let hash = hasher.finalize();
        println!("Piece hash A: {}", hex::encode(info_hash_piece));
        println!("Piece hash B: {}", hex::encode(hash));

        let offset = piece_index * torrent_file.info.piece_length;
        output_file
            .seek(std::io::SeekFrom::Start(offset as u64))
            .unwrap();
        output_file.write_all(piece).unwrap();
    }
}
