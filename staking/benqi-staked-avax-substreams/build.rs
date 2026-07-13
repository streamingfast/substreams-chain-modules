use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("SAVAX", "abi/SAVAX.json")?
        .generate()?
        .write_to_file("src/abi/savax.rs")?;
    Abigen::new("SAVAXOldImplementation", "abi/SAVAXOldImplementation.json")?
        .generate()?
        .write_to_file("src/abi/savax_old_implementation.rs")?;
    Ok(())
}
