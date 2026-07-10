use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("Minter", "abi/Minter.json")?
        .generate()?
        .write_to_file("src/abi/minter.rs")?;
    Ok(())
}
