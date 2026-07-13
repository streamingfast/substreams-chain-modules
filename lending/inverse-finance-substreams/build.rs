use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("Factory", "abi/Factory.json")?
        .generate()?
        .write_to_file("src/abi/factory.rs")?;
    Abigen::new("CToken", "abi/CToken.json")?
        .generate()?
        .write_to_file("src/abi/c_token.rs")?;
    Abigen::new("INV", "abi/INV.json")?
        .generate()?
        .write_to_file("src/abi/inv.rs")?;
    Abigen::new("DOLA", "abi/DOLA.json")?
        .generate()?
        .write_to_file("src/abi/dola.rs")?;
    Abigen::new("Stablizer", "abi/Stablizer.json")?
        .generate()?
        .write_to_file("src/abi/stablizer.rs")?;
    Ok(())
}
