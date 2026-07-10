use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("DepositManager", "abi/DepositManager.json")?
        .generate()?
        .write_to_file("src/abi/deposit_manager.rs")?;
    Abigen::new("SWEXIT", "abi/SWEXIT.json")?
        .generate()?
        .write_to_file("src/abi/swexit.rs")?;
    Abigen::new("SWETH", "abi/SWETH.json")?
        .generate()?
        .write_to_file("src/abi/sweth.rs")?;
    Ok(())
}
