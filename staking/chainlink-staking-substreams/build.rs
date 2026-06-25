fn main() {
    substreams_ethereum::Abigen::new("StakingPool", "abi/staking_pool.json")
        .expect("failed to load StakingPool ABI")
        .generate()
        .expect("failed to generate StakingPool bindings")
        .write_to_file("src/abi/staking_pool.rs")
        .expect("failed to write staking_pool.rs");

    substreams_ethereum::Abigen::new("StakingV1", "abi/staking_v1.json")
        .expect("failed to load StakingV1 ABI")
        .generate()
        .expect("failed to generate StakingV1 bindings")
        .write_to_file("src/abi/staking_v1.rs")
        .expect("failed to write staking_v1.rs");

    substreams_ethereum::Abigen::new("RewardVault", "abi/reward_vault.json")
        .expect("failed to load RewardVault ABI")
        .generate()
        .expect("failed to generate RewardVault bindings")
        .write_to_file("src/abi/reward_vault.rs")
        .expect("failed to write reward_vault.rs");
}
