use ring::rand;
use ring::signature::Ed25519KeyPair;

/// Generate a random key pair.
pub fn random() -> Ed25519KeyPair {
    let rng = rand::SystemRandom::new();
    let pkcs8_bytes = Ed25519KeyPair::generate_pkcs8(&rng).unwrap();
    Ed25519KeyPair::from_pkcs8(pkcs8_bytes.as_ref().into()).unwrap()
}

 pub fn Hardcoded() -> Ed25519KeyPair {
    let hardcode_bytes = [48, 83, 2, 1, 1, 48, 5, 6, 3, 43, 101, 112, 4, 34, 4, 32, 164, 196, 187, 131, 199, 71, 156, 239, 32, 227, 138, 181, 123, 135, 161, 30, 135, 62, 221, 229, 53, 40, 141, 194, 32, 153, 204, 201, 82, 74, 136, 52, 161, 35, 3, 33, 0, 214, 108, 94, 124, 153, 185, 216, 21, 188, 195, 246, 195, 103, 23, 29, 199, 96, 116, 165, 210, 11, 198, 245, 234, 179, 119, 199, 232, 74, 79, 155, 35];
    Ed25519KeyPair::from_pkcs8(&hardcode_bytes).unwrap()
 }
