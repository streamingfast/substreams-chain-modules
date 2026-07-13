use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("MEVETH", "abi/MEVETH.json")?
        .generate()?
        .write_to_file("src/abi/meveth.rs")?;
    Ok(())
}
