use super::message::Message;
use super::peer;
use crate::network::server::Handle as ServerHandle;
use crossbeam::channel;
use log::{debug, warn};
use std::sync::{Arc, Mutex};
use crate::blockchain::{Blockchain, State};
use crate::block::{Block, Header, Content};
use crate::crypto::hash::{H256, Hashable, H160};
use std::thread;
use std::time;
use serde::{Serialize,Deserialize};
use std::collections::HashMap;
use crate::transaction::{Transaction, SignedTransaction};
use crate::transaction::verify;


pub struct Orphan {
    orphan_blocks: HashMap<H256, Block>,
}

#[derive(Clone)]
pub struct Context {
    msg_chan: channel::Receiver<(Vec<u8>, peer::Handle)>,
    num_worker: usize,
    server: ServerHandle,
    blockchain : Arc<Mutex<Blockchain>>,
    mempool : Arc<Mutex<HashMap<H256, SignedTransaction>>>,
    states : Arc<Mutex<State>>,
}

pub fn new(
    num_worker: usize,
    msg_src: channel::Receiver<(Vec<u8>, peer::Handle)>,
    server: &ServerHandle,
    blockchain: &Arc<Mutex<Blockchain>>,
    mempool: &Arc<Mutex<HashMap<H256, SignedTransaction>>>,
    states: &Arc<Mutex<State>>,
) -> Context {
    Context {
        msg_chan: msg_src,
        num_worker,
        server: server.clone(),
        blockchain: Arc::clone(blockchain),
        mempool: Arc::clone(mempool),
        states: Arc::clone(states),
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

        let mut orphan_blocks: HashMap<H256, Block> = HashMap::new();
        let mut orphan_buffer = Orphan {orphan_blocks: orphan_blocks};
        let mut counter = 0;
        let mut sum = 0;
        let mut mark = 0;
        let mut start = 0;
        loop {
            println!("{:?}", self.states.lock().unwrap().accountMaping);
            //println!("Total blockchain len: {}", self.blockchain.lock().unwrap().hash_blocks.len());
            //println!("Orphan size: {}", orphan.len());
            //println!("sum: {:?}", sum);
            //println!("counter: {:?}", counter);

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
                    println!("Total number of blocks in blockchain: {} blocks", self.blockchain.lock().unwrap().hash_blocks.len());
                    println!("The number of orphan blocks: {} blocks", orphan_buffer.orphan_blocks.len());
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
                    println!("Total number of blocks in blockchain: {} blocks", self.blockchain.lock().unwrap().hash_blocks.len());
                    println!("The number of orphan blocks: {} blocks", orphan_buffer.orphan_blocks.len());
                    peer.write(Message::Blocks(exist_blocks));

                }
                Message::Blocks(blocks) => {
                    debug!("Blocks");
                    let size = blocks.len();
                
                    let mut orphan_size = orphan_buffer.orphan_blocks.len();
                    for i in (0..size) {
                        //check 
                        let mut valid_block = true;
                        let signedTxs_size = blocks[i].content.data.len();
                        for j in (0..signedTxs_size) {
                            //Transaction signature check
                            let public_key_bytes : &[u8] = &blocks[i].content.data[j].public_key;
                            let hash_key : H256 = ring::digest::digest(&ring::digest::SHA256, public_key_bytes).into();
                            let hash_key_20bytes : H160 = hash_key.as_ref()[12..=31].into();

                            if(!verify(&blocks[i].content.data[j].Transaction, &blocks[i].content.data[j].public_key, &blocks[i].content.data[j].Signature)){
                                //we need to discard the block or not is not yet decided
                                valid_block = false;
                            }
                            else if(hash_key_20bytes != blocks[i].content.data[j].sender_addr){
                                valid_block = false;
                            }
                        }

                        if (!valid_block) {
                            continue;
                        }

                        //transactions in the blocks are valid, we should remove them from mempool and update the states

                        for i in (0..size) {
                            let signedTxs_size = blocks[i].content.data.len();
                            for j in (0..signedTxs_size) {
                               if (self.mempool.lock().unwrap().contains_key(&blocks[i].content.data[j].hash())) {
                                    self.mempool.lock().unwrap().remove(&blocks[i].content.data[j].hash()); 
                                    let sender_addr = blocks[i].content.data[j].sender_addr;
                                    let recver_addr = blocks[i].content.data[j].Transaction.recipAddress;
                                    let trans_money = blocks[i].content.data[j].Transaction.val;

                                    if (self.states.lock().unwrap().accountMaping[&sender_addr].1 >= trans_money) {
                                        if let Some(x) = self.states.lock().unwrap().accountMaping.get_mut(&sender_addr) {
                                            x.1 -= trans_money; 
                                        }
                                        if let Some(y) = self.states.lock().unwrap().accountMaping.get_mut(&recver_addr) {
                                            y.1 += trans_money; 
                                        }
                                    }

                                } 
                            }
                        }
                        
                        if (!self.blockchain.lock().unwrap().hash_blocks.contains_key(&blocks[i].hash())) {
                            if (!self.blockchain.lock().unwrap().hash_blocks.contains_key(&blocks[i].header.parent)) {
                                orphan_buffer.orphan_blocks.insert(blocks[i].header.parent, blocks[i].clone());
                                println!("The number of orphan blocks is increased to {} blocks", orphan_buffer.orphan_blocks.len());
                                let mut parent_hash: Vec<H256> = Vec::new();
                                parent_hash.push(blocks[i].header.parent);
                                peer.write(Message::GetBlocks(parent_hash.clone()));
                            }
                            else {
                                let par = self.blockchain.lock().unwrap().tip();
                                let diff = self.blockchain.lock().unwrap().hash_blocks[&par].header.difficulty;
                                if (blocks[i].hash() <= diff) {

                                    // get network delay
                                    let mut timestamp = blocks[i].header.timestamp;
                                    let mut cur_time;
                                    match time::SystemTime::now().duration_since(time::UNIX_EPOCH) 
                                    {
                                        Ok(n) => cur_time = n.as_millis(),
                                        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
                                    }
                                    let mut delay = cur_time - timestamp;
                                    println!("Network delay: {:?} ms", delay);

                                    // get the average delay
                                    sum += delay;
                                    counter += 1;
                                    let avg: f32 = (sum as f32)/(counter as f32);
                                    println!("Average network delay: {:?} ms", avg);

                                    // get block size
                                    let serialized: Vec<u8> = bincode::serialize(&blocks[i]).unwrap();
                                    let block_size = serialized.len();
                                    println!("Block size: {:?}", block_size);

                                    // get duration
                                    // setting start as starting time
                                    if mark == 0
                                    {
                                        start = cur_time;
                                        mark = 1;
                                    }
                                    let time_diff = cur_time - start;
                                    let dura = (time_diff as f32)/(1000 as f32);
                                    println!("Time elapsed: {:?} seconds", dura.clone());

                                    self.blockchain.lock().unwrap().insert(&blocks[i]);
                                    self.blockchain.lock().unwrap().chainState.insert(blocks[i].hash(), self.states.lock().unwrap().clone());
                                    //insert new block to blockchain, so we need to remove SignedTransaction inside this block
                                    let size = blocks[i].content.data.len();
                                    for j in (0..size) {
                                       if (self.mempool.lock().unwrap().contains_key(&blocks[i].content.data[j].hash())) {
                                            self.mempool.lock().unwrap().remove(&blocks[i].content.data[j].hash());
                                        }
                                    }
                                    let mut new_blockHash: Vec<H256> = Vec::new();
                                    new_blockHash.push(blocks[i].hash());
                                    self.server.broadcast(Message::NewBlockHashes(new_blockHash));
                                }
                            }

                            if (orphan_buffer.orphan_blocks.contains_key(&blocks[0].hash())) {
                                self.blockchain.lock().unwrap().insert(&orphan_buffer.orphan_blocks[&blocks[0].hash()]); 
                                //remove corresponding txs in the inserted block from mempool
                                let size = blocks[0].content.data.len();
                                for j in (0..size) {
                                    if (self.mempool.lock().unwrap().contains_key(&blocks[0].content.data[j].hash())) {
                                        self.mempool.lock().unwrap().remove(&blocks[0].content.data[j].hash());
                                    }
                                }

                                let mut new_blockHash_orphan: Vec<H256> = Vec::new();
                                new_blockHash_orphan.push(orphan_buffer.orphan_blocks[&blocks[0].hash()].hash());
                                self.server.broadcast(Message::NewBlockHashes(new_blockHash_orphan));
                                orphan_buffer.orphan_blocks.remove(&blocks[0].hash());
                                println!("The number of orphan blocks is decreased to {} blocks", orphan_buffer.orphan_blocks.len());
                                break;
                            }                        
                        }
                    }
                    let longest_chain = self.blockchain.lock().unwrap().all_blocks_in_longest_chain();
                    println!("{:?}", longest_chain);
                    println!("Total number of blocks in blockchain: {} blocks", self.blockchain.lock().unwrap().hash_blocks.len());
                    println!("The number of orphan blocks: {} blocks", orphan_buffer.orphan_blocks.len());
                }

                Message::NewTransactionHashes(trans_hashes) => {
                    debug!("NewTransactionHashes");
                    let size = trans_hashes.len();
                    for i in (0..size) {
                        let exist = self.mempool.lock().unwrap().contains_key(&trans_hashes[i]);
                        if(!exist)
                        {
                            peer.write(Message::GetTransactions(trans_hashes.clone()));
                            break;
                        }
                    }
                }

                Message::GetTransactions(get_trans) => {
                    debug!("GetTransactions");
                    let size = get_trans.len();
                    let mut exist = true;
                    for i in (0..size) {
                        if(!self.mempool.lock().unwrap().contains_key(&get_trans[i]))
                        {
                            exist = false;
                            break;
                        }
                    }
                    let mut exist_trans : Vec<SignedTransaction> = Vec::new();
                    if exist {
                        for i in (0..size) {
                            exist_trans.push(self.mempool.lock().unwrap()[&get_trans[i]].clone());
                        }
                    }
                    peer.write(Message::Transactions(exist_trans));

                }

                Message::Transactions(trans) => {
                    debug!("Transactions");
                    let size = trans.len();
                    let mut new_transHash: Vec<H256> = Vec::new();
                    let mut verified = true;
                    for i in (0..size) {
                        if(verify(&trans[i].Transaction, &trans[i].public_key, &trans[i].Signature)){
                            //put into mempool
                            self.mempool.lock().unwrap().insert(trans[i].hash(), trans[i].clone());
                            new_transHash.push(trans[i].hash());
                        }
                        else {
                            //verify fail, need to ask the original node to send transaction again
                            verified = false;
                            break;
                        }
                    }

                    //If there is unverified transaction, pack it in vector and re-ask for it
                    if (!verified) {
                        let mut ask_trans : Vec<H256> = Vec::new();
                        for j in (0..size) {
                            ask_trans.push(trans[j].hash());
                        }
                        peer.write(Message::GetTransactions(ask_trans));
                        continue;
                    }
                    println!("mempool size: {}", self.mempool.lock().unwrap().len());
                    self.server.broadcast(Message::NewTransactionHashes(new_transHash));

                }

            }
        }
    }
}
