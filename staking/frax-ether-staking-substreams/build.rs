use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("SFRXETH", "abi/SFRXETH.json")?
        .generate()?
        .write_to_file("src/abi/sfrxeth.rs")?;
    Ok(())
}
