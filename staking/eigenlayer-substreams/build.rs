fn main() {
    substreams_ethereum::Abigen::new("StrategyManager", "abi/StrategyManager.json")
        .expect("failed to load StrategyManager ABI")
        .generate()
        .expect("failed to generate StrategyManager bindings")
        .write_to_file("src/abi/strategy_manager.rs")
        .expect("failed to write strategy_manager.rs");

    substreams_ethereum::Abigen::new("EigenPodManager", "abi/EigenPodManager.json")
        .expect("failed to load EigenPodManager ABI")
        .generate()
        .expect("failed to generate EigenPodManager bindings")
        .write_to_file("src/abi/eigen_pod_manager.rs")
        .expect("failed to write eigen_pod_manager.rs");
}
