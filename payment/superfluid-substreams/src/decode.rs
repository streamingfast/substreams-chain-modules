use crate::abi;
use crate::pb::superfluid::v1 as sf;
use crate::PrefixedGet;
use substreams::Hex;
use substreams_ethereum::pb::eth::v2 as eth;
use substreams_ethereum::Event;
use std::collections::HashSet;

// Fixed Base mainnet protocol addresses (lowercase hex, no 0x).
// Source: superfluid-finance metadata networks.json (base-mainnet / startBlockV1).
// Generic-topic events (Role*, Set, Host) MUST be gated — OZ AccessControl
// RoleGranted/Revoked collide with thousands of unrelated contracts.
const HOST: &str = "4c073b3bab6d8826b8c5b229f3cfdc1ec6e47e74";
const CFA: &str = "19ba78b9cdb05a877718841c574325fdb53601bb";
const IDA: &str = "66df3f8e14cf870361378d8f61356d15d9f425c4";
const GDA: &str = "fe6c87be05fedb2059d2ec41ba0a09826c9fd7aa";
const FACTORY: &str = "e20b9a38e0c96f61d1ba6b42a61512d56fea1eb3";
const RESOLVER: &str = "6a214c324553f96f04efbdd66908685525da0e0d";
const TOGA: &str = "a87f76e99f6c8ff8996d14f550cef47f193d9a09";
const KNOWN_GOV: &str = "55f7758dd99d5e185f4cc08d4ad95b71f598264d";

struct BlockMeta {
    number: u64,
    timestamp: u64,
}

fn event_id(tx: &str, log_index: u32) -> String {
    format!("{tx}-{log_index}")
}

fn addr_hex(addr: &[u8]) -> String {
    hex::encode(addr)
}

#[inline]
fn is_fixed(contract: &str, expected: &str) -> bool {
    contract == expected
}

pub fn decode_block(
    blk: &eth::Block,
    super_tokens: &PrefixedGet<'_>,
    pools: &PrefixedGet<'_>,
    gov_addrs: &PrefixedGet<'_>,
) -> sf::Events {
    let mut out = sf::Events::default();
    let meta = BlockMeta {
        number: blk.number,
        timestamp: blk.timestamp_seconds(),
    };

    let mut local_super_tokens: HashSet<String> = HashSet::new();
    let mut local_pools: HashSet<String> = HashSet::new();
    let mut local_gov: HashSet<String> = HashSet::new();
    local_gov.insert(KNOWN_GOV.to_string());

    for receipt in blk.receipts() {
        for log in receipt.receipt.logs.iter() {
            let emitter = addr_hex(&log.address);
            if is_fixed(&emitter, FACTORY) {
                if let Some(ev) = abi::factory::events::SuperTokenCreated::match_and_decode(log) {
                    local_super_tokens.insert(hex::encode(&ev.token));
                }
                if let Some(ev) = abi::factory::events::CustomSuperTokenCreated::match_and_decode(log) {
                    local_super_tokens.insert(hex::encode(&ev.token));
                }
            }
            if is_fixed(&emitter, GDA) {
                if let Some(ev) = abi::gda::events::PoolCreated::match_and_decode(log) {
                    local_pools.insert(hex::encode(&ev.pool));
                }
            }
            if is_fixed(&emitter, HOST) {
                if let Some(ev) = abi::host::events::GovernanceReplaced::match_and_decode(log) {
                    local_gov.insert(hex::encode(&ev.new_gov));
                }
            }
        }
    }

    for receipt in blk.receipts() {
        let tx_hash = Hex(&receipt.transaction.hash).to_string();
        for log in receipt.receipt.logs.iter() {
            let contract = addr_hex(&log.address);
            let log_index = log.index;
            let id = event_id(&tx_hash, log_index);

            if let Some(event) = abi::cfa::events::FlowUpdated::match_and_decode(log) {
                if is_fixed(&contract, CFA) {
                    out.flow_updated_events.push(sf::FlowUpdatedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "FlowUpdated".to_string(),
                        token: hex::encode(&event.token),
                        sender: hex::encode(&event.sender),
                        receiver: hex::encode(&event.receiver),
                        flow_rate: event.flow_rate.to_string(),
                        total_sender_flow_rate: event.total_sender_flow_rate.to_string(),
                        total_receiver_flow_rate: event.total_receiver_flow_rate.to_string(),
                        user_data: hex::encode(&event.user_data),
                    });
                }
                continue;
            }

            if let Some(event) = abi::cfa::events::FlowUpdatedExtension::match_and_decode(log) {
                if is_fixed(&contract, CFA) {
                    out.flow_updated_extension_events.push(sf::FlowUpdatedExtensionEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "FlowUpdatedExtension".to_string(),
                        flow_operator: hex::encode(&event.flow_operator),
                        deposit: event.deposit.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::cfa::events::FlowOperatorUpdated::match_and_decode(log) {
                if is_fixed(&contract, CFA) {
                    out.flow_operator_updated_events.push(sf::FlowOperatorUpdatedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "FlowOperatorUpdated".to_string(),
                        token: hex::encode(&event.token),
                        sender: hex::encode(&event.sender),
                        flow_operator: hex::encode(&event.flow_operator),
                        permissions: event.permissions.to_string(),
                        flow_rate_allowance: event.flow_rate_allowance.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::ida::events::IndexCreated::match_and_decode(log) {
                if is_fixed(&contract, IDA) {
                    out.index_created_events.push(sf::IndexCreatedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "IndexCreated".to_string(),
                        token: hex::encode(&event.token),
                        publisher: hex::encode(&event.publisher),
                        index_id: event.index_id.to_string(),
                        user_data: hex::encode(&event.user_data),
                    });
                }
                continue;
            }

            if let Some(event) = abi::ida::events::IndexDistributionClaimed::match_and_decode(log) {
                if is_fixed(&contract, IDA) {
                    out.index_distribution_claimed_events.push(sf::IndexDistributionClaimedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "IndexDistributionClaimed".to_string(),
                        token: hex::encode(&event.token),
                        publisher: hex::encode(&event.publisher),
                        index_id: event.index_id.to_string(),
                        subscriber: hex::encode(&event.subscriber),
                        amount: event.amount.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::ida::events::IndexUpdated::match_and_decode(log) {
                if is_fixed(&contract, IDA) {
                    out.index_updated_events.push(sf::IndexUpdatedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "IndexUpdated".to_string(),
                        token: hex::encode(&event.token),
                        publisher: hex::encode(&event.publisher),
                        index_id: event.index_id.to_string(),
                        old_index_value: event.old_index_value.to_string(),
                        new_index_value: event.new_index_value.to_string(),
                        total_units_pending: event.total_units_pending.to_string(),
                        total_units_approved: event.total_units_approved.to_string(),
                        user_data: hex::encode(&event.user_data),
                    });
                }
                continue;
            }

            if let Some(event) = abi::ida::events::IndexSubscribed::match_and_decode(log) {
                if is_fixed(&contract, IDA) {
                    out.index_subscribed_events.push(sf::IndexSubscribedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "IndexSubscribed".to_string(),
                        token: hex::encode(&event.token),
                        publisher: hex::encode(&event.publisher),
                        index_id: event.index_id.to_string(),
                        subscriber: hex::encode(&event.subscriber),
                        user_data: hex::encode(&event.user_data),
                    });
                }
                continue;
            }

            if let Some(event) = abi::ida::events::IndexUnitsUpdated::match_and_decode(log) {
                if is_fixed(&contract, IDA) {
                    out.index_units_updated_events.push(sf::IndexUnitsUpdatedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "IndexUnitsUpdated".to_string(),
                        token: hex::encode(&event.token),
                        publisher: hex::encode(&event.publisher),
                        index_id: event.index_id.to_string(),
                        subscriber: hex::encode(&event.subscriber),
                        units: event.units.to_string(),
                        user_data: hex::encode(&event.user_data),
                    });
                }
                continue;
            }

            if let Some(event) = abi::ida::events::IndexUnsubscribed::match_and_decode(log) {
                if is_fixed(&contract, IDA) {
                    out.index_unsubscribed_events.push(sf::IndexUnsubscribedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "IndexUnsubscribed".to_string(),
                        token: hex::encode(&event.token),
                        publisher: hex::encode(&event.publisher),
                        index_id: event.index_id.to_string(),
                        subscriber: hex::encode(&event.subscriber),
                        user_data: hex::encode(&event.user_data),
                    });
                }
                continue;
            }

            if let Some(event) = abi::ida::events::SubscriptionApproved::match_and_decode(log) {
                if is_fixed(&contract, IDA) {
                    out.subscription_approved_events.push(sf::SubscriptionApprovedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "SubscriptionApproved".to_string(),
                        token: hex::encode(&event.token),
                        subscriber: hex::encode(&event.subscriber),
                        publisher: hex::encode(&event.publisher),
                        index_id: event.index_id.to_string(),
                        user_data: hex::encode(&event.user_data),
                    });
                }
                continue;
            }

            if let Some(event) = abi::ida::events::SubscriptionDistributionClaimed::match_and_decode(log) {
                if is_fixed(&contract, IDA) {
                    out.subscription_distribution_claimed_events.push(sf::SubscriptionDistributionClaimedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "SubscriptionDistributionClaimed".to_string(),
                        token: hex::encode(&event.token),
                        subscriber: hex::encode(&event.subscriber),
                        publisher: hex::encode(&event.publisher),
                        index_id: event.index_id.to_string(),
                        amount: event.amount.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::ida::events::SubscriptionRevoked::match_and_decode(log) {
                if is_fixed(&contract, IDA) {
                    out.subscription_revoked_events.push(sf::SubscriptionRevokedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "SubscriptionRevoked".to_string(),
                        token: hex::encode(&event.token),
                        subscriber: hex::encode(&event.subscriber),
                        publisher: hex::encode(&event.publisher),
                        index_id: event.index_id.to_string(),
                        user_data: hex::encode(&event.user_data),
                    });
                }
                continue;
            }

            if let Some(event) = abi::ida::events::SubscriptionUnitsUpdated::match_and_decode(log) {
                if is_fixed(&contract, IDA) {
                    out.subscription_units_updated_events.push(sf::SubscriptionUnitsUpdatedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "SubscriptionUnitsUpdated".to_string(),
                        token: hex::encode(&event.token),
                        subscriber: hex::encode(&event.subscriber),
                        publisher: hex::encode(&event.publisher),
                        index_id: event.index_id.to_string(),
                        units: event.units.to_string(),
                        user_data: hex::encode(&event.user_data),
                    });
                }
                continue;
            }

            if let Some(event) = abi::gda::events::BufferAdjusted::match_and_decode(log) {
                if is_fixed(&contract, GDA) {
                    out.buffer_adjusted_events.push(sf::BufferAdjustedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "BufferAdjusted".to_string(),
                        token: hex::encode(&event.token),
                        pool: hex::encode(&event.pool),
                        from: hex::encode(&event.from),
                        buffer_delta: event.buffer_delta.to_string(),
                        new_buffer_amount: event.new_buffer_amount.to_string(),
                        total_buffer_amount: event.total_buffer_amount.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::gda::events::FlowDistributionUpdated::match_and_decode(log) {
                if is_fixed(&contract, GDA) {
                    out.flow_distribution_updated_events.push(sf::FlowDistributionUpdatedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "FlowDistributionUpdated".to_string(),
                        token: hex::encode(&event.token),
                        pool: hex::encode(&event.pool),
                        distributor: hex::encode(&event.distributor),
                        operator: hex::encode(&event.operator),
                        old_flow_rate: event.old_flow_rate.to_string(),
                        new_distributor_to_pool_flow_rate: event.new_distributor_to_pool_flow_rate.to_string(),
                        new_total_distribution_flow_rate: event.new_total_distribution_flow_rate.to_string(),
                        adjustment_flow_recipient: hex::encode(&event.adjustment_flow_recipient),
                        adjustment_flow_rate: event.adjustment_flow_rate.to_string(),
                        user_data: hex::encode(&event.user_data),
                    });
                }
                continue;
            }

            if let Some(event) = abi::gda::events::InstantDistributionUpdated::match_and_decode(log) {
                if is_fixed(&contract, GDA) {
                    out.instant_distribution_updated_events.push(sf::InstantDistributionUpdatedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "InstantDistributionUpdated".to_string(),
                        token: hex::encode(&event.token),
                        pool: hex::encode(&event.pool),
                        distributor: hex::encode(&event.distributor),
                        operator: hex::encode(&event.operator),
                        requested_amount: event.requested_amount.to_string(),
                        actual_amount: event.actual_amount.to_string(),
                        user_data: hex::encode(&event.user_data),
                    });
                }
                continue;
            }

            if let Some(event) = abi::gda::events::PoolConnectionUpdated::match_and_decode(log) {
                if is_fixed(&contract, GDA) {
                    out.pool_connection_updated_events.push(sf::PoolConnectionUpdatedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "PoolConnectionUpdated".to_string(),
                        token: hex::encode(&event.token),
                        pool: hex::encode(&event.pool),
                        account: hex::encode(&event.account),
                        connected: event.connected,
                        user_data: hex::encode(&event.user_data),
                    });
                }
                continue;
            }

            if let Some(event) = abi::gda::events::PoolCreated::match_and_decode(log) {
                if is_fixed(&contract, GDA) {
                    out.pool_created_events.push(sf::PoolCreatedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "PoolCreated".to_string(),
                        token: hex::encode(&event.token),
                        admin: hex::encode(&event.admin),
                        pool: hex::encode(&event.pool),
                    });
                }
                continue;
            }

            if let Some(event) = abi::factory::events::SuperTokenCreated::match_and_decode(log) {
                if is_fixed(&contract, FACTORY) {
                    out.super_token_created_events.push(sf::SuperTokenCreatedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "SuperTokenCreated".to_string(),
                        token: hex::encode(&event.token),
                    });
                }
                continue;
            }

            if let Some(event) = abi::factory::events::CustomSuperTokenCreated::match_and_decode(log) {
                if is_fixed(&contract, FACTORY) {
                    out.custom_super_token_created_events.push(sf::CustomSuperTokenCreatedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "CustomSuperTokenCreated".to_string(),
                        token: hex::encode(&event.token),
                    });
                }
                continue;
            }

            if let Some(event) = abi::factory::events::SuperTokenLogicCreated::match_and_decode(log) {
                if is_fixed(&contract, FACTORY) {
                    out.super_token_logic_created_events.push(sf::SuperTokenLogicCreatedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "SuperTokenLogicCreated".to_string(),
                        token_logic: hex::encode(&event.token_logic),
                    });
                }
                continue;
            }

            if let Some(event) = abi::super_token::events::AgreementLiquidatedBy::match_and_decode(log) {
                if super_tokens.get_last(&contract).is_some() || local_super_tokens.contains(&contract) {
                    out.agreement_liquidated_by_events.push(sf::AgreementLiquidatedByEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "AgreementLiquidatedBy".to_string(),
                        liquidator_account: hex::encode(&event.liquidator_account),
                        agreement_class: hex::encode(&event.agreement_class),
                        id_2: hex::encode(&event.id),
                        penalty_account: hex::encode(&event.penalty_account),
                        bond_account: hex::encode(&event.bond_account),
                        reward_amount: event.reward_amount.to_string(),
                        bailout_amount: event.bailout_amount.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::super_token::events::AgreementLiquidatedV2::match_and_decode(log) {
                if super_tokens.get_last(&contract).is_some() || local_super_tokens.contains(&contract) {
                    out.agreement_liquidated_v2_events.push(sf::AgreementLiquidatedV2Event {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "AgreementLiquidatedV2".to_string(),
                        agreement_class: hex::encode(&event.agreement_class),
                        id_2: hex::encode(&event.id),
                        liquidator_account: hex::encode(&event.liquidator_account),
                        target_account: hex::encode(&event.target_account),
                        reward_amount_receiver: hex::encode(&event.reward_amount_receiver),
                        reward_amount: event.reward_amount.to_string(),
                        target_account_balance_delta: event.target_account_balance_delta.to_string(),
                        liquidation_type_data: hex::encode(&event.liquidation_type_data),
                    });
                }
                continue;
            }

            if let Some(event) = abi::super_token::events::Burned::match_and_decode(log) {
                if super_tokens.get_last(&contract).is_some() || local_super_tokens.contains(&contract) {
                    out.burned_events.push(sf::BurnedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "Burned".to_string(),
                        operator: hex::encode(&event.operator),
                        from: hex::encode(&event.from),
                        amount: event.amount.to_string(),
                        data: hex::encode(&event.data),
                        operator_data: hex::encode(&event.operator_data),
                    });
                }
                continue;
            }

            if let Some(event) = abi::super_token::events::Minted::match_and_decode(log) {
                if super_tokens.get_last(&contract).is_some() || local_super_tokens.contains(&contract) {
                    out.minted_events.push(sf::MintedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "Minted".to_string(),
                        operator: hex::encode(&event.operator),
                        to: hex::encode(&event.to),
                        amount: event.amount.to_string(),
                        data: hex::encode(&event.data),
                        operator_data: hex::encode(&event.operator_data),
                    });
                }
                continue;
            }

            if let Some(event) = abi::super_token::events::Sent::match_and_decode(log) {
                if super_tokens.get_last(&contract).is_some() || local_super_tokens.contains(&contract) {
                    out.sent_events.push(sf::SentEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "Sent".to_string(),
                        operator: hex::encode(&event.operator),
                        from: hex::encode(&event.from),
                        to: hex::encode(&event.to),
                        amount: event.amount.to_string(),
                        data: hex::encode(&event.data),
                        operator_data: hex::encode(&event.operator_data),
                    });
                }
                continue;
            }

            if let Some(event) = abi::super_token::events::TokenUpgraded::match_and_decode(log) {
                if super_tokens.get_last(&contract).is_some() || local_super_tokens.contains(&contract) {
                    out.token_upgraded_events.push(sf::TokenUpgradedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "TokenUpgraded".to_string(),
                        account: hex::encode(&event.account),
                        amount: event.amount.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::super_token::events::TokenDowngraded::match_and_decode(log) {
                if super_tokens.get_last(&contract).is_some() || local_super_tokens.contains(&contract) {
                    out.token_downgraded_events.push(sf::TokenDowngradedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "TokenDowngraded".to_string(),
                        account: hex::encode(&event.account),
                        amount: event.amount.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::super_token::events::Transfer::match_and_decode(log) {
                if super_tokens.get_last(&contract).is_some() || local_super_tokens.contains(&contract) {
                    out.transfer_events.push(sf::TransferEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "Transfer".to_string(),
                        from: hex::encode(&event.from),
                        to: hex::encode(&event.to),
                        value: event.value.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::super_token::events::Approval::match_and_decode(log) {
                if super_tokens.get_last(&contract).is_some() || local_super_tokens.contains(&contract) {
                    out.approval_events.push(sf::ApprovalEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "Approval".to_string(),
                        owner: hex::encode(&event.owner),
                        spender: hex::encode(&event.spender),
                        value: event.value.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::host::events::GovernanceReplaced::match_and_decode(log) {
                if is_fixed(&contract, HOST) {
                    out.governance_replaced_events.push(sf::GovernanceReplacedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "GovernanceReplaced".to_string(),
                        old_gov: hex::encode(&event.old_gov),
                        new_gov: hex::encode(&event.new_gov),
                    });
                }
                continue;
            }

            if let Some(event) = abi::host::events::AgreementClassRegistered::match_and_decode(log) {
                if is_fixed(&contract, HOST) {
                    out.agreement_class_registered_events.push(sf::AgreementClassRegisteredEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "AgreementClassRegistered".to_string(),
                        agreement_type: hex::encode(&event.agreement_type),
                        code: hex::encode(&event.code),
                    });
                }
                continue;
            }

            if let Some(event) = abi::host::events::AgreementClassUpdated::match_and_decode(log) {
                if is_fixed(&contract, HOST) {
                    out.agreement_class_updated_events.push(sf::AgreementClassUpdatedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "AgreementClassUpdated".to_string(),
                        agreement_type: hex::encode(&event.agreement_type),
                        code: hex::encode(&event.code),
                    });
                }
                continue;
            }

            if let Some(event) = abi::host::events::SuperTokenFactoryUpdated::match_and_decode(log) {
                if is_fixed(&contract, HOST) {
                    out.super_token_factory_updated_events.push(sf::SuperTokenFactoryUpdatedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "SuperTokenFactoryUpdated".to_string(),
                        new_factory: hex::encode(&event.new_factory),
                    });
                }
                continue;
            }

            if let Some(event) = abi::host::events::SuperTokenLogicUpdated::match_and_decode(log) {
                if is_fixed(&contract, HOST) {
                    out.super_token_logic_updated_events.push(sf::SuperTokenLogicUpdatedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "SuperTokenLogicUpdated".to_string(),
                        token: hex::encode(&event.token),
                        code: hex::encode(&event.code),
                    });
                }
                continue;
            }

            if let Some(event) = abi::host::events::AppRegistered::match_and_decode(log) {
                if is_fixed(&contract, HOST) {
                    out.app_registered_events.push(sf::AppRegisteredEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "AppRegistered".to_string(),
                        app: hex::encode(&event.app),
                    });
                }
                continue;
            }

            if let Some(event) = abi::host::events::Jail::match_and_decode(log) {
                if is_fixed(&contract, HOST) {
                    out.jail_events.push(sf::JailEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "Jail".to_string(),
                        app: hex::encode(&event.app),
                        reason: event.reason.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::resolver::events::RoleAdminChanged::match_and_decode(log) {
                if is_fixed(&contract, RESOLVER) {
                    out.role_admin_changed_events.push(sf::RoleAdminChangedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "RoleAdminChanged".to_string(),
                        role: hex::encode(&event.role),
                        previous_admin_role: hex::encode(&event.previous_admin_role),
                        new_admin_role: hex::encode(&event.new_admin_role),
                    });
                }
                continue;
            }

            if let Some(event) = abi::resolver::events::RoleGranted::match_and_decode(log) {
                if is_fixed(&contract, RESOLVER) {
                    out.role_granted_events.push(sf::RoleGrantedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "RoleGranted".to_string(),
                        role: hex::encode(&event.role),
                        account: hex::encode(&event.account),
                        sender: hex::encode(&event.sender),
                    });
                }
                continue;
            }

            if let Some(event) = abi::resolver::events::RoleRevoked::match_and_decode(log) {
                if is_fixed(&contract, RESOLVER) {
                    out.role_revoked_events.push(sf::RoleRevokedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "RoleRevoked".to_string(),
                        role: hex::encode(&event.role),
                        account: hex::encode(&event.account),
                        sender: hex::encode(&event.sender),
                    });
                }
                continue;
            }

            if let Some(event) = abi::resolver::events::Set::match_and_decode(log) {
                if is_fixed(&contract, RESOLVER) {
                    out.set_events.push(sf::SetEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "Set".to_string(),
                        name: hex::encode(&event.name.hash),
                        target: hex::encode(&event.target),
                    });
                }
                continue;
            }

            if let Some(event) = abi::governance::events::ConfigChanged::match_and_decode(log) {
                if gov_addrs.get_last(&contract).is_some() || local_gov.contains(&contract) {
                    out.config_changed_events.push(sf::ConfigChangedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "ConfigChanged".to_string(),
                        host: hex::encode(&event.host),
                        super_token: hex::encode(&event.super_token),
                        key: hex::encode(&event.key),
                        is_key_set: event.is_key_set,
                        value: event.value.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::governance::events::RewardAddressChanged::match_and_decode(log) {
                if gov_addrs.get_last(&contract).is_some() || local_gov.contains(&contract) {
                    out.reward_address_changed_events.push(sf::RewardAddressChangedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "RewardAddressChanged".to_string(),
                        host: hex::encode(&event.host),
                        super_token: hex::encode(&event.super_token),
                        is_key_set: event.is_key_set,
                        reward_address: hex::encode(&event.reward_address),
                    });
                }
                continue;
            }

            if let Some(event) = abi::governance::events::CfAv1LiquidationPeriodChanged::match_and_decode(log) {
                if gov_addrs.get_last(&contract).is_some() || local_gov.contains(&contract) {
                    out.cfav1_liquidation_period_changed_events.push(sf::CfAv1LiquidationPeriodChangedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "CFAv1LiquidationPeriodChanged".to_string(),
                        host: hex::encode(&event.host),
                        super_token: hex::encode(&event.super_token),
                        is_key_set: event.is_key_set,
                        liquidation_period: event.liquidation_period.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::governance::events::PppConfigurationChanged::match_and_decode(log) {
                if gov_addrs.get_last(&contract).is_some() || local_gov.contains(&contract) {
                    out.pppconfiguration_changed_events.push(sf::PppConfigurationChangedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "PPPConfigurationChanged".to_string(),
                        host: hex::encode(&event.host),
                        super_token: hex::encode(&event.super_token),
                        is_key_set: event.is_key_set,
                        liquidation_period: event.liquidation_period.to_string(),
                        patrician_period: event.patrician_period.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::governance::events::SuperTokenMinimumDepositChanged::match_and_decode(log) {
                if gov_addrs.get_last(&contract).is_some() || local_gov.contains(&contract) {
                    out.super_token_minimum_deposit_changed_events.push(sf::SuperTokenMinimumDepositChangedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "SuperTokenMinimumDepositChanged".to_string(),
                        host: hex::encode(&event.host),
                        super_token: hex::encode(&event.super_token),
                        is_key_set: event.is_key_set,
                        minimum_deposit: event.minimum_deposit.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::governance::events::TrustedForwarderChanged::match_and_decode(log) {
                if gov_addrs.get_last(&contract).is_some() || local_gov.contains(&contract) {
                    out.trusted_forwarder_changed_events.push(sf::TrustedForwarderChangedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "TrustedForwarderChanged".to_string(),
                        host: hex::encode(&event.host),
                        super_token: hex::encode(&event.super_token),
                        is_key_set: event.is_key_set,
                        forwarder: hex::encode(&event.forwarder),
                        enabled: event.enabled,
                    });
                }
                continue;
            }

            if let Some(event) = abi::toga::events::NewPic::match_and_decode(log) {
                if is_fixed(&contract, TOGA) {
                    out.new_pic_events.push(sf::NewPicEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "NewPIC".to_string(),
                        token: hex::encode(&event.token),
                        pic: hex::encode(&event.pic),
                        bond: event.bond.to_string(),
                        exit_rate: event.exit_rate.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::toga::events::ExitRateChanged::match_and_decode(log) {
                if is_fixed(&contract, TOGA) {
                    out.exit_rate_changed_events.push(sf::ExitRateChangedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "ExitRateChanged".to_string(),
                        token: hex::encode(&event.token),
                        exit_rate: event.exit_rate.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::toga::events::BondIncreased::match_and_decode(log) {
                if is_fixed(&contract, TOGA) {
                    out.bond_increased_events.push(sf::BondIncreasedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "BondIncreased".to_string(),
                        token: hex::encode(&event.token),
                        additional_bond: event.additional_bond.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::pool::events::MemberUnitsUpdated::match_and_decode(log) {
                if pools.get_last(&contract).is_some() || local_pools.contains(&contract) {
                    out.member_units_updated_events.push(sf::MemberUnitsUpdatedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "MemberUnitsUpdated".to_string(),
                        token: hex::encode(&event.token),
                        member: hex::encode(&event.member),
                        old_units: event.old_units.to_string(),
                        new_units: event.new_units.to_string(),
                    });
                }
                continue;
            }

            if let Some(event) = abi::pool::events::DistributionClaimed::match_and_decode(log) {
                if pools.get_last(&contract).is_some() || local_pools.contains(&contract) {
                    out.distribution_claimed_events.push(sf::DistributionClaimedEvent {
                        id: id.clone(),
                        block_number: meta.number,
                        block_timestamp: meta.timestamp,
                        transaction_hash: tx_hash.clone(),
                        log_index,
                        contract: contract.clone(),
                        event_name: "DistributionClaimed".to_string(),
                        token: hex::encode(&event.token),
                        member: hex::encode(&event.member),
                        claimed_amount: event.claimed_amount.to_string(),
                        total_claimed: event.total_claimed.to_string(),
                    });
                }
                continue;
            }
        }
    }
    out
}
