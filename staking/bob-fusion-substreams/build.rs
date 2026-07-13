use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("FusionLock", "abi/FusionLock.json")?
        .generate()?
        .write_to_file("src/abi/fusion_lock.rs")?;
    Ok(())
}
