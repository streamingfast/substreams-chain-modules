use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("LRTVault", "abi/LRTVault.json")?
        .generate()?
        .write_to_file("src/abi/lrt_vault.rs")?;
    Ok(())
}
