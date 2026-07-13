use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("STBT", "abi/STBT.json")?
        .generate()?
        .write_to_file("src/abi/stbt.rs")?;
    Ok(())
}
