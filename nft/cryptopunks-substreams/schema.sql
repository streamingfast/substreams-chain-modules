CREATE TABLE IF NOT EXISTS trades (
    id                TEXT NOT NULL,
    punk_index        BIGINT NOT NULL,
    price_eth_raw     TEXT NOT NULL,
    buyer             TEXT NOT NULL,
    seller            TEXT NOT NULL,
    tx_hash           TEXT NOT NULL,
    log_index         BIGINT NOT NULL,
    block_num         BIGINT NOT NULL,
    timestamp         BIGINT NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS bids (
    id                TEXT NOT NULL,
    punk_index        BIGINT NOT NULL,
    amount_raw        TEXT NOT NULL,
    bidder            TEXT NOT NULL,
    event_type        TEXT NOT NULL,
    tx_hash           TEXT NOT NULL,
    log_index         BIGINT NOT NULL,
    block_num         BIGINT NOT NULL,
    timestamp         BIGINT NOT NULL,
    PRIMARY KEY (id)
);
