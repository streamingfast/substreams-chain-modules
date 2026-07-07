use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("LRTDepositPool", "abi/LRTDepositPool.json")?
        .generate()?
        .write_to_file("src/abi/lrt_deposit_pool.rs")?;
    Abigen::new("LRTWithdrawalManager", "abi/LRTWithdrawalManager.json")?
        .generate()?
        .write_to_file("src/abi/lrt_withdrawal_manager.rs")?;
    Ok(())
}
