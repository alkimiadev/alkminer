use alkminer::crypto::{bits_to_target, check_hash_meets_target, BlockHeader};

fn hex_to_bytes_80(hex: &str) -> [u8; 80] {
    let mut arr = [0u8; 80];
    for i in 0..80 {
        arr[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap();
    }
    arr
}

#[test]
fn test_block_0_genesis() {
    let header_hex = "0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4a29ab5f49ffff001d1dac2b7c";
    let expected_hash = "000000000019d6689c085ae165831e934ff763ae46a2a6c172b3f1b60a8ce26f";

    let header_bytes = hex_to_bytes_80(header_hex);
    let header = BlockHeader::from_bytes(&header_bytes);

    let hash = header.hash();
    let hash_hex: String = hash.iter().rev().map(|b| format!("{:02x}", b)).collect();

    assert_eq!(hash_hex, expected_hash);
}

#[test]
fn test_block_1() {
    let header_hex = "010000006fe28c0ab6f1b372c1a6a246ae63f74f931e8365e15a089c68d6190000000000982051fd1e4ba744bbbe680e1fee14677ba1a3c3540bf7b1cdb606e857233e0e61bc6649ffff001d01e36299";
    let expected_hash = "00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048";

    let header_bytes = hex_to_bytes_80(header_hex);
    let header = BlockHeader::from_bytes(&header_bytes);

    let hash = header.hash();
    let hash_hex: String = hash.iter().rev().map(|b| format!("{:02x}", b)).collect();

    assert_eq!(hash_hex, expected_hash);
}

#[test]
fn test_block_header_roundtrip() {
    let header = BlockHeader {
        version: 1,
        prev_block_hash: [0u8; 32],
        merkle_root: [1u8; 32],
        timestamp: 1234567890,
        bits: 0x1d00ffff,
        nonce: 2083236893,
    };

    let bytes = header.to_bytes();
    let restored = BlockHeader::from_bytes(&bytes);

    assert_eq!(header.version, restored.version);
    assert_eq!(header.prev_block_hash, restored.prev_block_hash);
    assert_eq!(header.merkle_root, restored.merkle_root);
    assert_eq!(header.timestamp, restored.timestamp);
    assert_eq!(header.bits, restored.bits);
    assert_eq!(header.nonce, restored.nonce);
}

#[test]
fn test_bits_to_target_genesis() {
    let bits: u32 = 0x1d00ffff;
    let target = bits_to_target(bits);

    assert_eq!(target[26], 0xff);
    assert_eq!(target[27], 0xff);
    assert_eq!(target[28], 0x00);
}

#[test]
fn test_bits_to_target_high_difficulty() {
    let bits: u32 = 0x17034219;
    let target = bits_to_target(bits);

    assert_eq!(target[20], 0x19);
    assert_eq!(target[21], 0x42);
    assert_eq!(target[22], 0x03);
}

#[test]
fn test_check_hash_meets_target_pass() {
    let mut target = [0u8; 32];
    target[31] = 0xff;

    let mut valid_hash = [0u8; 32];
    valid_hash[31] = 0xfe;

    assert!(check_hash_meets_target(&valid_hash, &target));
}

#[test]
fn test_check_hash_meets_target_fail() {
    let mut target = [0u8; 32];
    target[31] = 0xff;

    let mut invalid_hash = [0u8; 32];
    invalid_hash[31] = 0xff;
    invalid_hash[30] = 0x01;

    assert!(!check_hash_meets_target(&invalid_hash, &target));
}

#[test]
fn test_with_nonce() {
    let header = BlockHeader {
        version: 1,
        prev_block_hash: [0u8; 32],
        merkle_root: [1u8; 32],
        timestamp: 1234567890,
        bits: 0x1d00ffff,
        nonce: 0,
    };

    let modified = header.with_nonce(12345);

    assert_eq!(header.nonce, 0);
    assert_eq!(modified.nonce, 12345);
    assert_eq!(header.version, modified.version);
    assert_eq!(header.merkle_root, modified.merkle_root);
}

#[test]
fn test_with_merkle_root() {
    let header = BlockHeader {
        version: 1,
        prev_block_hash: [0u8; 32],
        merkle_root: [1u8; 32],
        timestamp: 1234567890,
        bits: 0x1d00ffff,
        nonce: 0,
    };

    let new_root = [2u8; 32];
    let modified = header.with_merkle_root(new_root);

    assert_eq!(header.merkle_root, [1u8; 32]);
    assert_eq!(modified.merkle_root, new_root);
    assert_eq!(header.nonce, modified.nonce);
}
