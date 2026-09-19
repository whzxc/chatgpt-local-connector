// Release tooling only; reuse the updater's signature library, never bundle this binary.
use base64::{engine::general_purpose::STANDARD, Engine};
use minisign_verify::{PublicKey, Signature};
use std::{env, fs};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let config: serde_json::Value =
        serde_json::from_slice(&fs::read(args.next().ok_or("missing config")?)?)?;
    let public = String::from_utf8(
        STANDARD.decode(
            config["plugins"]["updater"]["pubkey"]
                .as_str()
                .ok_or("missing public key")?,
        )?,
    )?;
    let public = PublicKey::decode(&public)?;
    let files: Vec<_> = args.collect();
    if files.is_empty() {
        return Err("no signed artifacts".into());
    }
    for file in &files {
        let signature =
            String::from_utf8(STANDARD.decode(fs::read_to_string(format!("{file}.sig"))?.trim())?)?;
        public.verify(&fs::read(file)?, &Signature::decode(&signature)?, true)?;
    }
    println!("Verified {} artifact signature(s).", files.len());
    Ok(())
}
