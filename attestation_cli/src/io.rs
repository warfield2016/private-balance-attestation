// Witness JSON format used by the CLI. Kept JSON so it's inspectable
// and producible by tools other than this CLI.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use attestation_core::Hash32;
use attestation_host::Witness;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessFile {
    pub npk: String,
    pub program_owner: String,
    pub balance: u64,
    pub nonce: String,
    pub data: String, // hex
    pub merkle_root: String,
    pub merkle_siblings: Vec<String>, // hex
    pub merkle_indices: Vec<bool>,
    pub spending_pk: String,
    pub spending_pk_offset: u64,
}

impl WitnessFile {
    pub fn merkle_root_bytes(&self) -> Result<Hash32> {
        decode_hash32(&self.merkle_root, "merkle_root")
    }

    pub fn into_witness(&self) -> Result<Witness> {
        let npk = decode_hash32(&self.npk, "npk")?;
        let program_owner = decode_hash32(&self.program_owner, "program_owner")?;
        let nonce = decode_hash32(&self.nonce, "nonce")?;
        let spending_pk = decode_hash32(&self.spending_pk, "spending_pk")?;
        let data = decode_hex(&self.data, "data")?;
        let siblings: Result<Vec<Hash32>> = self
            .merkle_siblings
            .iter()
            .enumerate()
            .map(|(i, s)| decode_hash32(s, &format!("merkle_siblings[{i}]")))
            .collect();

        Ok(Witness {
            npk,
            program_owner,
            balance: self.balance,
            nonce,
            data,
            merkle_siblings: siblings?,
            merkle_indices: self.merkle_indices.clone(),
            spending_pk,
            spending_pk_offset: self.spending_pk_offset,
        })
    }
}

fn decode_hash32(s: &str, what: &str) -> Result<Hash32> {
    let bytes = decode_hex(s, what)?;
    if bytes.len() != 32 {
        return Err(anyhow!("{what}: expected 32 bytes, got {}", bytes.len()));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn decode_hex(s: &str, what: &str) -> Result<Vec<u8>> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    hex::decode(s).with_context(|| format!("{what}: invalid hex"))
}
