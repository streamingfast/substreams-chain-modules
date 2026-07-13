use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("PirexETH", "abi/PirexETH.json")?
        .generate()?
        .write_to_file("src/abi/pirex_eth.rs")?;
    Abigen::new("PirexFees", "abi/PirexFees.json")?
        .generate()?
        .write_to_file("src/abi/pirex_fees.rs")?;
    Ok(())
}
