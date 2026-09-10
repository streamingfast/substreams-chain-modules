//! Chainlink `AnswerUpdated(int256 indexed current, uint256 indexed roundId,
//! uint256 updatedAt)` from the tokenized-equity aggregators.

use hex_literal::hex;
use substreams::scalar::BigInt;
use substreams::Hex;
use substreams_ethereum::pb::eth::v2::BlockLazyView;
use substreams_ethereum::LogLike;

use crate::pb::hood::basis::v1::{ChainlinkAnswer, ChainlinkAnswers};
use crate::price;
use crate::registry;

pub const ANSWER_UPDATED_TOPIC: [u8; 32] = hex!("0559884fd3a460db3073b7fc896cc77986f16e378210ded43186175bf646fc5f");

const ANSWER_DECIMALS: u64 = 8;

pub struct AnswerUpdated {
    pub current: BigInt,
    pub round_id: BigInt,
    pub updated_at: u64,
}

pub fn decode<L: LogLike + ?Sized>(log: &L) -> Option<AnswerUpdated> {
    let topic0 = log.topic(0)?;
    let topic1 = log.topic(1)?;
    let topic2 = log.topic(2)?;
    let data = log.data();

    if log.topic_count() != 3
        || topic0 != ANSWER_UPDATED_TOPIC
        || topic1.len() != 32
        || topic2.len() != 32
        || data.len() < 32
    {
        return None;
    }
    let updated_at = u64::try_from(&BigInt::from_unsigned_bytes_be(&data[..32])).ok()?;
    Some(AnswerUpdated {
        current: BigInt::from_signed_bytes_be(topic1),
        round_id: BigInt::from_unsigned_bytes_be(topic2),
        updated_at,
    })
}

pub fn build(block: &BlockLazyView<'_>) -> ChainlinkAnswers {
    let block_num = block.number;
    let block_ts = block
        .header
        .get()
        .ok()
        .flatten()
        .map(|h| h.timestamp.as_option().map(|t| t.seconds).unwrap_or(0))
        .unwrap_or(0) as u64;
    let mut out = ChainlinkAnswers::default();

    for trx in block.transactions() {
        let Ok(Some(receipt)) = trx.receipt() else {
            continue;
        };
        for log in receipt.logs.iter() {
            let Ok(log) = log else { continue };

            if log.topic(0) != Some(&ANSWER_UPDATED_TOPIC[..]) {
                continue;
            }
            let aggregator = addr_hex(log.address);
            let Some(feed) = registry::feed_by_aggregator(&aggregator) else {
                continue;
            };
            let Some(ev) = decode(&log) else {
                continue;
            };
            out.answers.push(ChainlinkAnswer {
                block_num,
                block_ts,
                tx_hash: format!("0x{}", Hex::encode(&trx.hash)),
                log_index: log.block_index,
                ticker: feed.ticker.to_string(),
                feed: feed.proxy.to_string(),
                aggregator,
                answer_usd: price::fmt(&ev.current.to_decimal(ANSWER_DECIMALS)),
                round_id: ev.round_id.to_string(),
                updated_at: ev.updated_at,
                ..Default::default()
            });
        }
    }
    out
}

pub fn addr_hex(bytes: &[u8]) -> String {
    format!("0x{}", Hex::encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use substreams_ethereum::pb::eth::v2::Log;

    fn word_u64(v: u64) -> Vec<u8> {
        let mut w = vec![0u8; 32];
        w[24..].copy_from_slice(&v.to_be_bytes());
        w
    }

    fn word_i128(v: i128) -> Vec<u8> {
        let mut w = if v < 0 { vec![0xffu8; 32] } else { vec![0u8; 32] };
        w[16..].copy_from_slice(&v.to_be_bytes());
        w
    }

    fn answer_log(current: i128, round_id: u64, updated_at: u64) -> Log {
        Log {
            address: hex!("0e96b7708487f91baac09697593d3e8bf253f2d8").to_vec(),
            topics: vec![ANSWER_UPDATED_TOPIC.to_vec(), word_i128(current), word_u64(round_id)],
            data: word_u64(updated_at),
            block_index: 7,
            ..Default::default()
        }
    }

    #[test]
    fn decodes_hand_built_answer_updated() {
        let ev = decode(&answer_log(12_345_678_900, 42, 1_781_706_600)).unwrap();
        assert_eq!(ev.current.to_string(), "12345678900");
        assert_eq!(price::fmt(&ev.current.to_decimal(ANSWER_DECIMALS)), "123.456789");
        assert_eq!(ev.round_id.to_string(), "42");
        assert_eq!(ev.updated_at, 1_781_706_600);
    }

    #[test]
    fn decodes_negative_int256() {
        let ev = decode(&answer_log(-1, 1, 0)).unwrap();
        assert_eq!(ev.current.to_string(), "-1");
        assert_eq!(price::fmt(&ev.current.to_decimal(ANSWER_DECIMALS)), "-0.00000001");
    }

    #[test]
    fn rejects_malformed_logs() {
        let mut wrong_topic = answer_log(1, 1, 1);
        wrong_topic.topics[0][0] ^= 0xff;
        assert!(decode(&wrong_topic).is_none());

        let mut missing_topic = answer_log(1, 1, 1);
        missing_topic.topics.pop();
        assert!(decode(&missing_topic).is_none());

        let mut short_data = answer_log(1, 1, 1);
        short_data.data.truncate(31);
        assert!(decode(&short_data).is_none());
    }

    #[test]
    fn aggregator_address_resolves_to_feed() {
        let log = answer_log(1, 1, 1);
        let feed = registry::feed_by_aggregator(&addr_hex(&log.address)).unwrap();
        assert_eq!(feed.ticker, "SGOV");
        assert_eq!(feed.proxy, "0xa0df4ee0fff975306345875e3548fcc519577a11");
    }
}
