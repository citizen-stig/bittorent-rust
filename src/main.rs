use crate::torrent::network::PeerClient;

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
    for peer_addr in peers {
        println!("{:?}", peer_addr);
        let _peer_client = PeerClient::new(peer_addr, hash);
    }
}
