use crate::network::server::Handle as ServerHandle;
use log::info;
use crate::blockchain::{Blockchain, State};
use crate::block::{Block, Content, Header};
use crate::crypto::merkle::{MerkleTree};
use crate::transaction::{Transaction, SignedTransaction};
use crate::crypto::hash::{H256, H160, Hashable};
use crate::network::message::Message;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::time;
use std::sync::{Arc, Mutex};
use std::thread;
extern crate rand;
use rand::Rng;
use crate::crypto::key_pair;
use crate::transaction::sign;
use ring::signature::{Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters};
use std::collections::HashMap;


enum ControlSignal {
    Start(u64), // the number controls the lambda of interval between block generation
    Exit,
}

enum OperatingState {
    Paused,
    Run(u64),
    ShutDown,
}

pub struct Context {
    /// Channel for receiving control signal
    control_chan: Receiver<ControlSignal>,
    operating_state: OperatingState,
    server: ServerHandle,
    blockchain : Arc<Mutex<Blockchain>>,
    mempool : Arc<Mutex<HashMap<H256, SignedTransaction>>>,
    states : Arc<Mutex<State>>,
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(server: &ServerHandle, blockchain: &Arc<Mutex<Blockchain>>, mempool: &Arc<Mutex<HashMap<H256, SignedTransaction>>>, states: &Arc<Mutex<State>>) -> (Context, Handle) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        server: server.clone(),
        blockchain: Arc::clone(blockchain),
        mempool: Arc::clone(mempool),
        states: Arc::clone(states),
    };

    let handle = Handle {
        control_chan: signal_chan_sender,
    };

    (ctx, handle)
}

impl Handle {
    pub fn exit(&self) {
        self.control_chan.send(ControlSignal::Exit).unwrap();
    }

    pub fn start(&self, lambda: u64) {
        self.control_chan
            .send(ControlSignal::Start(lambda))
            .unwrap();
    }

}

impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("miner".to_string())
            .spawn(move || {
                self.miner_loop();
            })
            .unwrap();
        info!("Miner initialized into paused mode");
    }

    fn handle_control_signal(&mut self, signal: ControlSignal) {
        match signal {
            ControlSignal::Exit => {
                info!("Miner shutting down");
                self.operating_state = OperatingState::ShutDown;
            }
            ControlSignal::Start(i) => {
                info!("Miner starting in continuous mode with lambda {}", i);
                self.operating_state = OperatingState::Run(i);
            }
        }
    }

    fn miner_loop(&mut self) {
        // main mining loop
        let mut block_counter = 0;
        loop {
            // check and react to control signals
            match self.operating_state {
                OperatingState::Paused => {
                    let signal = self.control_chan.recv().unwrap();
                    self.handle_control_signal(signal);
                    continue;
                }
                OperatingState::ShutDown => {
                    return;
                }
                _ => match self.control_chan.try_recv() {
                    Ok(signal) => {
                        self.handle_control_signal(signal);
                    }
                    Err(TryRecvError::Empty) => {}
                    Err(TryRecvError::Disconnected) => panic!("Miner control channel detached"),
                },
            }
            if let OperatingState::ShutDown = self.operating_state {
                return;
            }

            // TODO: actual mining
            let parent = self.blockchain.lock().unwrap().tip();
            let timestamp;
            match time::SystemTime::now().duration_since(time::UNIX_EPOCH) 
            {
                Ok(n) => timestamp = n.as_millis(),
                Err(_) => panic!("SystemTime before UNIX EPOCH!"),
            }
            let difficulty = self.blockchain.lock().unwrap().hash_blocks[&parent].header.difficulty;

            let mut rng = rand::thread_rng();
            let nonce : u32 = rng.gen();
            let random_recipient: Vec<u8> = (0..20).map(|_| 1).collect();
            let mut raw_bytes_recipient = [0; 20];
            raw_bytes_recipient.copy_from_slice(&random_recipient);
            let Recipient : H160 =(&raw_bytes_recipient).into();
            let val : u32 = rng.gen();
            let mut accountNonce : u16 = 0;

            //sign the transaction and store it in SignedTransactions vector
            let transaction = Transaction{recipAddress : Recipient, val : val, accountNonce : accountNonce};
            let key = key_pair::random();
            let signature = sign(&transaction, &key);
            let hash_key : H256 = ring::digest::digest(&ring::digest::SHA256, key.public_key().as_ref()).into();
            let hash_key_20bytes : H160 = hash_key.as_ref()[12..=31].into();
            let signed_transaction = SignedTransaction{Transaction: transaction, Signature : signature.as_ref().to_vec(), public_key : key.public_key().as_ref().to_vec(), sender_addr : hash_key_20bytes};
            
            //in each block trial, we should remove them if we successfully mine the block
            let mut SignedTransactions: Vec<SignedTransaction> = Vec::new();
            let txs_perBlock = 2;
            let mut counter = 0;
            for (key, val) in self.mempool.lock().unwrap().iter() {
                if (counter == txs_perBlock) {
                    break;
                }
                SignedTransactions.push(val.clone());
                counter += 1;
            }
            //SignedTransactions.push(signed_transaction);
            if (SignedTransactions.len() == 0) {
                continue;
            }
            let merkle_tree = MerkleTree::new(&SignedTransactions);
            let root = merkle_tree.root();
     
            let header = Header{parent : parent, nonce : nonce, difficulty : difficulty, timestamp : 0, merkle_root : root};
            let content = Content{data : SignedTransactions};
            let new_block = Block{header : header, content : content};


            if(new_block.hash() <= difficulty)
            {
                block_counter += 1;
                println!("The current number of blocks mined: {} blocks", block_counter);
                self.blockchain.lock().unwrap().insert(&new_block);
                //println!("{:?}", self.blockchain.lock().unwrap().next_len - 1);

                let mut new_blockHash: Vec<H256> = Vec::new();
                new_blockHash.push(new_block.hash());
                let size = new_block.content.data.len(); // remove corresponding transactions in the blocks
                for i in (0..size) {
                    if (self.mempool.lock().unwrap().contains_key(&new_block.content.data[i].hash())) {
                        self.mempool.lock().unwrap().remove(&new_block.content.data[i].hash());

                        let sender_addr = new_block.content.data[i].sender_addr;
                        let recver_addr = new_block.content.data[i].Transaction.recipAddress;
                        let trans_money = new_block.content.data[i].Transaction.val; 
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
                //let longest_chain = self.blockchain.lock().unwrap().all_blocks_in_longest_chain();
                self.server.broadcast(Message::NewBlockHashes(new_blockHash));
            }

            if let OperatingState::Run(i) = self.operating_state {
                if i != 0 {
                    let interval = time::Duration::from_micros(i as u64);
                    thread::sleep(interval);
                }
            }
        }
    }
}
