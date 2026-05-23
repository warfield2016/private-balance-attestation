// Merkle path verification. LEZ hashes children with SHA256(left || right).
// `indices[i] == true` means the current node at level i is the RIGHT
// child, so we hash (sibling, current) at that level.

use crate::{sha256_concat, CoreError, Hash32};
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MerklePath {
    pub siblings: Vec<Hash32>,
    pub indices: Vec<bool>,
}

pub fn verify_merkle_path(
    leaf: &Hash32,
    path: &MerklePath,
    expected_root: &Hash32,
) -> Result<(), CoreError> {
    if path.siblings.len() != path.indices.len() {
        return Err(CoreError::MerklePathShapeMismatch);
    }
    if path.siblings.is_empty() {
        return Err(CoreError::EmptyMerklePath);
    }

    let mut node: Hash32 = *leaf;
    for (sibling, is_right) in path.siblings.iter().zip(path.indices.iter()) {
        node = if *is_right {
            sha256_concat(&[sibling, &node])
        } else {
            sha256_concat(&[&node, sibling])
        };
    }

    if node == *expected_root {
        Ok(())
    } else {
        Err(CoreError::RootMismatch)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair_hash(l: &Hash32, r: &Hash32) -> Hash32 {
        sha256_concat(&[l, r])
    }

    #[test]
    fn three_level_tree_round_trip() {
        let leaves: Vec<Hash32> = (0u8..8)
            .map(|i| {
                let mut buf = [0u8; 32];
                buf[0] = i;
                buf
            })
            .collect();

        let l1: Vec<Hash32> = (0..4)
            .map(|i| pair_hash(&leaves[2 * i], &leaves[2 * i + 1]))
            .collect();
        let l2: Vec<Hash32> = (0..2)
            .map(|i| pair_hash(&l1[2 * i], &l1[2 * i + 1]))
            .collect();
        let root = pair_hash(&l2[0], &l2[1]);

        // Path for leaf index 5 = 0b101: right, left, right.
        let leaf_idx = 5usize;
        let siblings = vec![leaves[4], l1[3], l2[0]];
        let indices = vec![
            leaf_idx & 1 == 1,
            (leaf_idx >> 1) & 1 == 1,
            (leaf_idx >> 2) & 1 == 1,
        ];
        assert_eq!(indices, vec![true, false, true]);

        let path = MerklePath { siblings, indices };
        assert!(verify_merkle_path(&leaves[5], &path, &root).is_ok());
    }

    #[test]
    fn mismatched_lengths_reject() {
        let path = MerklePath {
            siblings: vec![[0u8; 32]; 3],
            indices: vec![true, false],
        };
        let leaf = [7u8; 32];
        let root = [1u8; 32];
        assert_eq!(
            verify_merkle_path(&leaf, &path, &root),
            Err(CoreError::MerklePathShapeMismatch)
        );
    }

    #[test]
    fn wrong_root_rejected() {
        let leaf = [9u8; 32];
        let sibling = [3u8; 32];
        let real_root = sha256_concat(&[&leaf, &sibling]);
        let path = MerklePath {
            siblings: vec![sibling],
            indices: vec![false],
        };
        assert!(verify_merkle_path(&leaf, &path, &real_root).is_ok());
        let wrong_root = [42u8; 32];
        assert_eq!(
            verify_merkle_path(&leaf, &path, &wrong_root),
            Err(CoreError::RootMismatch)
        );
    }
}
