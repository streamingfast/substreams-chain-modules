use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("EigenConfig", "abi/EigenConfig.json")?
        .generate()?
        .write_to_file("src/abi/eigen_config.rs")?;
    Abigen::new("EigenStaking", "abi/EigenStaking.json")?
        .generate()?
        .write_to_file("src/abi/eigen_staking.rs")?;
    Ok(())
}
