CREATE TABLE IF NOT EXISTS stake_events (
    id                    TEXT NOT NULL,
    pool                  TEXT NOT NULL,
    staker                TEXT NOT NULL,
    event_type            TEXT NOT NULL,
    amount                TEXT NOT NULL,
    new_stake             TEXT NOT NULL,
    new_total_principal   TEXT NOT NULL,
    tx_hash               TEXT NOT NULL,
    log_index             BIGINT NOT NULL,
    block_num             BIGINT NOT NULL,
    timestamp             BIGINT NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS stake_events_v1 (
    id                  TEXT NOT NULL,
    staker              TEXT NOT NULL,
    event_type          TEXT NOT NULL,
    new_stake           TEXT NOT NULL,
    total_stake         TEXT NOT NULL,
    principal           TEXT NOT NULL,
    base_reward         TEXT NOT NULL,
    delegation_reward   TEXT NOT NULL,
    tx_hash             TEXT NOT NULL,
    log_index           BIGINT NOT NULL,
    block_num           BIGINT NOT NULL,
    timestamp           BIGINT NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS reward_updates (
    id                          TEXT NOT NULL,
    pool_type                   TEXT NOT NULL,
    base_reward_per_token       TEXT NOT NULL,
    delegated_reward_per_token  TEXT NOT NULL,
    tx_hash                     TEXT NOT NULL,
    log_index                   BIGINT NOT NULL,
    block_num                   BIGINT NOT NULL,
    timestamp                   BIGINT NOT NULL,
    PRIMARY KEY (id)
);

CREATE TABLE IF NOT EXISTS reward_vault_sets (
    id                TEXT NOT NULL,
    pool              TEXT NOT NULL,
    old_reward_vault  TEXT NOT NULL,
    new_reward_vault  TEXT NOT NULL,
    tx_hash           TEXT NOT NULL,
    log_index         BIGINT NOT NULL,
    block_num         BIGINT NOT NULL,
    timestamp         BIGINT NOT NULL,
    PRIMARY KEY (id)
);
