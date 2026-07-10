use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("STLINK", "abi/STLINK.json")?
        .generate()?
        .write_to_file("src/abi/stlink.rs")?;
    Abigen::new("PriorityPool", "abi/PriorityPool.json")?
        .generate()?
        .write_to_file("src/abi/priority_pool.rs")?;
    Ok(())
}
