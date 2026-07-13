use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("Hourglass", "abi/Hourglass.json")?
        .generate()?
        .write_to_file("src/abi/hourglass.rs")?;
    Ok(())
}
