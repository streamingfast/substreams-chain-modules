fn main() {
    substreams_ethereum::Abigen::new("MorphoBlue", "abi/MorphoBlue.json")
        .expect("failed to load MorphoBlue ABI")
        .generate()
        .expect("failed to generate MorphoBlue bindings")
        .write_to_file("src/abi/morpho_blue.rs")
        .expect("failed to write morpho_blue.rs");
}
