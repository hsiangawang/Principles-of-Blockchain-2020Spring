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
use std::collections::{HashMap, VecDeque};


pub struct Context {
    server: ServerHandle,
    mempool : Arc<Mutex<HashMap<H256, SignedTransaction>>>,
    states : Arc<Mutex<State>>,
    txs: Arc<Mutex<VecDeque<SignedTransaction>>>,
}


pub fn new(server: &ServerHandle, mempool: &Arc<Mutex<HashMap<H256, SignedTransaction>>>, states: &Arc<Mutex<State>>, txs: &Arc<Mutex<VecDeque<SignedTransaction>>>) -> Context {

    let ctx = Context {
        server: server.clone(),
        mempool: Arc::clone(mempool),
        states: Arc::clone(states),
        txs: Arc::clone(txs),
    };

    return ctx;
}


impl Context {
    pub fn start(mut self) {
        thread::Builder::new()
            .name("TransGenerator".to_string())
            .spawn(move || {
                self.generate();
            })
            .unwrap();
        info!("Generator is ready to move!");
    }

    fn generate(&mut self) {
        // main mining loop
        let mut rng = rand::thread_rng();
        let nonce : u32 = rng.gen();
        let random_recipient: Vec<u8> = (0..20).map(|_| 1).collect();
        let mut raw_bytes_recipient = [0; 20];
        raw_bytes_recipient.copy_from_slice(&random_recipient);
        let Recipient : H160 =(&raw_bytes_recipient).into();
        let mut accountNonce : u16 = 1;
        let key = key_pair::Hardcoded();
        let hash_key : H256 = ring::digest::digest(&ring::digest::SHA256, key.public_key().as_ref()).into();
        let hash_key_20bytes : H160 = hash_key.as_ref()[12..=31].into();

        //In transaction generator, we decide to let sender's address and receiver's address be hardcoded
        //The code above is to generate random key
        let senderAddress : H160 = [70, 8, 220, 215, 80, 53, 152, 74, 136, 126, 87, 62, 230, 168, 2, 10, 237, 58, 51, 50].into();
        let recverAddress : H160 = [140, 160, 200, 230, 190, 145, 185, 70, 100, 30, 122, 218, 43, 212, 90, 238, 170, 7, 122, 128].into();
        // println!("{:?}", senderAddress);
        // println!("{:?}", recverAddress);

        //State Initialization
        info!("ICO with 10,000 in two accounts");
        self.states.lock().unwrap().accountMaping.insert(senderAddress, (0, 10000));
        self.states.lock().unwrap().accountMaping.insert(recverAddress, (0, 10000));
        println!("{:?}", self.states.lock().unwrap().accountMaping);

        info!("{:?}", "Generate new transaction...");
        loop {
            //let val : u32 = rng.gen();
            info!("{:?}", self.states.lock().unwrap().accountMaping);
            let val = 1;
            let transaction = Transaction{recipAddress : recverAddress, val : val, accountNonce : accountNonce};
            let signature = sign(&transaction, &key);
            let signed_transaction = SignedTransaction{Transaction: transaction, Signature : signature.as_ref().to_vec(), public_key : key.public_key().as_ref().to_vec(), sender_addr : senderAddress};
            self.txs.lock().unwrap().push_back(signed_transaction.clone());
            self.mempool.lock().unwrap().insert(signed_transaction.hash(), signed_transaction.clone());
            println!("mempool size: {}", self.mempool.lock().unwrap().len());
            //println!("txs len: {}", self.txs.lock().unwrap().len());
            let mut new_blockHash: Vec<H256> = Vec::new();
            new_blockHash.push(signed_transaction.hash());
            self.server.broadcast(Message::NewTransactionHashes(new_blockHash));

            accountNonce += 1;

            let interval = time::Duration::from_millis(1000);
            thread::sleep(interval);
        }
        

        
    }
}
