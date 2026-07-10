use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("VaultV1", "abi/VaultV1.json")?
        .generate()?
        .write_to_file("src/abi/vault_v1.rs")?;
    Ok(())
}
