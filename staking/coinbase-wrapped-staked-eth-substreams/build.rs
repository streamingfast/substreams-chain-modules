use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("CBETH", "abi/CBETH.json")?
        .generate()?
        .write_to_file("src/abi/cbeth.rs")?;
    Ok(())
}
