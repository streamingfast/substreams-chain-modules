use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("LiquidStaking", "abi/LiquidStaking.json")?
        .generate()?
        .write_to_file("src/abi/liquid_staking.rs")?;
    Abigen::new("WithdrawalRequest", "abi/WithdrawalRequest.json")?
        .generate()?
        .write_to_file("src/abi/withdrawal_request.rs")?;
    Abigen::new("NethPool", "abi/NethPool.json")?
        .generate()?
        .write_to_file("src/abi/neth_pool.rs")?;
    Abigen::new("RestakingPool", "abi/RestakingPool.json")?
        .generate()?
        .write_to_file("src/abi/restaking_pool.rs")?;
    Ok(())
}
