use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("LSETH", "abi/LSETH.json")?
        .generate()?
        .write_to_file("src/abi/lseth.rs")?;
    Abigen::new("RedeemManager", "abi/RedeemManager.json")?
        .generate()?
        .write_to_file("src/abi/redeem_manager.rs")?;
    Ok(())
}
