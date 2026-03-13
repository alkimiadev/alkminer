use alkminer::crypto::{double_sha256, Sha256, Xoshiro128Plus};

#[test]
fn test_sha256_empty() {
    let mut ctx = Sha256::new();
    ctx.update(b"");
    let hash = ctx.finalize();
    let expected_hex = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    let computed_hex = hex::encode(hash);
    assert_eq!(computed_hex, expected_hex);
}

#[test]
fn test_sha256_abc() {
    let mut ctx = Sha256::new();
    ctx.update(b"abc");
    let hash = ctx.finalize();
    let expected_hex = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    let computed_hex = hex::encode(hash);
    assert_eq!(computed_hex, expected_hex);
}

#[test]
fn test_double_sha256() {
    let hash = double_sha256(b"hello");
    assert_eq!(hash.len(), 32);
}

#[test]
fn test_rng_reproducible() {
    let mut rng1 = Xoshiro128Plus::new(42);
    let mut rng2 = Xoshiro128Plus::new(42);

    for _ in 0..10 {
        assert_eq!(rng1.next_u32(), rng2.next_u32());
    }
}

#[test]
fn test_rng_f32_range() {
    let mut rng = Xoshiro128Plus::new(12345);
    for _ in 0..100 {
        let f = rng.next_f32();
        assert!(f >= 0.0 && f < 1.0);
    }
}
