use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("BCT", "abi/BCT.json")?
        .generate()?
        .write_to_file("src/abi/bct.rs")?;
    Ok(())
}
