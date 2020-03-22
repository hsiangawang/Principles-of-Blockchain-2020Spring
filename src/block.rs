use serde::{Serialize, Deserialize};
use crate::crypto::hash::{H256, Hashable};
use crate::crypto::merkle::{MerkleTree};
use crate::transaction::{Transaction, SignedTransaction};
//use crate::transaction::tests::generate_random_transaction;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
	pub header : Header,
	pub content : Content,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Header {
	pub parent : H256,
	pub nonce : u32,
	pub difficulty : H256,
	pub timestamp : u128,
	pub merkle_root : H256,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Content {
	pub data : Vec<SignedTransaction>,
}

impl Hashable for Transaction {
    fn hash(&self) -> H256 {
    	let encoded = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &encoded).into()
    }
}

impl Hashable for SignedTransaction {
    fn hash(&self) -> H256 {
    	let encoded = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &encoded).into()
    }
}


impl Hashable for Header {
    fn hash(&self) -> H256 {
        let encoded = bincode::serialize(&self).unwrap();
        ring::digest::digest(&ring::digest::SHA256, &encoded).into()
    }
}

impl Hashable for Block {
    fn hash(&self) -> H256 {
        self.header.hash()
    }
}

#[cfg(any(test, test_utilities))]
pub mod test {
    use super::*;
    use crate::crypto::hash::H256;
    extern crate rand;
    use rand::Rng;

    pub fn generate_random_block(parent: &H256) -> Block {
        
        let mut rng = rand::thread_rng();
    	let nonce : u32 = rng.gen();
    	let Parent = parent.clone();
    	let mut difficulty_glob = hex!("0101010101010101010101010101010101010101010101010101010101010202").into();
    	let mut clock_glob = 1;
    	let mut transactions: Vec<Transaction> = Vec::new();
    	//transactions.push(generate_random_transaction()); // comment to make it compiles
        let mut rng = rand::thread_rng();
        let In : u8 = rng.gen();
        let Out : u8 = rng.gen();
        //println!("{:?}", In);
        //println!("{:?}", Out);
        let transaction = Transaction{Input: In, Output: Out};
        transactions.push(transaction);
    	let merkle_tree = MerkleTree::new(&transactions);
    	let root = merkle_tree.root();
 
    	let header = Header{parent : Parent, nonce : nonce, difficulty : difficulty_glob, timestamp : clock_glob, merkle_root : root};
    	let content = Content{data : transactions};
    	return Block{header : header, content : content};
    }
}
