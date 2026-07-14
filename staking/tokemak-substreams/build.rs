use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("Manager", "abi/Manager.json")?
        .generate()?
        .write_to_file("src/abi/manager.rs")?;
    Abigen::new("Vault", "abi/Vault.json")?
        .generate()?
        .write_to_file("src/abi/vault.rs")?;
    Abigen::new("Rewards", "abi/Rewards.json")?
        .generate()?
        .write_to_file("src/abi/rewards.rs")?;
    Abigen::new("OnChainVote", "abi/OnChainVote.json")?
        .generate()?
        .write_to_file("src/abi/on_chain_vote.rs")?;
    Ok(())
}
