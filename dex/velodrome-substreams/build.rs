fn main() {
    substreams_ethereum::Abigen::new("PoolFactory", "abi/pool_factory.json")
        .expect("failed to load PoolFactory ABI")
        .generate()
        .expect("failed to generate PoolFactory bindings")
        .write_to_file("src/abi/pool_factory.rs")
        .expect("failed to write pool_factory.rs");

    substreams_ethereum::Abigen::new("Pool", "abi/pool.json")
        .expect("failed to load Pool ABI")
        .generate()
        .expect("failed to generate Pool bindings")
        .write_to_file("src/abi/pool.rs")
        .expect("failed to write pool.rs");
}
