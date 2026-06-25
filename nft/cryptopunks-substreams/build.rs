fn main() {
    substreams_ethereum::Abigen::new("Cryptopunks", "abi/cryptopunks.json")
        .expect("failed to load Cryptopunks ABI")
        .generate()
        .expect("failed to generate Cryptopunks bindings")
        .write_to_file("src/abi/cryptopunks.rs")
        .expect("failed to write cryptopunks.rs");
}
