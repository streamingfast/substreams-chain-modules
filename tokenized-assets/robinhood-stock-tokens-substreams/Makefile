# `substreams pack` does not compile; it packages whatever wasm sits at the
# path in substreams.yaml. Always build before packing.

SPKG := robinhood-stock-tokens-substreams-v0.1.0.spkg
WASM := ../../target/wasm32-unknown-unknown/release/robinhood_stock_tokens_substreams.wasm
ENDPOINT ?= robinhood.substreams.pinax.network:443
START ?= 52700000
STOP ?= +5000

.PHONY: all protogen build pack test info run clean

all: protogen build pack

protogen:              ## regenerate src/pb from substreams.yaml (needs network for imports)
	substreams protogen

build:                 ## compile the wasm
	cargo build --target wasm32-unknown-unknown --release -p robinhood_stock_tokens_substreams

pack: build            ## build then package $(SPKG)
	substreams pack

test:                  ## host unit tests (calendar, price math, decoders, rows)
	cargo test -p robinhood_stock_tokens_substreams

info: $(SPKG)
	substreams info ./$(SPKG)

run: $(SPKG)           ## stream map_basis for a small window (needs `substreams auth`)
	substreams run ./$(SPKG) map_basis -e $(ENDPOINT) -s $(START) -t $(STOP)

clean:
	cargo clean
	rm -f $(SPKG)
