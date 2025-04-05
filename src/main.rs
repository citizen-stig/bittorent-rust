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

    let client = torrent::network::BitTorrentClient::new();

    let peers = client.get_peers(&torrent_file);

    for peer in peers {
        println!("{:?}", peer);
    }
}
