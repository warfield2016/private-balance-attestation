// Minimal LEZ sequencer RPC client. Plain HTTP over tokio; only hits
// the get_proof_for_commitment endpoint. A larger client lives in the
// SDK once needed.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use attestation_core::Hash32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SequencerProofResponse {
    pub leaf: String,
    pub merkle_root: String,
    pub merkle_siblings: Vec<String>,
    pub merkle_indices: Vec<bool>,
    pub anchor_slot: u64,
}

pub async fn get_proof_for_commitment(
    sequencer_url: &str,
    commitment: &Hash32,
) -> Result<SequencerProofResponse> {
    let url = format!(
        "{}/v1/account/by_commitment?c={}",
        sequencer_url.trim_end_matches('/'),
        hex::encode(commitment)
    );
    let body = http_get(&url, Duration::from_secs(15)).await?;
    serde_json::from_slice(&body).context("decode sequencer response")
}

async fn http_get(url: &str, timeout: Duration) -> Result<Vec<u8>> {
    let stripped = url
        .strip_prefix("http://")
        .ok_or_else(|| anyhow!("only http:// URLs supported (got {url})"))?;
    let (host_port, path) = match stripped.find('/') {
        Some(i) => (&stripped[..i], &stripped[i..]),
        None => (stripped, "/"),
    };
    let (host, port) = match host_port.find(':') {
        Some(i) => (&host_port[..i], host_port[i + 1..].parse::<u16>()?),
        None => (host_port, 80),
    };

    let mut stream = tokio::time::timeout(timeout, TcpStream::connect((host, port)))
        .await
        .context("connect timeout")?
        .context("connect")?;

    let req = format!(
        "GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\nUser-Agent: attestation-cli/0.1\r\n\r\n"
    );
    stream.write_all(req.as_bytes()).await.context("send")?;

    let mut buf = Vec::with_capacity(4096);
    tokio::time::timeout(timeout, stream.read_to_end(&mut buf))
        .await
        .context("read timeout")??;

    let body_start = buf
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| anyhow!("malformed HTTP response (no CRLFCRLF)"))?
        + 4;
    Ok(buf[body_start..].to_vec())
}
