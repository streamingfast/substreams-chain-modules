use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("RocketTokenRETH", "abi/RocketTokenRETH.json")?
        .generate()?
        .write_to_file("src/abi/rocket_token_reth.rs")?;
    Ok(())
}
