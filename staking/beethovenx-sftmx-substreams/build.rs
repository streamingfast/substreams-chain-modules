use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("FTMStaking", "abi/FTMStaking.json")?
        .generate()?
        .write_to_file("src/abi/ftm_staking.rs")?;
    Ok(())
}
