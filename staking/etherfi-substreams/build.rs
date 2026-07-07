fn main() {
    substreams_ethereum::Abigen::new("EarlyAdopterPool", "abi/EarlyAdopterPool.json")
        .expect("failed to load EarlyAdopterPool ABI")
        .generate()
        .expect("failed to generate EarlyAdopterPool bindings")
        .write_to_file("src/abi/early_adopter_pool.rs")
        .expect("failed to write early_adopter_pool.rs");

    substreams_ethereum::Abigen::new("LiquidityPool", "abi/LiquidityPool.json")
        .expect("failed to load LiquidityPool ABI")
        .generate()
        .expect("failed to generate LiquidityPool bindings")
        .write_to_file("src/abi/liquidity_pool.rs")
        .expect("failed to write liquidity_pool.rs");

    substreams_ethereum::Abigen::new("EtherFiNodesManager", "abi/EtherFiNodesManager.json")
        .expect("failed to load EtherFiNodesManager ABI")
        .generate()
        .expect("failed to generate EtherFiNodesManager bindings")
        .write_to_file("src/abi/etherfi_nodes_manager.rs")
        .expect("failed to write etherfi_nodes_manager.rs");
}
