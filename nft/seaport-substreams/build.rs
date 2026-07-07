use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("SeaportExchange", "abi/SeaportExchange.json")?
        .generate()?
        .write_to_file("src/abi/seaport_exchange.rs")?;
    Ok(())
}
