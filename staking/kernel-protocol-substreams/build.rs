use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("KRETH", "abi/KRETH.json")?
        .generate()?
        .write_to_file("src/abi/kreth.rs")?;
    Abigen::new("KSETH", "abi/KSETH.json")?
        .generate()?
        .write_to_file("src/abi/kseth.rs")?;
    Abigen::new("KUSD", "abi/KUSD.json")?
        .generate()?
        .write_to_file("src/abi/kusd.rs")?;
    Ok(())
}
