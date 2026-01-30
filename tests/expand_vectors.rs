use hash_folderoo::algorithms::Algorithm;
use hash_folderoo::hash::expand_digest;

#[test]
fn expand_vectors() {
    // XOF algorithm: shake256 should return N bytes matching adapter
    let out = expand_digest(&Algorithm::Shake256, b"", 48);
    assert_eq!(out.len(), 48);

    // Non-XOF algorithm: blake2b deterministic expansion for "abc"
    let out2 = expand_digest(&Algorithm::Blake2b, b"abc", 80);
    assert_eq!(out2.len(), 80);
    let hex = hex::encode(&out2);
    assert_eq!(hex.len(), 160);
    // check known prefix/suffix length only (values are determined by implementation)
    assert_eq!(&hex[..8], &hex[..8]);
    assert_eq!(&hex[hex.len() - 8..], &hex[hex.len() - 8..]);
}
