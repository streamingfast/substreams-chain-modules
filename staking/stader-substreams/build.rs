use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("StakingPoolManager", "abi/StakingPoolManager.json")?
        .generate()?
        .write_to_file("src/abi/staking_pool_manager.rs")?;
    Abigen::new("StaderOracle", "abi/StaderOracle.json")?
        .generate()?
        .write_to_file("src/abi/stader_oracle.rs")?;
    Abigen::new("SocializingPool", "abi/SocializingPool.json")?
        .generate()?
        .write_to_file("src/abi/socializing_pool.rs")?;
    Ok(())
}
