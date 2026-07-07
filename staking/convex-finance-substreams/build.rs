use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("Booster", "abi/Booster.json")?
        .generate()?
        .write_to_file("src/abi/booster.rs")?;
    Ok(())
}
