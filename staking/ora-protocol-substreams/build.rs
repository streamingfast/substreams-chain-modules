use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("OraStakeRouter", "abi/OraStakeRouter.json")?
        .generate()?
        .write_to_file("src/abi/ora_stake_router.rs")?;
    Ok(())
}
