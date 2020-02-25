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
        let mut orphan: Vec<Block> = Vec::new();
        loop {
            //println!("Total blockchain len: {}", self.blockchain.lock().unwrap().hash_blocks.len());
            println!("Orphan size: {}", orphan.len());
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
                    debug!("NewBlockHashes");
                    let size = block_hashes.len();
                    for i in (0..size) {
                        let exist = self.blockchain.lock().unwrap().hash_blocks.contains_key(&block_hashes[i]);
                        if(!exist)
                        {
                            peer.write(Message::GetBlocks(block_hashes.clone()));
                            break;
                        }
                    }
                    let longest_chain = self.blockchain.lock().unwrap().all_blocks_in_longest_chain();
                    println!("{:?}", longest_chain);
                    println!("Total blockchain len: {}", self.blockchain.lock().unwrap().hash_blocks.len());
                }
                Message::GetBlocks(getblocks) => {
                    debug!("GetBlocks");
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
                    let longest_chain = self.blockchain.lock().unwrap().all_blocks_in_longest_chain();
                    println!("{:?}", longest_chain);
                    println!("Total blockchain len: {}", self.blockchain.lock().unwrap().hash_blocks.len());
                    peer.write(Message::Blocks(exist_blocks));

                }
                Message::Blocks(blocks) => {
                    debug!("Blocks");
                    let size = blocks.len();
                    let mut orphan_size = orphan.len();
                    for i in (0..size) {
                        
                        if (!self.blockchain.lock().unwrap().hash_blocks.contains_key(&blocks[i].hash())) {
                            //let len = self.blockchain.lock().unwrap().hash_blocks.len();
                            //println!("{:?}", blocks[i].header.timestamp);
                            //println!("blocks[i].header.parent: {}", blocks[i].header.parent);
                            //println!("current blockchain's genesis parent: {}", self.blockchain.lock().unwrap().genesis.header.parent);
                            //println!("blocks_height: {}", self.blockchain.lock().unwrap().blocks_height.len());
                            if (!self.blockchain.lock().unwrap().hash_blocks.contains_key(&blocks[i].header.parent)) {
                                orphan.push(blocks[i].clone());
                                let mut parent_hash: Vec<H256> = Vec::new();
                                parent_hash.push(blocks[i].header.parent);
                                peer.write(Message::GetBlocks(parent_hash.clone()));
                            }
                            else {
                                self.blockchain.lock().unwrap().insert(&blocks[i]);
                                let mut new_blockHash: Vec<H256> = Vec::new();
                                new_blockHash.push(blocks[i].hash());
                                self.server.broadcast(Message::NewBlockHashes(new_blockHash));
                            }

                            for j in (0..orphan_size){
                                if (self.blockchain.lock().unwrap().hash_blocks.contains_key(&orphan[j].header.parent)) {
                                    self.blockchain.lock().unwrap().insert(&orphan[j]); 
                                    let mut new_blockHash_orphan: Vec<H256> = Vec::new();
                                    new_blockHash_orphan.push(orphan[j].hash());
                                    self.server.broadcast(Message::NewBlockHashes(new_blockHash_orphan));
                                    orphan.remove(j);
                                    break;
                                    //orphan_size = orphan.len();
                                }
                            }

                        }
                    }
                    let longest_chain = self.blockchain.lock().unwrap().all_blocks_in_longest_chain();
                    println!("{:?}", longest_chain);
                    println!("Total blockchain len: {}", self.blockchain.lock().unwrap().hash_blocks.len());
                }
            }
        }
    }
}
