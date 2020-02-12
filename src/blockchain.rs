use crate::crypto::hash::{H256, Hashable};
use crate::crypto::merkle::{MerkleTree};
use crate::block::{Block, Header, Content};
use crate::transaction::{Transaction};
use crate::transaction::tests::generate_random_transaction;
use std::collections::HashMap;
use std::collections::BTreeMap;
extern crate rand;
use rand::Rng;

pub struct Blockchain {
    hash_blocks : BTreeMap<H256, Block>,
    genesis : Block,
    tip : H256,
    blocks_height : BTreeMap<H256, u8>,
    next_len : u8,
}

impl Blockchain {
    /// Create a new blockchain, only containing the genesis block
    pub fn new() -> Self {

        let mut hash_blocks: BTreeMap<H256, Block> = BTreeMap::new();
        let mut blocks_height: BTreeMap<H256, u8> = BTreeMap::new();
        let mut tip : H256;
        let mut next_len = 1;

        let mut rng = rand::thread_rng();
        let nonce : u32 = rng.gen();
        let Parent = hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0a").into();
        let difficulty_glob = hex!("010101010101010101010101010101010101010101010101010101010101020a").into();
        let mut transactions: Vec<Transaction> = Vec::new();
        transactions.push(generate_random_transaction());
        let merkle_tree = MerkleTree::new(&transactions);
        let root = merkle_tree.root();
 
        let header = Header{parent : Parent, nonce : nonce, difficulty : difficulty_glob, timestamp : 0, merkle_root : root};
        let content = Content{data : transactions};
        let genesis_block = Block{header : header, content : content};
        tip = genesis_block.hash();
        hash_blocks.insert(genesis_block.hash(), genesis_block.clone());
        blocks_height.insert(genesis_block.hash(), next_len);
        next_len += 1;

        println!("{:?}", tip);
        return Self{hash_blocks : hash_blocks, genesis : genesis_block, tip : tip, blocks_height : blocks_height, next_len : next_len};
        
    }

    /// Insert a block into blockchain
    pub fn insert(&mut self, block: &Block) {
        self.hash_blocks.insert(block.hash(), block.clone());
        self.blocks_height.insert(block.hash(), self.next_len);
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
    #[cfg(any(test, test_utilities))]
    pub fn all_blocks_in_longest_chain(&self) -> Vec<H256> {
        let mut longest_chain: Vec<H256> = Vec::new();

        for (key, value) in self.hash_blocks.iter()
        {
            longest_chain.push(value.hash());
        }

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
        let mut blockchain = Blockchain::new();
        let genesis_hash = blockchain.tip();
        let block = generate_random_block(&genesis_hash);
        blockchain.insert(&block);
        assert_eq!(blockchain.tip(), block.hash());

    }
}
