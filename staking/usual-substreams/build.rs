use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("USYC", "abi/USYC.json")?
        .generate()?
        .write_to_file("src/abi/usyc.rs")?;
    Ok(())
}
