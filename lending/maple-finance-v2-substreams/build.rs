use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("ContractFactory", "abi/ContractFactory.json")?
        .generate()?
        .write_to_file("src/abi/contract_factory.rs")?;
    Abigen::new("MigrationHelper", "abi/MigrationHelper.json")?
        .generate()?
        .write_to_file("src/abi/migration_helper.rs")?;
    Ok(())
}
