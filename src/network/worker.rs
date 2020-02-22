use super::message::Message;
use super::peer;
use crate::network::server::Handle as ServerHandle;
use crossbeam::channel;
use log::{debug, warn};
use std::sync::{Arc, Mutex};
use crate::blockchain::Blockchain;
use crate::block::{Block, Header, Content};
use crate::crypto::hash::{H256, Hashable};
use std::thread;

#[derive(Clone)]
pub struct Context {
    msg_chan: channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    blockchain : Arc<Mutex<Blockchain>>,
}

pub fn new(
    num_worker: usize,
    msg_src: channel::Receiver<(Vec<u8>, peer::Handle)>,
    server: &ServerHandle,
    blockchain: &Arc<Mutex<Blockchain>>
) -> Context {
    Context {
        msg_chan: msg_src,
        num_worker,
        server: server.clone(),
        blockchain: Arc::clone(blockchain),
    }
}

impl Context {
    pub fn start(self) {
        let num_worker = self.num_worker;
        for i in 0..num_worker {
            let cloned = self.clone();
            thread::spawn(move || {
                cloned.worker_loop();
                warn!("Worker thread {} exited", i);
            });
        }
    }

    fn worker_loop(&self) {
        loop {
            let msg = self.msg_chan.recv().unwrap();
            let (msg, peer) = msg;
            let msg: Message = bincode::deserialize(&msg).unwrap();
            match msg {
                Message::Ping(nonce) => {
                    debug!("Ping: {}", nonce);
                    peer.write(Message::Pong(nonce.to_string()));
                }
                Message::Pong(nonce) => {
                    debug!("Pong: {}", nonce);
                }
                Message::NewBlockHashes(block_hashes) => {
                    //debug!("NewBlockHashes");
                    let size = block_hashes.len();
                    for i in (0..size) {
                        let exist = self.blockchain.lock().unwrap().hash_blocks.contains_key(&block_hashes[i]);
                        if(!exist)
                        {
                            peer.write(Message::GetBlocks(block_hashes.clone()));
                            break;
                        }
                    }
                }
                Message::GetBlocks(getblocks) => {
                    //debug!("GetBlocks");
                    let size = getblocks.len();
                    let mut exist = true;
                    for i in (0..size) {
                        if(!self.blockchain.lock().unwrap().hash_blocks.contains_key(&getblocks[i]))
                        {
                            exist = false;
                            break;
                        }
                    }
                    let mut exist_blocks : Vec<Block> = Vec::new();
                    if exist {
                        for i in (0..size) {
                            exist_blocks.push(self.blockchain.lock().unwrap().hash_blocks[&getblocks[i]].clone());
                        }
                    }

                    peer.write(Message::Blocks(exist_blocks));

                }
                Message::Blocks(blocks) => {
                    //debug!("Blocks");
                    let size = blocks.len();
                    for i in (0..size) {
                        if (!self.blockchain.lock().unwrap().hash_blocks.contains_key(&blocks[i].hash())) {
                            self.blockchain.lock().unwrap().insert(&blocks[i]);
                        }
                    }
                }
            }
        }
    }
}
