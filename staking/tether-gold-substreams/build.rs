use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("XAUT", "abi/XAUT.json")?
        .generate()?
        .write_to_file("src/abi/xaut.rs")?;
    Ok(())
}
