use simplelog::{
    ColorChoice, CombinedLogger, Config as LogConfig, LevelFilter, TermLogger, TerminalMode,
};
use std::env::args;
use std::net::SocketAddr;

use chamomile::prelude::{start, Broadcast, Config, Peer, ReceiveMessage, SendMessage};
use std::time::Duration;

#[tokio::main]
async fn main() {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Debug,
        LogConfig::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])
    .unwrap();

    let addr_str = args().nth(1).expect("missing path");
    let self_addr: SocketAddr = addr_str.parse().expect("invalid addr");

    println!("START A PERMISSIONLESS PEER. socket: {}", self_addr);
    let peer = Peer::socket(self_addr);
    // let mut peer = Peer::socket(self_addr);
    // peer.transport = chamomile_types::types::TransportType::TCP; // DEBUG different transport.

    let mut config = Config::default(peer);
    config.permission = false; // Permissionless.
    config.only_stable_data = false; // Receive all peer's data.
    config.db_dir = std::path::PathBuf::from(addr_str);

    let (peer_id, send, mut recv) = start(config).await.unwrap();
    println!("peer id: {}", peer_id.to_hex());

    if args().nth(2).is_some() {
        let remote_addr: SocketAddr = args().nth(2).unwrap().parse().expect("invalid addr");
        println!("start DHT connect to remote: {}", remote_addr);
        let _ = send
            .send(SendMessage::Connect(Peer::socket(remote_addr)))
            .await;

        println!("sleep 3s and then broadcast...");
        tokio::time::sleep(Duration::from_secs(2)).await;

        fn mod_reduce(mut i: u32) -> u8 {
            loop {
                if i > 255 {
                    i = i - 255
                } else {
                    break;
                }
            }
            i as u8
        }

        let mut bytes = vec![];
        for i in 0..10u32 {
            bytes.push(mod_reduce(i));
        }

        println!("Will send bytes: {}-{:?}", bytes.len(), &bytes);
        let _ = send
            .send(SendMessage::Broadcast(Broadcast::Gossip, bytes))
            .await;
    }

    while let Some(message) = recv.recv().await {
        match message {
            ReceiveMessage::Data(remote_id, bytes) => {
                println!(
                    "Recv permissionless data from: {}, {}-{:?}",
                    remote_id.short_show(),
                    bytes.len(),
                    bytes
                );

                // only for test circle to send-self.
                if bytes != vec![9, 9, 9, 9] {
                    let _ = send
                        .send(SendMessage::Data(9999, peer_id, vec![9, 9, 9, 9]))
                        .await;
                }
            }
            ReceiveMessage::Stream(..) => {
                panic!("Nerver here (stream)");
            }
            ReceiveMessage::StableConnect(from, data) => {
                println!("Recv peer want to build a stable connected: {:?}", data);

                let tid = 2u64;

                let _ = send
                    .send(SendMessage::StableResult(
                        tid,
                        from,
                        true,
                        false,
                        vec![3, 3, 3, 3],
                    ))
                    .await;
            }
            ReceiveMessage::StableLeave(peer_id) => {
                println!("Recv stable connected leave: {}", peer_id.to_hex());
            }
            ReceiveMessage::StableResult(peer, is_ok, remark) => {
                println!(
                    "Recv stable connected result: {:?} {} {:?}",
                    peer, is_ok, remark
                );
            }
            ReceiveMessage::ResultConnect(from, _data) => {
                println!("Recv Result Connect {:?}", from);
            }
            ReceiveMessage::Delivery(t, tid, had, _data) => {
                println!("Recv {:?} Delivery: {} {}", t, tid, had);
            }
            ReceiveMessage::NetworkLost => {
                println!("No peers conneced.")
            }
        }
    }
}
