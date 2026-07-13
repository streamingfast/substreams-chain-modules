use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("Shares", "abi/Shares.json")?
        .generate()?
        .write_to_file("src/abi/shares.rs")?;
    Ok(())
}
