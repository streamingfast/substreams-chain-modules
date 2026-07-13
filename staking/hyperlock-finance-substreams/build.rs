use anyhow::Result;
use substreams_ethereum::Abigen;

fn main() -> Result<()> {
    Abigen::new("ThrusterPointNFT", "abi/ThrusterPointNFT.json")?
        .generate()?
        .write_to_file("src/abi/thruster_point_nft.rs")?;
    Abigen::new("ERC20PointsDeposit", "abi/ERC20PointsDeposit.json")?
        .generate()?
        .write_to_file("src/abi/erc20_points_deposit.rs")?;
    Abigen::new("ERC721PointsDeposit", "abi/ERC721PointsDeposit.json")?
        .generate()?
        .write_to_file("src/abi/erc721_points_deposit.rs")?;
    Ok(())
}
