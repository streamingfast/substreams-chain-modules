use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("Vault", "abi/Vault.json")?
        .generate()?
        .write_to_file("src/abi/vault.rs")?;
    Ok(())
}
