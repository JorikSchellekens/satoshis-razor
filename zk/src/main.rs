//! zk-prover: Groth16 tooling for Satoshi's Razor private submissions.
//!
//! - `setup [--n <len>] [--pk-out F] [--vk-out F]`: trusted setup for the
//!   sorted-witness circuit on <len> values (default 4); prints JSON with
//!   the constraint count, key paths, and vk hash
//! - `prove --list a,b,… [--pk F]`: prove knowledge of the list; the list's
//!   length picks the circuit, which must match the proving key's
//! - `verify --vk <hex or file> --proof <hex> --public <hex>`: exit 0 iff
//!   the proof verifies
//!
//! The setup uses a fixed RNG seed so anyone can reproduce the CRS; a real
//! deployment uses an MPC ceremony. Stated plainly because it matters.
//! Key paths default to zk/keys/ under the current directory - pass
//! --pk-out/--vk-out/--pk/--vk to use it from anywhere.

mod circuit;

use ark_bls12_381::{Bls12_381, Fr};
use ark_groth16::Groth16;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_snark::SNARK;
use circuit::SortedWitnessCircuit;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

fn arg(args: &[String], flag: &str) -> Option<String> {
    args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1).cloned())
}

fn to_hex<T: CanonicalSerialize>(t: &T) -> String {
    let mut buf = Vec::new();
    t.serialize_compressed(&mut buf).expect("serialize");
    hex::encode(buf)
}

fn from_hex<T: CanonicalDeserialize>(s: &str) -> T {
    let reject = |what: &str| -> ! {
        println!("{{\"verified\":false,\"reason\":\"{what}\"}}");
        std::process::exit(1);
    };
    let Ok(bytes) = hex::decode(s.trim()) else { reject("invalid hex") };
    match T::deserialize_compressed(&bytes[..]) {
        Ok(t) => t,
        Err(_) => reject("malformed group element (not a valid curve point)"),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str).unwrap_or("") {
        "setup" => {
            // Deterministic CRS: reproducible by anyone (demo-grade trust).
            let n: usize = arg(&args, "--n").map(|v| v.parse().expect("--n")).unwrap_or(4);
            assert!((circuit::MIN_LIST..=circuit::MAX_LIST).contains(&n),
                "--n must be between {} and {}", circuit::MIN_LIST, circuit::MAX_LIST);
            let mut rng = ChaCha20Rng::seed_from_u64(0x5A7A_5A7A);
            let blank = SortedWitnessCircuit { hash: None, xs: None, n };
            let (pk, vk) = Groth16::<Bls12_381>::circuit_specific_setup(blank.clone(), &mut rng)
                .expect("setup");
            // Count constraints for the golf score.
            use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
            let cs = ConstraintSystem::<Fr>::new_ref();
            SortedWitnessCircuit { hash: Some(Fr::from(0u64)), xs: Some(vec![0; n]), n }
                .generate_constraints(cs.clone())
                .expect("constraints");
            let out_pk = arg(&args, "--pk-out").unwrap_or("zk/keys/pk.hex".into());
            let out_vk = arg(&args, "--vk-out").unwrap_or("zk/keys/vk.hex".into());
            std::fs::create_dir_all(std::path::Path::new(&out_pk).parent().unwrap()).ok();
            std::fs::write(&out_pk, to_hex(&pk)).expect("write pk");
            std::fs::write(&out_vk, to_hex(&vk)).expect("write vk");
            let vk_hash = {
                use sha2::{Digest, Sha256};
                let mut h = Sha256::new();
                h.update(to_hex(&vk).as_bytes());
                format!("{:x}", h.finalize())[..32].to_string()
            };
            println!(
                "{{\"n\":{n},\"constraints\":{},\"pk\":\"{out_pk}\",\"vk\":\"{out_vk}\",\"vk_hash\":\"{vk_hash}\"}}",
                cs.num_constraints(),
            );
        }
        "prove" => {
            let xs: Vec<u64> = arg(&args, "--list").expect("--list a,b,…")
                .split(',').map(|v| v.trim().parse().expect("u64")).collect();
            assert!((circuit::MIN_LIST..=circuit::MAX_LIST).contains(&xs.len())
                && xs.iter().all(|&v| v < 256),
                "need {}..={} values, each < 256", circuit::MIN_LIST, circuit::MAX_LIST);
            let n = xs.len();
            let pk_path = arg(&args, "--pk").unwrap_or("zk/keys/pk.hex".into());
            let pk = from_hex(&std::fs::read_to_string(&pk_path)
                .unwrap_or_else(|_| panic!("cannot read proving key {pk_path} - run setup first (--n {n})")));
            let h = circuit::commit(&xs);
            let mut rng = ChaCha20Rng::from_entropy();
            let proof = Groth16::<Bls12_381>::prove(
                &pk,
                SortedWitnessCircuit { hash: Some(h), xs: Some(xs), n },
                &mut rng,
            )
            .unwrap_or_else(|e| panic!("prove failed ({e:?}) - does the proving key's list length match? re-run setup --n {n}"));
            println!("{{\"public\":\"{}\",\"proof\":\"{}\"}}", to_hex(&h), to_hex(&proof));
        }
        "verify" => {
            let vk_arg = arg(&args, "--vk").expect("--vk <hex or path>");
            let vk_hex = if std::path::Path::new(&vk_arg).exists() {
                std::fs::read_to_string(&vk_arg).expect("read vk")
            } else {
                vk_arg
            };
            let vk = from_hex(&vk_hex);
            let proof = from_hex(&arg(&args, "--proof").expect("--proof <hex>"));
            let public: Fr = from_hex(&arg(&args, "--public").expect("--public <hex>"));
            let pvk = Groth16::<Bls12_381>::process_vk(&vk).expect("pvk");
            let ok = Groth16::<Bls12_381>::verify_with_processed_vk(&pvk, &[public], &proof)
                .expect("verify");
            println!("{{\"verified\":{ok}}}");
            std::process::exit(if ok { 0 } else { 1 });
        }
        _ => {
            eprintln!("zk-prover - Groth16 sorted-witness proofs (list lengths {}..={})", circuit::MIN_LIST, circuit::MAX_LIST);
            eprintln!("  setup  [--n <len>] [--pk-out F] [--vk-out F]   trusted setup (reproducible seed)");
            eprintln!("  prove  --list a,b,… [--pk F]                   prints {{public, proof}} hex");
            eprintln!("  verify --vk <hex|file> --proof H --public H    exit 0 iff valid");
            std::process::exit(2);
        }
    }
}
