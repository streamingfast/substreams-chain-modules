fn main() {
    substreams_ethereum::Abigen::new("Comet", "abi/Comet.json")
        .expect("failed to load Comet ABI")
        .generate()
        .expect("failed to generate Comet bindings")
        .write_to_file("src/abi/comet.rs")
        .expect("failed to write comet.rs");
}
