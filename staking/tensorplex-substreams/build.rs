use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("PLXTAO", "abi/PLXTAO.json")?
        .generate()?
        .write_to_file("src/abi/plxtao.rs")?;
    Ok(())
}
