CREATE TABLE IF NOT EXISTS markets (
    market_id         VARCHAR PRIMARY KEY,
    loan_token        VARCHAR NOT NULL,
    collateral_token  VARCHAR NOT NULL,
    oracle            VARCHAR NOT NULL,
    irm               VARCHAR NOT NULL,
    lltv              NUMERIC NOT NULL,
    tx_hash           VARCHAR NOT NULL,
    log_index         BIGINT  NOT NULL,
    block_num         BIGINT  NOT NULL,
    timestamp         BIGINT  NOT NULL
);

CREATE TABLE IF NOT EXISTS supplies (
    id          VARCHAR PRIMARY KEY,
    market_id   VARCHAR NOT NULL,
    caller      VARCHAR NOT NULL,
    on_behalf   VARCHAR NOT NULL,
    assets      NUMERIC NOT NULL,
    shares      NUMERIC NOT NULL,
    tx_hash     VARCHAR NOT NULL,
    log_index   BIGINT  NOT NULL,
    block_num   BIGINT  NOT NULL,
    timestamp   BIGINT  NOT NULL
);

CREATE TABLE IF NOT EXISTS supply_collaterals (
    id          VARCHAR PRIMARY KEY,
    market_id   VARCHAR NOT NULL,
    caller      VARCHAR NOT NULL,
    on_behalf   VARCHAR NOT NULL,
    assets      NUMERIC NOT NULL,
    tx_hash     VARCHAR NOT NULL,
    log_index   BIGINT  NOT NULL,
    block_num   BIGINT  NOT NULL,
    timestamp   BIGINT  NOT NULL
);

CREATE TABLE IF NOT EXISTS borrows (
    id          VARCHAR PRIMARY KEY,
    market_id   VARCHAR NOT NULL,
    caller      VARCHAR NOT NULL,
    on_behalf   VARCHAR NOT NULL,
    receiver    VARCHAR NOT NULL,
    assets      NUMERIC NOT NULL,
    shares      NUMERIC NOT NULL,
    tx_hash     VARCHAR NOT NULL,
    log_index   BIGINT  NOT NULL,
    block_num   BIGINT  NOT NULL,
    timestamp   BIGINT  NOT NULL
);

CREATE TABLE IF NOT EXISTS repays (
    id          VARCHAR PRIMARY KEY,
    market_id   VARCHAR NOT NULL,
    caller      VARCHAR NOT NULL,
    on_behalf   VARCHAR NOT NULL,
    assets      NUMERIC NOT NULL,
    shares      NUMERIC NOT NULL,
    tx_hash     VARCHAR NOT NULL,
    log_index   BIGINT  NOT NULL,
    block_num   BIGINT  NOT NULL,
    timestamp   BIGINT  NOT NULL
);

CREATE TABLE IF NOT EXISTS withdraws (
    id          VARCHAR PRIMARY KEY,
    market_id   VARCHAR NOT NULL,
    caller      VARCHAR NOT NULL,
    on_behalf   VARCHAR NOT NULL,
    receiver    VARCHAR NOT NULL,
    assets      NUMERIC NOT NULL,
    shares      NUMERIC NOT NULL,
    tx_hash     VARCHAR NOT NULL,
    log_index   BIGINT  NOT NULL,
    block_num   BIGINT  NOT NULL,
    timestamp   BIGINT  NOT NULL
);

CREATE TABLE IF NOT EXISTS withdraw_collaterals (
    id          VARCHAR PRIMARY KEY,
    market_id   VARCHAR NOT NULL,
    caller      VARCHAR NOT NULL,
    on_behalf   VARCHAR NOT NULL,
    receiver    VARCHAR NOT NULL,
    assets      NUMERIC NOT NULL,
    tx_hash     VARCHAR NOT NULL,
    log_index   BIGINT  NOT NULL,
    block_num   BIGINT  NOT NULL,
    timestamp   BIGINT  NOT NULL
);

CREATE TABLE IF NOT EXISTS liquidations (
    id                VARCHAR PRIMARY KEY,
    market_id         VARCHAR NOT NULL,
    caller            VARCHAR NOT NULL,
    borrower          VARCHAR NOT NULL,
    repaid_assets     NUMERIC NOT NULL,
    repaid_shares     NUMERIC NOT NULL,
    seized_assets     NUMERIC NOT NULL,
    bad_debt_assets   NUMERIC NOT NULL,
    bad_debt_shares   NUMERIC NOT NULL,
    tx_hash           VARCHAR NOT NULL,
    log_index         BIGINT  NOT NULL,
    block_num         BIGINT  NOT NULL,
    timestamp         BIGINT  NOT NULL
);

CREATE TABLE IF NOT EXISTS accrued_interests (
    id                VARCHAR PRIMARY KEY,
    market_id         VARCHAR NOT NULL,
    prev_borrow_rate  NUMERIC NOT NULL,
    interest          NUMERIC NOT NULL,
    fee_shares        NUMERIC NOT NULL,
    tx_hash           VARCHAR NOT NULL,
    log_index         BIGINT  NOT NULL,
    block_num         BIGINT  NOT NULL,
    timestamp         BIGINT  NOT NULL
);

CREATE TABLE IF NOT EXISTS flash_loans (
    id          VARCHAR PRIMARY KEY,
    caller      VARCHAR NOT NULL,
    token       VARCHAR NOT NULL,
    assets      NUMERIC NOT NULL,
    tx_hash     VARCHAR NOT NULL,
    log_index   BIGINT  NOT NULL,
    block_num   BIGINT  NOT NULL,
    timestamp   BIGINT  NOT NULL
);
