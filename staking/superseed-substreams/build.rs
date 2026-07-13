use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("SuperSaleDeposit", "abi/SuperSaleDeposit.json")?
        .generate()?
        .write_to_file("src/abi/super_sale_deposit.rs")?;
    Ok(())
}
