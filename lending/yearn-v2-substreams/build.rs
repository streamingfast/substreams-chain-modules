use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("RegistryV1", "abi/Registry_v1.json")?
        .generate()?
        .write_to_file("src/abi/registry_v1.rs")?;
    Abigen::new("RegistryV2", "abi/Registry_v2.json")?
        .generate()?
        .write_to_file("src/abi/registry_v2.rs")?;
    Ok(())
}
