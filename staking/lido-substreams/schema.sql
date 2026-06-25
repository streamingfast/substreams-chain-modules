CREATE TABLE IF NOT EXISTS deposits (
    id             TEXT NOT NULL,
    sender         TEXT NOT NULL,
    amount_raw     TEXT NOT NULL,
    referral       TEXT NOT NULL,
    tx_hash        TEXT NOT NULL,
    log_index      BIGINT NOT NULL,
    block_num      BIGINT NOT NULL,
    timestamp      BIGINT NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS transfers (
    id             TEXT NOT NULL,
    "from"         TEXT NOT NULL,
    "to"           TEXT NOT NULL,
    value_raw      TEXT NOT NULL,
    tx_hash        TEXT NOT NULL,
    log_index      BIGINT NOT NULL,
    block_num      BIGINT NOT NULL,
    timestamp      BIGINT NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS eth_distributions (
    id                       TEXT NOT NULL,
    report_timestamp         TEXT NOT NULL,
    pre_cl_balance           TEXT NOT NULL,
    post_cl_balance          TEXT NOT NULL,
    withdrawals_withdrawn    TEXT NOT NULL,
    el_rewards_withdrawn     TEXT NOT NULL,
    post_buffered_ether      TEXT NOT NULL,
    tx_hash                  TEXT NOT NULL,
    log_index                BIGINT NOT NULL,
    block_num                BIGINT NOT NULL,
    timestamp                BIGINT NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS oracle_reports (
    id                        TEXT NOT NULL,
    post_total_pooled_ether   TEXT NOT NULL,
    pre_total_pooled_ether    TEXT NOT NULL,
    time_elapsed              TEXT NOT NULL,
    total_shares              TEXT NOT NULL,
    tx_hash                   TEXT NOT NULL,
    log_index                 BIGINT NOT NULL,
    block_num                 BIGINT NOT NULL,
    timestamp                 BIGINT NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS withdrawal_requests (
    id                TEXT NOT NULL,
    request_id        TEXT NOT NULL,
    requestor         TEXT NOT NULL,
    owner             TEXT NOT NULL,
    amount_of_st_eth  TEXT NOT NULL,
    amount_of_shares  TEXT NOT NULL,
    tx_hash           TEXT NOT NULL,
    log_index         BIGINT NOT NULL,
    block_num         BIGINT NOT NULL,
    timestamp         BIGINT NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS withdrawal_claims (
    id             TEXT NOT NULL,
    request_id     TEXT NOT NULL,
    owner          TEXT NOT NULL,
    receiver       TEXT NOT NULL,
    amount_of_eth  TEXT NOT NULL,
    tx_hash        TEXT NOT NULL,
    log_index      BIGINT NOT NULL,
    block_num      BIGINT NOT NULL,
    timestamp      BIGINT NOT NULL,
    PRIMARY KEY (id)
);
