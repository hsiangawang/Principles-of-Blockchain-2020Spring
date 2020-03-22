use crate::crypto::hash::{H256, H160, Hashable};
use serde::{Serialize, Deserialize};
use std::convert::TryInto;
use crate::crypto::merkle::{MerkleTree};
use crate::block::{Block, Header, Content};
use crate::transaction::{Transaction, SignedTransaction};
use crate::crypto::hash::generate_random_hash;
use crate::crypto::key_pair;
use std::collections::HashMap;
use std::collections::BTreeMap;
extern crate rand;
use rand::Rng;
use crate::transaction::sign;
use ring::signature::{Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct State {
    pub accountMaping : HashMap<H160, (u16, u32)>, // accountAddress (accountNonce, balance)
}

impl State {
    pub fn new() -> Self {
        let mut accountMaping : HashMap<H160, (u16, u32)> = HashMap::new();

        return Self{accountMaping : accountMaping};
    }
}

pub struct Blockchain {
    pub hash_blocks : HashMap<H256, Block>,
    pub genesis : Block,
    pub tip : H256,
    pub blocks_height : HashMap<H256, u16>,
    pub next_len : u16,
    pub chainState : HashMap<H256, State>,
}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {

        let mut hash_blocks: HashMap<H256, Block> = HashMap::new();
        let mut blocks_height: HashMap<H256, u16> = HashMap::new();
        let mut chainState: HashMap<H256, State> = HashMap::new();
        let mut tip : H256;
        let mut next_len : u16 = 1;

        let mut rng = rand::thread_rng();
        let nonce : u32 = 0;
        
        //to hard-code the blockchain difficulty and parent and genesis root
        let random_diff: Vec<u8> = (0..32).map(|_| 10).collect();
        let mut raw_bytes = [0; 32];
        raw_bytes.copy_from_slice(&random_diff);
        let difficulty_glob =(&raw_bytes).into();

        let random_parent: Vec<u8> = (0..32).map(|_| 1).collect();
        let mut raw_bytes_parent = [0; 32];
        raw_bytes_parent.copy_from_slice(&random_parent);
        let Parent =(&raw_bytes_parent).into();

        let random_genesisRoot: Vec<u8> = (0..32).map(|_| 5).collect();
        let mut raw_bytes_parent = [0; 32];
        raw_bytes_parent.copy_from_slice(&random_genesisRoot);
        let genesis_root =(&raw_bytes_parent).into();

        //create transaction content for genesis block
        println!("{:?}", "Start create transaction content for genesis block");
        let random_recipient: Vec<u8> = (0..20).map(|_| 1).collect();
        let mut raw_bytes_recipient = [0; 20];
        raw_bytes_recipient.copy_from_slice(&random_recipient);
        let Recipient : H160 =(&raw_bytes_recipient).into();
        let val : u32 = rng.gen();
        let mut accountNonce : u16 = 0;

        //sign the transaction and store it in SignedTransactions vector
        println!("{:?}", "Sign the transaction and store it in SignedTransactions vector");
        let mut SignedTransactions: Vec<SignedTransaction> = Vec::new();
        /*let transaction = Transaction{recipArrress : Recipient, val : val, accountNonce : accountNonce};
        let key = key_pair::random();
        let signature = sign(&transaction, &key);
        let hash_key : H256 = ring::digest::digest(&ring::digest::SHA256, key.public_key().as_ref()).into();
        let hash_key_20bytes : H160 = hash_key.as_ref()[12..=31].into();
        let signed_transaction = SignedTransaction{Transaction: transaction, Signature : signature.as_ref().to_vec(), public_key : key.public_key().as_ref().to_vec(), sender_addr : hash_key_20bytes};*/
        //SignedTransactions.push(signed_transaction);

        //let merkle_tree = MerkleTree::new(&SignedTransactions);
        //let root = merkle_tree.root();
        let root = genesis_root;
        
        let header = Header{parent : Parent, nonce : nonce, difficulty : difficulty_glob, timestamp : 0, merkle_root : root};
        let content = Content{data : SignedTransactions};
        let genesis_block = Block{header : header, content : content};
        tip = genesis_block.hash();
        hash_blocks.insert(genesis_block.hash(), genesis_block.clone());
        blocks_height.insert(genesis_block.hash(), next_len);
        next_len += 1;

        return Self{hash_blocks : hash_blocks, genesis : genesis_block, tip : tip, blocks_height : blocks_height, next_len : next_len ,chainState: chainState};
        
    }

    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {
        self.hash_blocks.insert(block.hash(), block.clone());
        println!("{:?}", "before find parent");
        let parent_height = self.blocks_height[&block.header.parent];
        //println!("{:?}", "after find parent");
        self.blocks_height.insert(block.hash(), parent_height + 1);
        //self.blocks_height.insert(block.hash(), self.next_len);
        if(self.blocks_height[&block.hash()] > self.blocks_height[&self.tip])
        {
            self.tip = block.hash();
        }
        self.next_len += 1;
    }

    /// Get the last block's hash of the longest chain
    pub fn tip(&self) -> H256 {
        return self.tip;
    }

    /// Get the last block's hash of the longest chain
    ///#[cfg(any(test, test_utilities))]
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
        let mut longest_chain: Vec<H256> = Vec::new();

        let mut pointer : H256 = self.tip;
        
        let genesis_parent = self.genesis.header.parent;

        while pointer != genesis_parent
        {
            longest_chain.push(pointer.clone());
            let cur_block : Block = self.hash_blocks[&pointer].clone();
            pointer = cur_block.header.parent;
        }
        longest_chain.reverse();

        return longest_chain;
    }
}

#[cfg(any(test, test_utilities))]
mod tests {
    use super::*;
    use crate::block::test::generate_random_block;
    use crate::crypto::hash::Hashable;

    #[test]
    fn insert_one() {
        println!("{:?}", "Start testing!!!");
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block = generate_random_block(&genesis_hash);
        println!("{:?}", block.hash());
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());
        // additional test
        let block_2 = generate_random_block(&block.hash());
        blockchain.insert(&block_2);
        assert_eq!(blockchain.tip(), block_2.hash());

        let block_3 = generate_random_block(&block_2.hash());
        blockchain.insert(&block_3);
        assert_eq!(blockchain.tip(), block_3.hash());

        let block_4 = generate_random_block(&block.hash());
        println!("{:?}", block_4.hash());
        blockchain.insert(&block_4);
        assert_eq!(blockchain.tip(), block_3.hash());

        let block_5 = generate_random_block(&block_4.hash());
        println!("{:?}", block_5.hash());
        blockchain.insert(&block_5);
        assert_eq!(blockchain.tip(), block_3.hash());

        let block_6 = generate_random_block(&block_5.hash());
        println!("{:?}", block_6.hash());
        blockchain.insert(&block_6);

        let longest_chain = blockchain.all_blocks_in_longest_chain();
        println!("{:?}", longest_chain);

        assert_eq!(blockchain.tip(), block_6.hash());
    }


}
