use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("GGAVAX", "abi/GGAVAX.json")?
        .generate()?
        .write_to_file("src/abi/ggavax.rs")?;
    Ok(())
}
