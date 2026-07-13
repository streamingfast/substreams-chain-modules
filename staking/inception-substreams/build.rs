use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("INETH", "abi/INETH.json")?
        .generate()?
        .write_to_file("src/abi/ineth.rs")?;
    Ok(())
}
