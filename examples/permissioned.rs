use simplelog::{
    ColorChoice, CombinedLogger, Config as LogConfig, LevelFilter, TermLogger, TerminalMode,
};
use std::env::args;
use std::net::SocketAddr;

use chamomile::prelude::{start, Config, Peer, ReceiveMessage, SendMessage};

#[tokio::main]
async fn main() {
    CombinedLogger::init(vec![TermLogger::new(
        LevelFilter::Debug,
        LogConfig::default(),
        TerminalMode::Mixed,
        ColorChoice::Auto,
    )])
    .unwrap();

    let self_addr: SocketAddr = args()
        .nth(1)
        .expect("missing path")
        .parse()
        .expect("invalid addr");

    let mut config = Config::default(Peer::socket(self_addr));
    config.permission = true;

    let (peer_id, send, mut recv) = start(config).await.unwrap();
    println!("peer id: {}", peer_id.to_hex());

    if args().nth(2).is_some() {
        let remote_addr: SocketAddr = args().nth(2).unwrap().parse().expect("invalid addr");
        println!("start connect to remote: {}", remote_addr);
        send.send(SendMessage::Connect(Peer::socket(remote_addr)))
            .await
            .expect("channel failure!");
    }

    while let Some(message) = recv.recv().await {
        match message {
            ReceiveMessage::Data(peer_id, bytes) => {
                println!("Recv data from: {}, {:?}", peer_id.short_show(), bytes);
            }
            ReceiveMessage::StableConnect(peer, join_data) => {
                println!("Peer join: {:?}, join data: {:?}", peer, join_data);
                send.send(SendMessage::StableResult(0, peer, true, false, vec![1]))
                    .await
                    .expect("channel failure!");
            }
            ReceiveMessage::StableResult(peer, is_ok, data) => {
                println!("Peer Join Result: {:?} {}, data: {:?}", peer, is_ok, data);
            }
            ReceiveMessage::ResultConnect(from, _data) => {
                println!("Recv Result Connect {:?}", from);
            }
            ReceiveMessage::StableLeave(peer_id) => {
                println!("Peer_leave: {:?}", peer_id);
            }
            ReceiveMessage::Stream(..) => {
                panic!("Not stream");
            }
            ReceiveMessage::Delivery(..) => {}
            ReceiveMessage::NetworkLost => {
                println!("No peers conneced.")
            }
        }
    }
}
