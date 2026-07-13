use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("ListaStakeManager", "abi/ListaStakeManager.json")?
        .generate()?
        .write_to_file("src/abi/lista_stake_manager.rs")?;
    Ok(())
}
