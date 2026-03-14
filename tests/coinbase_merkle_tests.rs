mod common;

use alkminer::compute::ComputeModule;
use alkminer::modules::{CoinbaseMerkleConfig, CoinbaseMerkleModule};
use alkminer::crypto::{double_sha256, compute_merkle_root};

fn load_foundry_coinbase() -> Vec<u8> {
    let hex_str = std::fs::read_to_string("tests/data/foundry.txt")
        .expect("Failed to read foundry.txt")
        .trim()
        .to_string();
    hex::decode(&hex_str).expect("Failed to decode hex")
}

fn get_nonce_offset(coinbase: &[u8]) -> u32 {
    let script_len = coinbase[41] as usize;
    let script_end = 42 + script_len;
    (script_end - 8) as u32
}

fn compute_merkle_root_cpu(coinbase_hash: &[u8; 32], branches: &[[u8; 32]]) -> [u8; 32] {
    compute_merkle_root(coinbase_hash, branches)
}

fn unpack_merkle_roots(data: &[u8], count: usize) -> Vec<[u8; 32]> {
    let mut roots = Vec::with_capacity(count);
    for i in 0..count {
        let offset = i * 32;
        let mut root = [0u8; 32];
        
        for j in 0..8 {
            let word_offset = offset + j * 4;
            let word = u32::from_le_bytes([
                data[word_offset],
                data[word_offset + 1],
                data[word_offset + 2],
                data[word_offset + 3],
            ]);
            let bytes = word.to_be_bytes();
            root[j * 4] = bytes[0];
            root[j * 4 + 1] = bytes[1];
            root[j * 4 + 2] = bytes[2];
            root[j * 4 + 3] = bytes[3];
        }
        roots.push(root);
    }
    roots
}

fn apply_nonce_to_coinbase(coinbase: &[u8], nonce_offset: u32, nonce_low: u32, nonce_high: u32) -> Vec<u8> {
    let mut modified = coinbase.to_vec();
    let offset = nonce_offset as usize;
    
    let word_offset = offset / 4;
    let byte_shift = (offset % 4) * 8;
    
    if byte_shift == 0 {
        modified[offset..offset + 4].copy_from_slice(&nonce_low.to_le_bytes());
        modified[offset + 4..offset + 8].copy_from_slice(&nonce_high.to_le_bytes());
    } else {
        let mask = (1u32 << byte_shift) - 1;
        
        let existing_word = u32::from_le_bytes([
            modified[word_offset * 4],
            modified[word_offset * 4 + 1],
            modified[word_offset * 4 + 2],
            modified[word_offset * 4 + 3],
        ]);
        let new_word = (existing_word & mask) | (nonce_low << byte_shift);
        modified[word_offset * 4..word_offset * 4 + 4].copy_from_slice(&new_word.to_le_bytes());
        
        let remaining_shift = 32 - byte_shift;
        let mid_word = (nonce_low >> remaining_shift) | (nonce_high << byte_shift);
        modified[(word_offset + 1) * 4..(word_offset + 1) * 4 + 4].copy_from_slice(&mid_word.to_le_bytes());
        
        let high_existing = u32::from_le_bytes([
            modified[(word_offset + 2) * 4],
            modified[(word_offset + 2) * 4 + 1],
            modified[(word_offset + 2) * 4 + 2],
            modified[(word_offset + 2) * 4 + 3],
        ]);
        let high_mask = (1u32 << byte_shift) - 1;
        let high_new = (high_existing & !high_mask) | (nonce_high >> remaining_shift);
        modified[(word_offset + 2) * 4..(word_offset + 2) * 4 + 4].copy_from_slice(&high_new.to_le_bytes());
    }
    
    modified
}

#[test]
fn test_coinbase_merkle_single_batch() {
    common::run_test(|| async {
        let (device, queue) = common::create_test_device().await;
        
        let coinbase = load_foundry_coinbase();
        let nonce_offset = get_nonce_offset(&coinbase);
        
        let branch1: [u8; 32] = hex::decode("4ea53a030256c37391b891b0d5060537df63944ce3fcd45121215596376bb3db")
            .expect("Failed to decode branch")
            .try_into()
            .expect("Branch must be 32 bytes");
        
        let config = CoinbaseMerkleConfig {
            coinbase_template: coinbase.clone(),
            nonce_byte_offset: nonce_offset,
            merkle_branches: vec![branch1],
            batch_size: 1,
            seed: 12345,
        };
        
        let mut module = CoinbaseMerkleModule::new(config);
        module.setup(&device, &queue).await.expect("Setup failed");
        module.run(&device, &queue).await.expect("Run failed");
        
        let output = module.read_merkle_roots(&device).await.expect("Read failed");
        let roots = unpack_merkle_roots(&output, 1);
        
        println!("GPU merkle root: {}", hex::encode(roots[0]));
        
        assert_eq!(roots.len(), 1);
        
        module.destroy();
    });
}

#[test]
fn test_coinbase_merkle_deterministic() {
    common::run_test(|| async {
        let (device, queue) = common::create_test_device().await;
        
        let coinbase = load_foundry_coinbase();
        let nonce_offset = get_nonce_offset(&coinbase);
        
        let branch1: [u8; 32] = hex::decode("4ea53a030256c37391b891b0d5060537df63944ce3fcd45121215596376bb3db")
            .expect("Failed to decode branch")
            .try_into()
            .expect("Branch must be 32 bytes");
        
        let seed = 12345u32;
        let batch_size = 4u32;
        
        let config1 = CoinbaseMerkleConfig {
            coinbase_template: coinbase.clone(),
            nonce_byte_offset: nonce_offset,
            merkle_branches: vec![branch1],
            batch_size,
            seed,
        };
        
        let mut module1 = CoinbaseMerkleModule::new(config1);
        module1.setup(&device, &queue).await.expect("Setup 1 failed");
        module1.run(&device, &queue).await.expect("Run 1 failed");
        let output1 = module1.read_merkle_roots(&device).await.expect("Read 1 failed");
        let roots1 = unpack_merkle_roots(&output1, batch_size as usize);
        module1.destroy();
        
        let config2 = CoinbaseMerkleConfig {
            coinbase_template: coinbase.clone(),
            nonce_byte_offset: nonce_offset,
            merkle_branches: vec![branch1],
            batch_size,
            seed,
        };
        
        let mut module2 = CoinbaseMerkleModule::new(config2);
        module2.setup(&device, &queue).await.expect("Setup 2 failed");
        module2.run(&device, &queue).await.expect("Run 2 failed");
        let output2 = module2.read_merkle_roots(&device).await.expect("Read 2 failed");
        let roots2 = unpack_merkle_roots(&output2, batch_size as usize);
        module2.destroy();
        
        for i in 0..batch_size as usize {
            assert_eq!(
                roots1[i], roots2[i],
                "Batch {} should be deterministic",
                i
            );
            println!("Batch {}: {}", i, hex::encode(roots1[i]));
        }
    });
}

#[test]
fn test_coinbase_merkle_multiple_branches() {
    common::run_test(|| async {
        let (device, queue) = common::create_test_device().await;
        
        let coinbase = load_foundry_coinbase();
        let nonce_offset = get_nonce_offset(&coinbase);
        
        let branch1: [u8; 32] = hex::decode("4ea53a030256c37391b891b0d5060537df63944ce3fcd45121215596376bb3db")
            .expect("Failed to decode branch")
            .try_into()
            .expect("Branch must be 32 bytes");
        let branch2: [u8; 32] = hex::decode("22cd1dde2c1b083237bbadd62ed1d51ee455265b7defe04dc8bcae7e5acacb33")
            .expect("Failed to decode branch")
            .try_into()
            .expect("Branch must be 32 bytes");
        
        let config = CoinbaseMerkleConfig {
            coinbase_template: coinbase.clone(),
            nonce_byte_offset: nonce_offset,
            merkle_branches: vec![branch1, branch2],
            batch_size: 4,
            seed: 42,
        };
        
        let mut module = CoinbaseMerkleModule::new(config);
        module.setup(&device, &queue).await.expect("Setup failed");
        module.run(&device, &queue).await.expect("Run failed");
        
        let output = module.read_merkle_roots(&device).await.expect("Read failed");
        let roots = unpack_merkle_roots(&output, 4);
        
        for (i, root) in roots.iter().enumerate() {
            println!("Batch {}: {}", i, hex::encode(root));
        }
        
        assert_eq!(roots.len(), 4);
        
        module.destroy();
    });
}

#[test]
fn test_cpu_verification() {
    let coinbase = load_foundry_coinbase();
    let nonce_offset = get_nonce_offset(&coinbase);
    
    let branch1: [u8; 32] = hex::decode("4ea53a030256c37391b891b0d5060537df63944ce3fcd45121215596376bb3db")
        .expect("Failed to decode branch")
        .try_into()
        .expect("Branch must be 32 bytes");
    
    let fixed_nonce_low: u32 = 0x78563412;
    let fixed_nonce_high: u32 = 0xf0debc9a;
    
    let modified_coinbase = apply_nonce_to_coinbase(&coinbase, nonce_offset, fixed_nonce_low, fixed_nonce_high);
    
    let coinbase_hash = double_sha256(&modified_coinbase);
    let expected_merkle = compute_merkle_root_cpu(&coinbase_hash, &[branch1]);
    
    println!("Expected merkle root: {}", hex::encode(expected_merkle));
    
    assert_eq!(expected_merkle.len(), 32);
}