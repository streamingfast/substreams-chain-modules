use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("BancorNetwork", "abi/BancorNetwork.json")?
        .generate()?
        .write_to_file("src/abi/bancor_network.rs")?;
    Abigen::new("PoolCollection", "abi/PoolCollection.json")?
        .generate()?
        .write_to_file("src/abi/pool_collection.rs")?;
    Abigen::new("PoolTokenFactory", "abi/PoolTokenFactory.json")?
        .generate()?
        .write_to_file("src/abi/pool_token_factory.rs")?;
    Abigen::new("NetworkSettings", "abi/NetworkSettings.json")?
        .generate()?
        .write_to_file("src/abi/network_settings.rs")?;
    Abigen::new("StandardRewards", "abi/StandardRewards.json")?
        .generate()?
        .write_to_file("src/abi/standard_rewards.rs")?;
    Abigen::new("BNTPool", "abi/BNTPool.json")?
        .generate()?
        .write_to_file("src/abi/bnt_pool.rs")?;
    Ok(())
}
