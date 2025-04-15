use crate::torrent::network::{PeerClient, PeerMessage};

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
    for piece_info in torrent_file.info.as_piece_infos() {
        println!("Piece info: {:?}", piece_info);
        peer_client.send_message(PeerMessage::Request(piece_info));
        let msg_l = peer_client.read_message();
        println!("RECEIVED MESSAGE L: {:?}", msg_l);
    }
}
