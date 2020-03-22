use serde::{Serialize,Deserialize};
use ring::signature::{Ed25519KeyPair, Signature, KeyPair, VerificationAlgorithm, EdDSAParameters};
use crate::crypto::hash::{H256, H160, Hashable};



#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct Transaction {
    pub recipAddress : H160,
    pub val : u32,
    pub accountNonce : u16,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct SignedTransaction {
    pub Transaction : Transaction,
    pub public_key : Vec<u8>,
    pub Signature :  Vec<u8>, // by function into_vec
    pub sender_addr : H160,
}

/// Create digital signature of a transaction
pub fn sign(t: &Transaction, key: &Ed25519KeyPair) -> Signature {

    let encoded = bincode::serialize(t).unwrap();
    //println!("{:?}", encoded);
    let signature = key.sign(&encoded);

    return signature;
    //unimplemented!()
}

/// Verify digital signature of a transaction, using public key instead of secret key
/*pub fn verify(t: &Transaction, public_key: &<Ed25519KeyPair as KeyPair>::PublicKey, signature: &Signature) -> bool {

    let peer_public_key_bytes = public_key.as_ref();
    //println!("{:?}", peer_public_key_bytes);
    let peer_public_key = ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, peer_public_key_bytes);
    let encoded = bincode::serialize(t).unwrap();
    let res = peer_public_key.verify(&encoded, signature.as_ref());
    //println!("{:?}", res);
    match res {
        Ok(v) => return true,
        Err(e) => return false,
    }
    //unimplemented!()
}*/

pub fn verify(t: &Transaction, public_key: &Vec<u8>, signature: &Vec<u8>) -> bool {

    let peer_public_key_bytes : &[u8] = &public_key;
    let peer_public_key = ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, peer_public_key_bytes);
    let encoded = bincode::serialize(t).unwrap();
    let signature_bytes : &[u8] = &signature;
    let res = peer_public_key.verify(&encoded, signature_bytes);
    match res {
        Ok(v) => return true,
        Err(e) => return false,
    }
    //unimplemented!()
}

#[cfg(any(test, test_utilities))]
pub mod tests {
    use super::*;
    use crate::crypto::key_pair;

    pub fn generate_random_transaction() -> Transaction {
        //Default::default()
        extern crate rand;
        use rand::Rng;

        let mut rng = rand::thread_rng();
        let In : u8 = rng.gen();
        let Out : u8 = rng.gen();
        //println!("{:?}", In);
        //println!("{:?}", Out);
        let transaction = Transaction{Input: In, Output: Out};
        return transaction;
        //unimplemented!()
    }

    #[test]
    fn sign_verify() {
        let t = generate_random_transaction();
        let key = key_pair::random();
        let signature = sign(&t, &key);
        assert!(verify(&t, &(key.public_key()), &signature));
    }
}
