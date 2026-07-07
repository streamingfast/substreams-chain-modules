fn main() {
    substreams_ethereum::Abigen::new("LooksRareExchange", "abi/LooksRareExchange.json")
        .expect("failed to load LooksRareExchange ABI")
        .generate()
        .expect("failed to generate LooksRareExchange bindings")
        .write_to_file("src/abi/looksrare_exchange.rs")
        .expect("failed to write looksrare_exchange.rs");
}
