use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("PAXG", "abi/PAXG.json")?
        .generate()?
        .write_to_file("src/abi/paxg.rs")?;
    Ok(())
}
