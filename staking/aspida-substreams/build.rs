use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("AETH", "abi/AETH.json")?
        .generate()?
        .write_to_file("src/abi/aeth.rs")?;
    Ok(())
}
