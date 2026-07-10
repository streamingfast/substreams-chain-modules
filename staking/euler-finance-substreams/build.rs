use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("euler", "abi/euler.json")?
        .generate()?
        .write_to_file("src/abi/euler.rs")?;
    Abigen::new("EulStakes", "abi/EulStakes.json")?
        .generate()?
        .write_to_file("src/abi/eul_stakes.rs")?;
    Ok(())
}
