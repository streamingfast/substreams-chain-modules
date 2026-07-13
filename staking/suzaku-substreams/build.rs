use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("CollateralFactory", "abi/CollateralFactory.json")?
        .generate()?
        .write_to_file("src/abi/collateral_factory.rs")?;
    Abigen::new("Collateral", "abi/Collateral.json")?
        .generate()?
        .write_to_file("src/abi/collateral.rs")?;
    Ok(())
}
