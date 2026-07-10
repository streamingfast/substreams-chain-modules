use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("StakedAvail", "abi/StakedAvail.json")?
        .generate()?
        .write_to_file("src/abi/staked_avail.rs")?;
    Ok(())
}
