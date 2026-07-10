use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("Cream", "abi/Cream.json")?
        .generate()?
        .write_to_file("src/abi/cream.rs")?;
    Abigen::new("CRETH", "abi/CRETH.json")?
        .generate()?
        .write_to_file("src/abi/creth.rs")?;
    Ok(())
}
