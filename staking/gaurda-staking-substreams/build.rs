use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("GETH", "abi/GETH.json")?
        .generate()?
        .write_to_file("src/abi/geth.rs")?;
    Ok(())
}
