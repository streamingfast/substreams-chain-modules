use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("TroveManager", "abi/TroveManager.json")?
        .generate()?
        .write_to_file("src/abi/trove_manager.rs")?;
    Abigen::new("BorrowerOperations", "abi/BorrowerOperations.json")?
        .generate()?
        .write_to_file("src/abi/borrower_operations.rs")?;
    Abigen::new("PriceFeed", "abi/PriceFeed.json")?
        .generate()?
        .write_to_file("src/abi/price_feed.rs")?;
    Abigen::new("ActivePool", "abi/ActivePool.json")?
        .generate()?
        .write_to_file("src/abi/active_pool.rs")?;
    Abigen::new("CollSurplusPool", "abi/CollSurplusPool.json")?
        .generate()?
        .write_to_file("src/abi/coll_surplus_pool.rs")?;
    Abigen::new("StabilityPool", "abi/StabilityPool.json")?
        .generate()?
        .write_to_file("src/abi/stability_pool.rs")?;
    Ok(())
}
