use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("APETH", "abi/APETH.json")?
        .generate()?
        .write_to_file("src/abi/apeth.rs")?;
    Abigen::new("APETHEarlyDeposits", "abi/APETHEarlyDeposits.json")?
        .generate()?
        .write_to_file("src/abi/apeth_early_deposits.rs")?;
    Ok(())
}
