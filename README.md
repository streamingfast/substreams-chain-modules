# Substreams Chain Modules

Substreams modules for various blockchains around datasets like DEX, Stablecoins, Prediction Markets, Tokenized Assets and more.

## Overview

This repository contains [Substreams](https://substreams.streamingfast.io) modules that extract and process higher-level blockchain datasets across multiple chains. While [substreams-foundational-modules](https://github.com/streamingfast/substreams-foundational-modules) provides low-level data extraction and filtering primitives, this project builds on top of them to deliver domain-specific datasets.

### Datasets

| Dataset | Description |
|---------|-------------|
| DEX | Decentralized exchange trades, pools, and liquidity events |
| Stablecoins | Stablecoin transfers, mints, burns, and supply tracking |
| Prediction Markets | Market creation, trading, and resolution events |
| Tokenized Assets | Real-world asset tokenization events and transfers |

## Prerequisites

- [Rust](https://rustup.rs/) (see [rust-toolchain.toml](rust-toolchain.toml) for version)
- [Substreams CLI](https://substreams.streamingfast.io/getting-started/installing-the-cli)

## License

[Apache 2.0](LICENSE)
