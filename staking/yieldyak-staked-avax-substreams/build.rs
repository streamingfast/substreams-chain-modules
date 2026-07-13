use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("GAVAX", "abi/GAVAX.json")?
        .generate()?
        .write_to_file("src/abi/gavax.rs")?;
    Ok(())
}
