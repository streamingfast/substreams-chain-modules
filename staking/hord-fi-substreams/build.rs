use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("HETH", "abi/HETH.json")?
        .generate()?
        .write_to_file("src/abi/heth.rs")?;
    Ok(())
}
