use std::collections::HashMap;
use std::sync::Arc;
use tokio::{
    io::Result,
    sync::{mpsc::Sender, RwLock},
};

use chamomile_types::{
    message::ReceiveMessage,
    types::{new_io_error, TransportType},
    Peer, PeerId,
};

use crate::buffer::Buffer;
use crate::kad::KadValue;
use crate::keys::{Keypair, SessionKey};
use crate::peer_list::PeerList;
use crate::transports::{start, RemotePublic, TransportRecvMessage, TransportSendMessage};

pub(crate) struct Global {
    pub peer: Peer,
    pub key: Keypair,
    pub trans: Sender<TransportRecvMessage>,
    pub transports: Arc<RwLock<HashMap<TransportType, Sender<TransportSendMessage>>>>,
    pub out_sender: Sender<ReceiveMessage>,
    pub peer_list: Arc<RwLock<PeerList>>,
    pub buffer: Arc<RwLock<Buffer>>,
    pub is_relay_data: bool,
    pub delivery_length: usize,
}

impl Global {
    #[inline]
    pub fn peer_id(&self) -> &PeerId {
        &self.peer.id
    }

    #[inline]
    pub fn generate_remote(&self) -> (SessionKey, RemotePublic) {
        // random gennerate, so must return. no keep-loop.
        loop {
            if let Ok(session_key) = self.key.generate_session_key() {
                let remote_pk = RemotePublic(
                    self.key.public(),
                    self.peer.clone(),
                    session_key.out_bytes(),
                );
                return (session_key, remote_pk);
            }
        }
    }

    #[inline]
    pub fn complete_remote(
        &self,
        remote_key: &Keypair,
        dh_bytes: Vec<u8>,
    ) -> Option<(SessionKey, RemotePublic)> {
        if let Some(session_key) = self.key.complete_session_key(remote_key, dh_bytes) {
            let remote_pk = RemotePublic(
                self.key.public(),
                self.peer.clone(),
                session_key.out_bytes(),
            );
            Some((session_key, remote_pk))
        } else {
            None
        }
    }

    #[inline]
    pub async fn trans_send(
        &self,
        trans_type: &TransportType,
        msg: TransportSendMessage,
    ) -> Result<()> {
        let trans_lock = self.transports.read().await;
        if let Some(sender) = trans_lock.get(trans_type) {
            sender
                .send(msg)
                .await
                .map_err(|_e| new_io_error("Transport missing"))
        } else {
            drop(trans_lock);
            // start new transport to send it.
            // Only TCP & QUIC
            let main_send = self.trans.clone();
            let mut new_peer = self.peer.clone();
            new_peer.transport = *trans_type;
            new_peer.zero_port();

            let (_, trans_send, _, _) = start(&new_peer, Some(main_send)).await?;
            trans_send
                .send(msg)
                .await
                .map_err(|_e| new_io_error("Transport missing"))?;
            self.transports
                .write()
                .await
                .insert(*trans_type, trans_send);
            Ok(())
        }
    }

    #[inline]
    pub async fn out_send(&self, msg: ReceiveMessage) -> Result<()> {
        self.out_sender
            .send(msg)
            .await
            .map_err(|_e| new_io_error("Outside missing"))
    }

    pub async fn add_tmp(&self, p: PeerId, k: KadValue, d: bool) -> Vec<(u64, Vec<u8>)> {
        let mut buffer_lock = self.buffer.write().await;
        let stables = buffer_lock.remove_connect(&p);
        buffer_lock.add_tmp(p, k, d);
        drop(buffer_lock);
        stables
    }

    pub async fn add_all_tmp(
        &self,
        peer_id: PeerId,
        kv: KadValue,
        is_direct: bool,
    ) -> (Vec<(u64, Vec<u8>)>, Vec<(u64, Vec<u8>)>) {
        let mut buffer_lock = self.buffer.write().await;
        let connects = buffer_lock.remove_connect(&peer_id);
        let results = buffer_lock.remove_result(&peer_id);
        buffer_lock.add_tmp(peer_id, kv, is_direct);
        drop(buffer_lock);

        (connects, results)
    }

    pub async fn upgrade(&self, peer_id: &PeerId) -> Result<()> {
        let v_some = self.buffer.write().await.remove_tmp(peer_id);
        if let Some((v, is_d)) = v_some {
            self.peer_list.write().await.add_stable(*peer_id, v, is_d);
            Ok(())
        } else {
            self.peer_list.write().await.dht_to_stable(peer_id)
        }
    }

    pub async fn tmp_to_dht(&self, peer_id: &PeerId) -> Result<()> {
        let v_some = self.buffer.write().await.remove_tmp(peer_id);
        if let Some((v, is_d)) = v_some {
            if is_d {
                if self.peer_list.write().await.add_dht(v).await {
                    return Ok(());
                }
            }
        }
        Err(new_io_error("missing buffer"))
    }

    #[inline]
    pub async fn stable_to_dht(&self, peer_id: &PeerId) -> Result<()> {
        let mut buffer_lock = self.buffer.write().await;
        buffer_lock.remove_tmp(peer_id);
        buffer_lock.remove_stable(peer_id);
        drop(buffer_lock);

        self.peer_list.write().await.stable_to_dht(peer_id)
    }
}
