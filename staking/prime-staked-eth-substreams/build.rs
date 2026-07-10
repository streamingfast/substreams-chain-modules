use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("LRTConfig", "abi/LRTConfig.json")?
        .generate()?
        .write_to_file("src/abi/lrt_config.rs")?;
    Abigen::new("LRTDepositPool", "abi/LRTDepositPool.json")?
        .generate()?
        .write_to_file("src/abi/lrt_deposit_pool.rs")?;
    Ok(())
}
