use super::hash::{Hashable, H256};

/// A Merkle tree.
#[derive(Debug, Default)]
pub struct MerkleTree {
    level_hashes: Vec<Vec<H256>>,
    Root: H256,
}

impl MerkleTree {

    pub fn new<T>(data: &[T]) -> Self where T: Hashable, {
        
        let mut level_hash: Vec<H256> = Vec::new();
        let mut level_hashes : Vec<Vec<H256>> = Vec::new();
        
        for it in data.iter() {
            level_hash.push(it.hash());
        }

        /*for i in level_hash{
            println!("{:?}", i);
        }*/

        if(level_hash.len() != 1 && (level_hash.len())%2 == 1)
        {
            level_hash.push(level_hash[level_hash.len()-1]);
        }

        while level_hash.len() != 1 {

            if(level_hash.len() % 2 != 0)
            {
                //println!("{:?}", "odd!");
                level_hash.push(level_hash[level_hash.len()-1]);
            }
            
            let cur_size = level_hash.len();
            level_hashes.push(level_hash.clone());

            for i in (0..cur_size).step_by(2) {
                
                
                let tmp1 = level_hash[i].clone();
                let tmp2 = level_hash[i+1].clone();
                let v1 = tmp1.as_ref();
                let v2 = tmp2.as_ref();
                let concatenation = [v1, v2].concat();
                //println!("{:?}", &concatenation);

                let hash_value = ring::digest::digest(&ring::digest::SHA256, &concatenation);
                let new_hash = H256::from(hash_value);
                //println!("{}", new_hash);
                level_hash.push(new_hash.clone());

            }
            level_hash = level_hash[cur_size..].to_vec();
        }
        level_hashes.push(level_hash.clone());
        //println!("{:?}", level_hashes[2].len());
        Self{level_hashes : level_hashes, Root : level_hash[0]}
    }

    pub fn root(&self) -> H256 {
        return self.Root;
    }

    /// Returns the Merkle Proof of data at index i
    pub fn proof(&self, index: usize) -> Vec<H256> {
        
        let mut proofs: Vec<H256> = Vec::new();
        let levels = self.level_hashes.len();
        let mut Idx = index;

        for i in 0..(levels-1) {
            let quotient = Idx/2;
            let remain = Idx%2;

            if remain == 0 {
                proofs.push(self.level_hashes[i][2*quotient + 1].clone());
            }
            else {
                proofs.push(self.level_hashes[i][2*quotient].clone());
            }
            Idx /= 2;
        }

        return proofs;
    }
}

/// Verify that the datum hash with a vector of proofs will produce the Merkle root. Also need the
/// index of datum and `leaf_size`, the total number of leaves.
pub fn verify(root: &H256, datum: &H256, proof: &[H256], index: usize, leaf_size: usize) -> bool {
    
    let proof_num = proof.len();
    let mut cur_hash = datum.clone();
    let mut Idx = index;
    let mut concatenation =  [cur_hash.as_ref(), cur_hash.as_ref()].concat();

    for i in (0..proof_num) {
        if(Idx % 2 == 0)
        {
            concatenation = [cur_hash.as_ref(), proof[i].as_ref()].concat();
        }
        else
        {
            concatenation = [proof[i].as_ref(), cur_hash.as_ref()].concat();
        }
        let hash_value = ring::digest::digest(&ring::digest::SHA256, &concatenation);
        cur_hash = H256::from(hash_value);
        Idx /= 2;
    }

    if cur_hash == *root {
        //println!("{}", "true");
        return true;
    }
    else {
        //println!("{}", "false");
        return false;
    }
    
}

#[cfg(test)]
mod tests {
    use crate::crypto::hash::H256;
    use super::*;

    macro_rules! gen_merkle_tree_data {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
            ]
        }};
    }

    macro_rules! gen_merkle_tree_data_v1 {
        () => {{
            vec![
                (hex!("0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010202")).into(),
                (hex!("010101010101010101010101010101010101010101010101010101010101020a")).into(),
                (hex!("010101010101010101010101010101010101010101010101010101010101020f")).into(),
                (hex!("01010101010101010101010101010101010101010101010101010101010102aa")).into(),
                (hex!("01010101010101010101010101010101010101010101010101010101010102ff")).into(),
                (hex!("0101010101010101010101010101010101010101010101010101010101010aaa")).into(),
            ]
        }};
    }    

    #[test]
    fn root() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        //let test = input_data[0].hash();
        //println!("{}", input_data.capacity());
        let merkle_tree = MerkleTree::new(&input_data);
        let root = merkle_tree.root();
        assert_eq!(
            root,
            (hex!("6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920")).into()
        );
        // "b69566be6e1720872f73651d1851a0eae0060a132cf0f64a0ffaea248de6cba0" is the hash of
        // "0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d0a0b0c0d0e0f0e0d"
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
        // "6b787718210e0b3b608814e04e61fde06d0df794319a12162f287412df3ec920" is the hash of
        // the concatenation of these two hashes "b69..." and "965..."
        // notice that the order of these two matters
    }

    #[test]
    fn proof() {
        let input_data: Vec<H256> = gen_merkle_tree_data!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert_eq!(proof,
                   vec![hex!("965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f").into()]
        );
        // "965b093a75a75895a351786dd7a188515173f6928a8af8c9baa4dcff268a4f0f" is the hash of
        // "0101010101010101010101010101010101010101010101010101010101010202"
    }

    #[test]
    fn verifying() {
        //let input_data: Vec<H256> = gen_merkle_tree_data!();
        let input_data: Vec<H256> = gen_merkle_tree_data_v1!();
        let merkle_tree = MerkleTree::new(&input_data);
        let proof = merkle_tree.proof(0);
        assert!(verify(&merkle_tree.root(), &input_data[0].hash(), &proof, 0, input_data.len()));
    }
}
