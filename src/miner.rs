use crate::network::server::Handle as ServerHandle;
use log::info;
use crate::blockchain::Blockchain;
use crate::block::{Block, Content, Header};
use crate::crypto::merkle::{MerkleTree};
use crate::transaction::{Transaction};
use crate::crypto::hash::{H256, Hashable};
use crate::network::message::Message;
use crossbeam::channel::{unbounded, Receiver, Sender, TryRecvError};
use std::time;
use std::sync::{Arc, Mutex};
use std::thread;
extern crate rand;
use rand::Rng;

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
}

#[derive(Clone)]
pub struct Handle {
    /// Channel for sending signal to the miner thread
    control_chan: Sender<ControlSignal>,
}

pub fn new(server: &ServerHandle, blockchain: &Arc<Mutex<Blockchain>>) -> (Context, Handle) {
    let (signal_chan_sender, signal_chan_receiver) = unbounded();

    let ctx = Context {
        control_chan: signal_chan_receiver,
        operating_state: OperatingState::Paused,
        server: server.clone(),
        blockchain: Arc::clone(blockchain),
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

            let mut transactions: Vec<Transaction> = Vec::new();    
            let mut rng = rand::thread_rng();
            let In : u8 = rng.gen();
            let Out : u8 = rng.gen();
            let transaction = Transaction{Input: In, Output: Out};
            transactions.push(transaction);
            let merkletree = MerkleTree::new(&transactions);
            let nonce : u32 = rng.gen();
            let content = Content{data : transactions};
            let header = Header{parent : parent, nonce : nonce, difficulty : difficulty, timestamp : timestamp, merkle_root : merkletree.root(
                )};
            let new_block = Block{header : header, content : content}; //

            println!("The block mined: {}", block_counter);
            if(new_block.hash() <= difficulty)
            {
                block_counter += 1;
                self.blockchain.lock().unwrap().insert(&new_block);
                //println!("{:?}", self.blockchain.lock().unwrap().next_len - 1);
                let mut new_blockHash: Vec<H256> = Vec::new();
                new_blockHash.push(new_block.hash());
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
