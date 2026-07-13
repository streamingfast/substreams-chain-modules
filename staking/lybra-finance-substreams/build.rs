use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("LybraV1", "abi/LybraV1.json")?
        .generate()?
        .write_to_file("src/abi/lybra_v1.rs")?;
    Ok(())
}
