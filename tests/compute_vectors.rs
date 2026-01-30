use hash_folderoo::algorithms::Algorithm;
use hash_folderoo::hash::expand_digest;

#[test]
fn print_vectors() {
    let out1 = expand_digest(&Algorithm::Shake256, b"abc", 48);
    println!("SHAKE256_abc_48: {}", hex::encode(out1));
    let out2 = expand_digest(&Algorithm::Blake2b, b"abc", 80);
    println!("BLAKE2B_abc_80: {}", hex::encode(out2));
}
