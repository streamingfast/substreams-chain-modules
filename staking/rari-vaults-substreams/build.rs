use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("RariUSDCFundManager", "abi/RariUSDCFundManager.json")?
        .generate()?
        .write_to_file("src/abi/rari_usdc_fund_manager.rs")?;
    Abigen::new("RariYieldFundManager", "abi/RariYieldFundManager.json")?
        .generate()?
        .write_to_file("src/abi/rari_yield_fund_manager.rs")?;
    Abigen::new("RariDAIFundManager", "abi/RariDAIFundManager.json")?
        .generate()?
        .write_to_file("src/abi/rari_dai_fund_manager.rs")?;
    Abigen::new("RariEtherFundManager", "abi/RariEtherFundManager.json")?
        .generate()?
        .write_to_file("src/abi/rari_ether_fund_manager.rs")?;
    Ok(())
}
