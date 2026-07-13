use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("STCELO", "abi/STCELO.json")?
        .generate()?
        .write_to_file("src/abi/stcelo.rs")?;
    Ok(())
}
