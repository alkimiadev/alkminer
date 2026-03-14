use super::double_sha256;

pub fn compute_merkle_root(coinbase_hash: &[u8; 32], branches: &[[u8; 32]]) -> [u8; 32] {
    let mut current = *coinbase_hash;

    for branch in branches {
        let mut concat = [0u8; 64];
        concat[..32].copy_from_slice(&current);
        concat[32..].copy_from_slice(branch);
        current = double_sha256(&concat);
    }

    current
}

pub fn compute_merkle_root_from_txids(txids: &[[u8; 32]]) -> [u8; 32] {
    if txids.is_empty() {
        return [0u8; 32];
    }

    if txids.len() == 1 {
        return txids[0];
    }

    let mut current_level: Vec<[u8; 32]> = txids.to_vec();

    while current_level.len() > 1 {
        if current_level.len() % 2 == 1 {
            current_level.push(*current_level.last().unwrap());
        }

        let mut next_level = Vec::with_capacity(current_level.len() / 2);
        for chunk in current_level.chunks(2) {
            let mut concat = [0u8; 64];
            concat[..32].copy_from_slice(&chunk[0]);
            concat[32..].copy_from_slice(&chunk[1]);
            next_level.push(double_sha256(&concat));
        }
        current_level = next_level;
    }

    current_level[0]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex_to_bytes(hex: &str) -> [u8; 32] {
        let mut arr = [0u8; 32];
        for i in 0..32 {
            arr[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).unwrap();
        }
        arr
    }

    #[test]
    fn test_single_branch() {
        let coinbase =
            hex_to_bytes("4ea53a030256c37391b891b0d5060537df63944ce3fcd45121215596376bb3db");
        let branch1 =
            hex_to_bytes("22cd1dde2c1b083237bbadd62ed1d51ee455265b7defe04dc8bcae7e5acacb33");

        let result = compute_merkle_root(&coinbase, &[branch1]);

        let mut concat = [0u8; 64];
        concat[..32].copy_from_slice(&coinbase);
        concat[32..].copy_from_slice(&branch1);
        let expected = double_sha256(&concat);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_two_branches() {
        let coinbase =
            hex_to_bytes("4ea53a030256c37391b891b0d5060537df63944ce3fcd45121215596376bb3db");
        let branch1 =
            hex_to_bytes("22cd1dde2c1b083237bbadd62ed1d51ee455265b7defe04dc8bcae7e5acacb33");
        let branch2 =
            hex_to_bytes("1b4f5c6a079924ba811960ce15b74403765e1a251f697efb331d7588f7d7734b");

        let result = compute_merkle_root(&coinbase, &[branch1, branch2]);

        let mut concat1 = [0u8; 64];
        concat1[..32].copy_from_slice(&coinbase);
        concat1[32..].copy_from_slice(&branch1);
        let hash1 = double_sha256(&concat1);

        let mut concat2 = [0u8; 64];
        concat2[..32].copy_from_slice(&hash1);
        concat2[32..].copy_from_slice(&branch2);
        let expected = double_sha256(&concat2);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_no_branches() {
        let coinbase =
            hex_to_bytes("4ea53a030256c37391b891b0d5060537df63944ce3fcd45121215596376bb3db");
        let result = compute_merkle_root(&coinbase, &[]);
        assert_eq!(result, coinbase);
    }

    #[test]
    fn test_full_merkle_tree_two_txs() {
        let tx1 = [1u8; 32];
        let tx2 = [2u8; 32];

        let result = compute_merkle_root_from_txids(&[tx1, tx2]);

        let mut concat = [0u8; 64];
        concat[..32].copy_from_slice(&tx1);
        concat[32..].copy_from_slice(&tx2);
        let expected = double_sha256(&concat);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_full_merkle_tree_three_txs() {
        let tx1 = [1u8; 32];
        let tx2 = [2u8; 32];
        let tx3 = [3u8; 32];

        let result = compute_merkle_root_from_txids(&[tx1, tx2, tx3]);

        let mut concat1 = [0u8; 64];
        concat1[..32].copy_from_slice(&tx1);
        concat1[32..].copy_from_slice(&tx2);
        let hash12 = double_sha256(&concat1);

        let mut concat2 = [0u8; 64];
        concat2[..32].copy_from_slice(&tx3);
        concat2[32..].copy_from_slice(&tx3);
        let hash33 = double_sha256(&concat2);

        let mut concat3 = [0u8; 64];
        concat3[..32].copy_from_slice(&hash12);
        concat3[32..].copy_from_slice(&hash33);
        let expected = double_sha256(&concat3);

        assert_eq!(result, expected);
    }

    #[test]
    fn test_full_merkle_tree_four_txs() {
        let tx1 = [1u8; 32];
        let tx2 = [2u8; 32];
        let tx3 = [3u8; 32];
        let tx4 = [4u8; 32];

        let result = compute_merkle_root_from_txids(&[tx1, tx2, tx3, tx4]);

        let mut concat12 = [0u8; 64];
        concat12[..32].copy_from_slice(&tx1);
        concat12[32..].copy_from_slice(&tx2);
        let hash12 = double_sha256(&concat12);

        let mut concat34 = [0u8; 64];
        concat34[..32].copy_from_slice(&tx3);
        concat34[32..].copy_from_slice(&tx4);
        let hash34 = double_sha256(&concat34);

        let mut concat_final = [0u8; 64];
        concat_final[..32].copy_from_slice(&hash12);
        concat_final[32..].copy_from_slice(&hash34);
        let expected = double_sha256(&concat_final);

        assert_eq!(result, expected);
    }
}
