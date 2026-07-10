use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("CGUSD", "abi/CGUSD.json")?
        .generate()?
        .write_to_file("src/abi/cgusd.rs")?;
    Ok(())
}
