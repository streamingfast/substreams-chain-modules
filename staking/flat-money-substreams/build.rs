use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("UNIT", "abi/UNIT.json")?
        .generate()?
        .write_to_file("src/abi/unit.rs")?;
    Abigen::new("LiquidationModule", "abi/LiquidationModule.json")?
        .generate()?
        .write_to_file("src/abi/liquidation_module.rs")?;
    Abigen::new("DelayedOrder", "abi/DelayedOrder.json")?
        .generate()?
        .write_to_file("src/abi/delayed_order.rs")?;
    Ok(())
}
