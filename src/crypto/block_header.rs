use super::double_sha256;

#[derive(Clone, Debug)]
pub struct BlockHeader {
    pub version: u32,
    pub prev_block_hash: [u8; 32],
    pub merkle_root: [u8; 32],
    pub timestamp: u32,
    pub bits: u32,
    pub nonce: u32,
}

impl BlockHeader {
    pub const SIZE: usize = 80;

    pub fn to_bytes(&self) -> [u8; 80] {
        let mut bytes = [0u8; 80];

        bytes[0..4].copy_from_slice(&self.version.to_le_bytes());
        bytes[4..36].copy_from_slice(&self.prev_block_hash);
        bytes[36..68].copy_from_slice(&self.merkle_root);
        bytes[68..72].copy_from_slice(&self.timestamp.to_le_bytes());
        bytes[72..76].copy_from_slice(&self.bits.to_le_bytes());
        bytes[76..80].copy_from_slice(&self.nonce.to_le_bytes());

        bytes
    }

    pub fn from_bytes(bytes: &[u8; 80]) -> Self {
        let version = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let mut prev_block_hash = [0u8; 32];
        prev_block_hash.copy_from_slice(&bytes[4..36]);
        let mut merkle_root = [0u8; 32];
        merkle_root.copy_from_slice(&bytes[36..68]);
        let timestamp = u32::from_le_bytes([bytes[68], bytes[69], bytes[70], bytes[71]]);
        let bits = u32::from_le_bytes([bytes[72], bytes[73], bytes[74], bytes[75]]);
        let nonce = u32::from_le_bytes([bytes[76], bytes[77], bytes[78], bytes[79]]);

        Self {
            version,
            prev_block_hash,
            merkle_root,
            timestamp,
            bits,
            nonce,
        }
    }

    pub fn hash(&self) -> [u8; 32] {
        double_sha256(&self.to_bytes())
    }

    pub fn with_nonce(&self, nonce: u32) -> Self {
        Self {
            version: self.version,
            prev_block_hash: self.prev_block_hash,
            merkle_root: self.merkle_root,
            timestamp: self.timestamp,
            bits: self.bits,
            nonce,
        }
    }

    pub fn with_merkle_root(&self, merkle_root: [u8; 32]) -> Self {
        Self {
            version: self.version,
            prev_block_hash: self.prev_block_hash,
            merkle_root,
            timestamp: self.timestamp,
            bits: self.bits,
            nonce: self.nonce,
        }
    }
}

pub fn check_hash_meets_target(hash: &[u8; 32], target: &[u8; 32]) -> bool {
    for i in (0..32).rev() {
        if hash[i] < target[i] {
            return true;
        }
        if hash[i] > target[i] {
            return false;
        }
    }
    true
}

pub fn bits_to_target(bits: u32) -> [u8; 32] {
    let exponent = (bits >> 24) as usize;
    let mantissa = bits & 0x00ffffff;

    let mut target = [0u8; 32];

    if exponent <= 3 {
        let shift = (3 - exponent) as usize;
        target[shift] = mantissa as u8;
        target[shift + 1] = (mantissa >> 8) as u8;
        target[shift + 2] = (mantissa >> 16) as u8;
    } else {
        let pos = exponent - 3;
        target[pos] = mantissa as u8;
        target[pos + 1] = (mantissa >> 8) as u8;
        target[pos + 2] = (mantissa >> 16) as u8;
    }

    target
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex_to_bytes_80(hex: &str) -> [u8; 80] {
        let mut arr = [0u8; 80];
        for i in 0..80 {
            arr[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap();
        }
        arr
    }

    #[test]
    fn test_block_0() {
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
    fn test_check_hash_meets_target() {
        let mut target = [0u8; 32];
        target[31] = 0xff;

        let mut valid_hash = [0u8; 32];
        valid_hash[31] = 0xfe;

        let mut invalid_hash = [0u8; 32];
        invalid_hash[31] = 0xff;
        invalid_hash[30] = 0x01;

        assert!(check_hash_meets_target(&valid_hash, &target));
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
}
