// attestation-cli — drive the prover, the verifier, and the LEZ
// sequencer from a terminal.

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};

use attestation_core::{
    context::{
        context_id_for_chat, context_id_for_fee_tier, context_id_for_program, context_id_generic,
    },
    journal::{JournalFields, CIRCUIT_VERSION},
    Hash32,
};
use attestation_host::{prove, sign_challenge, verifying_key_bytes, Witness};
use attestation_verifier::{peek_journal, verify_attestation, VerifyArgs};

mod io;
mod sequencer;

#[derive(Parser)]
#[command(
    name = "attestation-cli",
    version,
    about = "LP-0005 balance attestation CLI"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,

    #[arg(long, global = true)]
    json: bool,

    #[arg(long, global = true)]
    quiet: bool,
}

#[derive(Subcommand)]
enum Cmd {
    Prove(ProveArgs),
    Verify(VerifyCmdArgs),
    Submit(SubmitArgs),
    Fetch(FetchArgs),
    Inspect(InspectArgs),
    Sign(SignArgs),
    Pubkey(PubkeyArgs),
    #[command(name = "context-id")]
    ContextId(ContextIdArgs),
    Benchmark(BenchmarkArgs),
}

#[derive(Parser)]
struct ProveArgs {
    #[arg(long)]
    threshold: u64,
    #[arg(long = "context-id")]
    context_id: String,
    #[arg(long = "presenter-pk")]
    presenter_pk: String,
    #[arg(long = "program-owner")]
    program_owner: String,
    #[arg(long)]
    witness: PathBuf,
    #[arg(long)]
    out: PathBuf,
    #[arg(long, env = "LEZ_SEQUENCER_URL")]
    sequencer_url: Option<String>,
}

#[derive(Parser)]
struct VerifyCmdArgs {
    #[arg(long)]
    proof: PathBuf,
    #[arg(long = "context-id")]
    context_id: String,
    #[arg(long = "program-owner")]
    program_owner: String,
    #[arg(long)]
    challenge: PathBuf,
    #[arg(long)]
    signature: PathBuf,
    #[arg(long = "trusted-roots")]
    trusted_roots: PathBuf,
    #[arg(long = "min-threshold", default_value_t = 0)]
    min_threshold: u64,
}

#[derive(Parser)]
struct SubmitArgs {
    #[arg(long)]
    program: String,
    #[arg(long)]
    proof: PathBuf,
    #[arg(long = "action-tag")]
    action_tag: String,
    #[arg(long, env = "LEZ_SEQUENCER_URL")]
    sequencer_url: Option<String>,
}

#[derive(Parser)]
struct FetchArgs {
    #[arg(long)]
    commitment: String,
    #[arg(long, env = "LEZ_SEQUENCER_URL")]
    sequencer_url: String,
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Parser)]
struct InspectArgs {
    #[arg(long)]
    proof: PathBuf,
}

#[derive(Parser)]
struct SignArgs {
    #[arg(long)]
    keypath: PathBuf,
    #[arg(long)]
    message: PathBuf,
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Parser)]
struct PubkeyArgs {
    #[arg(long)]
    keypath: PathBuf,
}

#[derive(Parser)]
struct ContextIdArgs {
    #[arg()]
    domain: String,
    #[arg(num_args = 1..)]
    args: Vec<String>,
}

#[derive(Parser)]
struct BenchmarkArgs {
    #[arg(long, default_value_t = 5)]
    runs: u32,
    #[arg(long, default_value_t = 100)]
    threshold: u64,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let result = match cli.cmd {
        Cmd::Prove(a) => cmd_prove(a, cli.json, cli.quiet).await,
        Cmd::Verify(a) => cmd_verify(a, cli.json),
        Cmd::Submit(a) => cmd_submit(a, cli.json).await,
        Cmd::Fetch(a) => cmd_fetch(a, cli.json).await,
        Cmd::Inspect(a) => cmd_inspect(a, cli.json),
        Cmd::Sign(a) => cmd_sign(a),
        Cmd::Pubkey(a) => cmd_pubkey(a, cli.json),
        Cmd::ContextId(a) => cmd_context_id(a),
        Cmd::Benchmark(a) => cmd_benchmark(a, cli.json).await,
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::from(1)
        }
    }
}

fn parse_hash32(s: &str) -> Result<Hash32> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(s).context("invalid hex in 32-byte argument")?;
    if bytes.len() != 32 {
        return Err(anyhow!(
            "expected 32 bytes (64 hex chars), got {}",
            bytes.len()
        ));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}

fn load_signing_key(path: &PathBuf) -> Result<SigningKey> {
    let bytes = fs::read(path).with_context(|| format!("read keypath {}", path.display()))?;
    if bytes.len() != 32 {
        return Err(anyhow!(
            "spending key file must hold 32 raw bytes (seed), got {} bytes",
            bytes.len()
        ));
    }
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&bytes);
    Ok(SigningKey::from_bytes(&seed))
}

async fn cmd_prove(args: ProveArgs, json: bool, quiet: bool) -> Result<()> {
    let _ = args.sequencer_url;
    let context_id = parse_hash32(&args.context_id)?;
    let presenter_pk = parse_hash32(&args.presenter_pk)?;
    let program_owner = parse_hash32(&args.program_owner)?;

    let raw: io::WitnessFile = serde_json::from_str(
        &fs::read_to_string(&args.witness)
            .with_context(|| format!("read witness {}", args.witness.display()))?,
    )?;
    let witness = raw.into_witness()?;

    let public = JournalFields {
        merkle_root: raw.merkle_root_bytes()?,
        threshold: args.threshold,
        context_id,
        presenter_pk,
        program_owner,
        circuit_version: CIRCUIT_VERSION,
    };

    if !quiet {
        eprintln!("Proving (this can take a minute with RISC0_DEV_MODE=0)...");
    }
    let out = prove(witness, public)?;
    fs::write(&args.out, &out.receipt).context("write proof")?;

    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "proof_path": args.out.display().to_string(),
                "prove_ms": out.prove_ms,
                "journal": out.journal,
                "receipt_bytes": out.receipt.len(),
            })
        );
    } else {
        println!(
            "OK  proof_bytes={}  prove_ms={}",
            out.receipt.len(),
            out.prove_ms
        );
        println!(
            "    presenter_pk  = 0x{}",
            hex::encode(out.journal.presenter_pk)
        );
        println!(
            "    context_id    = 0x{}",
            hex::encode(out.journal.context_id)
        );
        println!(
            "    program_owner = 0x{}",
            hex::encode(out.journal.program_owner)
        );
        println!(
            "    merkle_root   = 0x{}",
            hex::encode(out.journal.merkle_root)
        );
        println!("    threshold     = {}", out.journal.threshold);
    }
    Ok(())
}

fn cmd_verify(args: VerifyCmdArgs, json: bool) -> Result<()> {
    let proof = fs::read(&args.proof).context("read proof")?;
    let context_id = parse_hash32(&args.context_id)?;
    let program_owner = parse_hash32(&args.program_owner)?;
    let challenge = read_fixed::<32>(&args.challenge, "challenge")?;
    let signature = read_fixed::<64>(&args.signature, "signature")?;

    let roots_bytes = fs::read(&args.trusted_roots).context("read trusted roots")?;
    if roots_bytes.len() % 32 != 0 {
        return Err(anyhow!("trusted_roots length must be a multiple of 32"));
    }
    let trusted: Vec<Hash32> = roots_bytes
        .chunks_exact(32)
        .map(|c| {
            let mut a = [0u8; 32];
            a.copy_from_slice(c);
            a
        })
        .collect();

    // Image ID is pinned by the methods crate at build time and
    // re-exported by attestation_host.
    let result = verify_attestation(VerifyArgs {
        receipt_bytes: &proof,
        image_id: &attestation_host::ATTESTATION_GUEST_ID,
        expected_context_id: &context_id,
        expected_program_owner: &program_owner,
        trusted_roots: &trusted,
        minimum_threshold: args.min_threshold,
        allowed_versions: &[CIRCUIT_VERSION],
        challenge: &challenge,
        signature: &signature,
    });

    match (result, json) {
        (Ok(j), true) => {
            println!("{}", serde_json::to_string_pretty(&j)?);
            Ok(())
        }
        (Ok(j), false) => {
            println!("OK  presenter_pk=0x{}", hex::encode(j.presenter_pk));
            Ok(())
        }
        (Err(e), true) => {
            println!(
                "{}",
                serde_json::json!({ "ok": false, "error_code": e.code(), "error": format!("{e}") })
            );
            std::process::exit(2);
        }
        (Err(e), false) => Err(anyhow!("verification failed: {e} (code={})", e.code())),
    }
}

async fn cmd_submit(_args: SubmitArgs, _json: bool) -> Result<()> {
    Err(anyhow!("submit: not yet wired to a live LEZ RPC"))
}

async fn cmd_fetch(args: FetchArgs, json: bool) -> Result<()> {
    let commitment = parse_hash32(&args.commitment)?;
    let resp = sequencer::get_proof_for_commitment(&args.sequencer_url, &commitment).await?;
    if let Some(out) = args.out {
        fs::write(&out, serde_json::to_vec_pretty(&resp)?)?;
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&resp)?);
    } else {
        println!("OK  depth={}", resp.merkle_siblings.len());
    }
    Ok(())
}

fn cmd_inspect(args: InspectArgs, json: bool) -> Result<()> {
    let bytes = fs::read(&args.proof).context("read proof")?;
    let j = peek_journal(&bytes).map_err(|e| anyhow!("peek_journal: {e}"))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&j)?);
    } else {
        println!("merkle_root     = 0x{}", hex::encode(j.merkle_root));
        println!("threshold       = {}", j.threshold);
        println!("context_id      = 0x{}", hex::encode(j.context_id));
        println!("presenter_pk    = 0x{}", hex::encode(j.presenter_pk));
        println!("program_owner   = 0x{}", hex::encode(j.program_owner));
        println!("circuit_version = {}", j.circuit_version);
    }
    Ok(())
}

fn cmd_sign(args: SignArgs) -> Result<()> {
    let sk = load_signing_key(&args.keypath)?;
    let msg = read_fixed::<32>(&args.message, "message")?;
    let sig = sign_challenge(&sk, &msg);
    match args.out {
        Some(p) => fs::write(&p, sig).context("write signature")?,
        None => {
            use std::io::Write;
            std::io::stdout().write_all(&sig)?;
        }
    }
    Ok(())
}

fn cmd_pubkey(args: PubkeyArgs, json: bool) -> Result<()> {
    let sk = load_signing_key(&args.keypath)?;
    let vk = verifying_key_bytes(&sk);

    // Round-trip the key to catch corruption.
    let vk_full = VerifyingKey::from_bytes(&vk)?;
    let msg = b"lp-0005:cli:pubkey-self-check";
    let sig: Signature = sk.sign(msg);
    vk_full.verify(msg, &sig)?;

    if json {
        println!(r#"{{"pubkey":"0x{}"}}"#, hex::encode(vk));
    } else {
        println!("0x{}", hex::encode(vk));
    }
    Ok(())
}

fn cmd_context_id(args: ContextIdArgs) -> Result<()> {
    let id = match args.domain.as_str() {
        "program" => {
            if args.args.len() != 2 {
                return Err(anyhow!(
                    "usage: context-id program <program_pk> <gate_seed>"
                ));
            }
            context_id_for_program(&parse_hash32(&args.args[0])?, &parse_hash32(&args.args[1])?)
        }
        "chat" => {
            if args.args.len() != 2 {
                return Err(anyhow!("usage: context-id chat <group_pk> <epoch>"));
            }
            let epoch: u64 = args.args[1].parse().context("epoch must be u64")?;
            context_id_for_chat(&parse_hash32(&args.args[0])?, epoch)
        }
        "fee" => {
            if args.args.len() != 2 {
                return Err(anyhow!("usage: context-id fee <tier> <group_pk>"));
            }
            let tier: u32 = args.args[0].parse().context("tier must be u32")?;
            context_id_for_fee_tier(tier, &parse_hash32(&args.args[1])?)
        }
        "generic" => {
            if args.args.is_empty() {
                return Err(anyhow!(
                    "usage: context-id generic <integration_id> [extra_hex]"
                ));
            }
            let extra = if args.args.len() >= 2 {
                let s = args.args[1].strip_prefix("0x").unwrap_or(&args.args[1]);
                hex::decode(s).context("extra must be hex")?
            } else {
                Vec::new()
            };
            context_id_generic(&args.args[0], &extra)
        }
        other => return Err(anyhow!("unknown domain: {other}")),
    };
    println!("0x{}", hex::encode(id));
    Ok(())
}

async fn cmd_benchmark(args: BenchmarkArgs, json: bool) -> Result<()> {
    let _ = (args.runs, args.threshold);
    if json {
        println!(r#"{{"runs":[],"note":"benchmark not yet wired"}}"#);
    } else {
        println!("benchmark not yet wired");
    }
    Ok(())
}

fn read_fixed<const N: usize>(path: &PathBuf, label: &str) -> Result<[u8; N]> {
    let bytes = fs::read(path).with_context(|| format!("read {label}"))?;
    if bytes.len() != N {
        return Err(anyhow!(
            "{label} must be exactly {N} bytes (got {})",
            bytes.len()
        ));
    }
    let mut out = [0u8; N];
    out.copy_from_slice(&bytes);
    Ok(out)
}
