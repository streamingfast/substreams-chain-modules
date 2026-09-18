fn main() {
    substreams_ethereum::Abigen::new("CtfExchangeV1", "abi/ctf_exchange_v1.json")
        .expect("failed to load CtfExchangeV1 ABI")
        .generate()
        .expect("failed to generate CtfExchangeV1 bindings")
        .write_to_file("src/abi/ctf_exchange_v1.rs")
        .expect("failed to write ctf_exchange_v1.rs");

    substreams_ethereum::Abigen::new("CtfExchangeV2", "abi/ctf_exchange_v2.json")
        .expect("failed to load CtfExchangeV2 ABI")
        .generate()
        .expect("failed to generate CtfExchangeV2 bindings")
        .write_to_file("src/abi/ctf_exchange_v2.rs")
        .expect("failed to write ctf_exchange_v2.rs");
}
