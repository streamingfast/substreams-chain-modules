fn main() {
    substreams_ethereum::Abigen::new("Lido", "abi/lido.json")
        .expect("failed to load Lido ABI")
        .generate()
        .expect("failed to generate Lido bindings")
        .write_to_file("src/abi/lido.rs")
        .expect("failed to write lido.rs");

    substreams_ethereum::Abigen::new("LidoOracle", "abi/lido_oracle.json")
        .expect("failed to load LidoOracle ABI")
        .generate()
        .expect("failed to generate LidoOracle bindings")
        .write_to_file("src/abi/lido_oracle.rs")
        .expect("failed to write lido_oracle.rs");

    substreams_ethereum::Abigen::new("WithdrawalQueue", "abi/withdrawal_queue.json")
        .expect("failed to load WithdrawalQueue ABI")
        .generate()
        .expect("failed to generate WithdrawalQueue bindings")
        .write_to_file("src/abi/withdrawal_queue.rs")
        .expect("failed to write withdrawal_queue.rs");
}
