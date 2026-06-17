CREATE TABLE IF NOT EXISTS pools (
  address   TEXT PRIMARY KEY,
  token0    TEXT NOT NULL,
  token1    TEXT NOT NULL,
  stable    BOOLEAN NOT NULL,
  block_num BIGINT NOT NULL,
  tx_hash   TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS swaps (
  id          TEXT PRIMARY KEY,  -- "{tx_hash}-{log_index}"
  pool        TEXT NOT NULL,
  sender      TEXT NOT NULL,
  "to"        TEXT NOT NULL,
  amount0_in  NUMERIC NOT NULL,
  amount1_in  NUMERIC NOT NULL,
  amount0_out NUMERIC NOT NULL,
  amount1_out NUMERIC NOT NULL,
  block_num   BIGINT NOT NULL,
  timestamp   BIGINT NOT NULL,
  tx_hash     TEXT NOT NULL,
  log_index   BIGINT NOT NULL
);

CREATE TABLE IF NOT EXISTS liquidity_events (
  id         TEXT PRIMARY KEY,  -- "{tx_hash}-{log_index}"
  event_type TEXT NOT NULL,     -- "mint" or "burn"
  pool       TEXT NOT NULL,
  sender     TEXT NOT NULL,
  amount0    NUMERIC NOT NULL,
  amount1    NUMERIC NOT NULL,
  block_num  BIGINT NOT NULL,
  timestamp  BIGINT NOT NULL,
  tx_hash    TEXT NOT NULL,
  log_index  BIGINT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_swaps_pool ON swaps (pool);
CREATE INDEX IF NOT EXISTS idx_swaps_block ON swaps (block_num);
CREATE INDEX IF NOT EXISTS idx_liquidity_pool ON liquidity_events (pool);
CREATE INDEX IF NOT EXISTS idx_liquidity_block ON liquidity_events (block_num);
