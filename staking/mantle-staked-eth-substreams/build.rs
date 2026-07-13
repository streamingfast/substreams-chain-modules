use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("Staking", "abi/Staking.json")?
        .generate()?
        .write_to_file("src/abi/staking.rs")?;
    Abigen::new("ReturnsAggregator", "abi/ReturnsAggregator.json")?
        .generate()?
        .write_to_file("src/abi/returns_aggregator.rs")?;
    Ok(())
}
