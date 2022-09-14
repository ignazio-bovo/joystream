// Copyright 2019 Joystream Contributors
// This file is part of Joystream node.

// Joystream node is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Joystream node is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Joystream node.  If not, see <http://www.gnu.org/licenses/>.

// Clippy linter warning.
// Disable it because we use such syntax for a code readability.
// Example:  voting_period: 1 * DAY
#![allow(clippy::identity_op)]

pub mod content_config;
pub mod council_config;
pub mod forum_config;
pub mod initial_balances;
pub mod storage_config;

use grandpa_primitives::AuthorityId as GrandpaId;

use node_runtime::{
    constants::currency::{ENDOWMENT, MIN_NOMINATOR_BOND, MIN_VALIDATOR_BOND, STASH},
    wasm_binary_unwrap, BabeConfig, BalancesConfig, Block, ContentConfig,
    GrandpaConfig, SessionKeys, StakerStatus,
    StorageConfig, SudoConfig, SystemConfig, TransactionPaymentConfig,
    VestingConfig,
};
use sc_chain_spec::ChainSpecExtension;
use sc_service::ChainType;

use serde::{Deserialize, Serialize};
use serde_json as json;
use sp_consensus_babe::AuthorityId as BabeId;
use sp_core::{sr25519, Pair, Public};
use sp_runtime::{
    traits::{IdentifyAccount, Verify},
    Perbill,
};

pub use node_runtime::constants::JOY_ADDRESS_PREFIX;
pub use node_runtime::primitives::{AccountId, Balance, BlockNumber, Signature};
pub use node_runtime::GenesisConfig;

type AccountPublic = <Signature as Verify>::Signer;

/// Node `ChainSpec` extensions.
///
/// Additional parameters for some Substrate core modules,
/// customizable from the chain spec.
#[derive(Default, Clone, Serialize, Deserialize, ChainSpecExtension)]
#[serde(rename_all = "camelCase")]
pub struct Extensions {
    /// Block numbers with known hashes.
    pub fork_blocks: sc_client_api::ForkBlocks<Block>,
    /// Known bad block hashes.
    pub bad_blocks: sc_client_api::BadBlocks<Block>,
    /// The light sync state extension used by the sync-state rpc.
    pub light_sync_state: sc_sync_state_rpc::LightSyncStateExtension,
}

/// Specialized `ChainSpec`.
pub type ChainSpec = sc_service::GenericChainSpec<GenesisConfig, Extensions>;

fn session_keys(
    grandpa: GrandpaId,
    babe: BabeId,
) -> SessionKeys {
    SessionKeys {
        grandpa,
        babe,
    }
}

/// Helper function to generate a crypto pair from seed
pub fn get_from_seed<TPublic: Public>(seed: &str) -> <TPublic::Pair as Pair>::Public {
    TPublic::Pair::from_string(&format!("//{}", seed), None)
        .expect("static values are valid; qed")
        .public()
}

/// Helper function to generate an account ID from seed
pub fn get_account_id_from_seed<TPublic: Public>(seed: &str) -> AccountId
where
    AccountPublic: From<<TPublic::Pair as Pair>::Public>,
{
    AccountPublic::from(get_from_seed::<TPublic>(seed)).into_account()
}

/// Helper function to generate stash, controller and session key from seed
pub fn authority_keys_from_seed(
    seed: &str,
) -> (
    AccountId,
    AccountId,
    GrandpaId,
    BabeId,
) {
    (
        get_account_id_from_seed::<sr25519::Public>(&format!("{}//stash", seed)),
        get_account_id_from_seed::<sr25519::Public>(seed),
        get_from_seed::<GrandpaId>(seed),
        get_from_seed::<BabeId>(seed),
    )
}

// Accounts to endow on dev and local test networks
fn development_endowed_accounts() -> Vec<AccountId> {
    vec![
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        get_account_id_from_seed::<sr25519::Public>("Bob"),
        get_account_id_from_seed::<sr25519::Public>("Charlie"),
        get_account_id_from_seed::<sr25519::Public>("Dave"),
        get_account_id_from_seed::<sr25519::Public>("Eve"),
        get_account_id_from_seed::<sr25519::Public>("Ferdie"),
        get_account_id_from_seed::<sr25519::Public>("Alice//stash"),
        get_account_id_from_seed::<sr25519::Public>("Bob//stash"),
        get_account_id_from_seed::<sr25519::Public>("Charlie//stash"),
        get_account_id_from_seed::<sr25519::Public>("Dave//stash"),
        get_account_id_from_seed::<sr25519::Public>("Eve//stash"),
        get_account_id_from_seed::<sr25519::Public>("Ferdie//stash"),
    ]
}

pub fn joy_chain_spec_properties() -> json::map::Map<String, json::Value> {
    let mut properties: json::map::Map<String, json::Value> = json::map::Map::new();
    properties.insert(
        String::from("tokenDecimals"),
        json::Value::Number(json::Number::from(10)),
    );
    properties.insert(
        String::from("tokenSymbol"),
        json::Value::String(String::from("JOY")),
    );
    properties.insert(
        String::from("ss58Format"),
        json::Value::Number(json::Number::from(JOY_ADDRESS_PREFIX)),
    );
    properties
}

#[allow(clippy::too_many_arguments)]
/// Helper function to create GenesisConfig for testing
pub fn testnet_genesis(
    initial_authorities: Vec<(
        AccountId,
        AccountId,
        GrandpaId,
        BabeId,
    )>,
    initial_nominators: Vec<AccountId>,
    root_key: AccountId,
    mut endowed_accounts: Vec<AccountId>,
    genesis_balances: Vec<(AccountId, Balance)>,
    vesting_accounts: Vec<(AccountId, BlockNumber, BlockNumber, Balance)>,
    content_cfg: ContentConfig,
    storage_cfg: StorageConfig,
) -> GenesisConfig {
    // endow all authorities and nominators.
    initial_authorities
        .iter()
        .map(|x| &x.0)
        .chain(initial_nominators.iter())
        .for_each(|x| {
            if !endowed_accounts.contains(x) {
                endowed_accounts.push(x.clone())
            }
        });

    // staking benchmakrs is not sensitive to actual value of min bonds so
    // accounts are not funded with sufficient funds and fail with InsufficientBond err
    // so for benchmarks we set min bond to zero.
    let min_nominator_bond = if cfg!(feature = "runtime-benchmarks") {
        0
    } else {
        MIN_NOMINATOR_BOND
    };
    let min_validator_bond = if cfg!(feature = "runtime-benchmarks") {
        0
    } else {
        MIN_VALIDATOR_BOND
    };

    GenesisConfig {
        system: SystemConfig {
            code: wasm_binary_unwrap().to_vec(),
        },
        balances: BalancesConfig {
            balances: endowed_accounts
                .iter()
                .cloned()
                .map(|x| (x, ENDOWMENT))
                .chain(
                    genesis_balances
                        .iter()
                        .map(|(account, balance)| (account.clone(), *balance)),
                )
                .collect(),
        },
        sudo: SudoConfig {
            key: Some(root_key),
        },
        babe: BabeConfig {
            authorities: vec![],
            epoch_config: Some(node_runtime::BABE_GENESIS_EPOCH_CONFIG),
        },
        grandpa: GrandpaConfig {
            authorities: vec![],
        },
        transaction_payment: TransactionPaymentConfig {},
        vesting: VestingConfig {
            vesting: vesting_accounts,
        },
        council: council_config::create_council_config(),
        forum: forum_config::empty(),
        content: content_cfg,
        storage: storage_cfg,
        referendum: council_config::create_referendum_config(),
        project_token: Default::default(),
        proposals_discussion: Default::default(),
    }
}

fn development_config_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![authority_keys_from_seed("Alice")],
        vec![
            get_account_id_from_seed::<sr25519::Public>("Bob"),
            get_account_id_from_seed::<sr25519::Public>("Charlie"),
        ],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        development_endowed_accounts(),
        vec![],
        vec![],
        content_config::testing_config(),
        storage_config::testing_config(),
    )
}

/// Development config (single validator Alice)
pub fn development_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Development",
        "dev",
        ChainType::Development,
        development_config_genesis,
        vec![],
        None,
        None,
        None,
        Some(joy_chain_spec_properties()),
        Default::default(),
    )
}

fn local_testnet_genesis() -> GenesisConfig {
    testnet_genesis(
        vec![
            authority_keys_from_seed("Alice"),
            authority_keys_from_seed("Bob"),
        ],
        vec![],
        get_account_id_from_seed::<sr25519::Public>("Alice"),
        development_endowed_accounts(),
        vec![],
        vec![],
        content_config::testing_config(),
        storage_config::testing_config(),
    )
}

/// Local testnet config (multivalidator Alice + Bob)
pub fn local_testnet_config() -> ChainSpec {
    ChainSpec::from_genesis(
        "Local Testnet",
        "local_testnet",
        ChainType::Local,
        local_testnet_genesis,
        vec![],
        None,
        None,
        None,
        Some(joy_chain_spec_properties()),
        Default::default(),
    )
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::service::{new_full_base, NewFullBase};
    use sc_service_test;
    use sp_runtime::BuildStorage;

    fn local_testnet_genesis_instant_single() -> GenesisConfig {
        testnet_genesis(
            vec![authority_keys_from_seed("Alice")],
            vec![],
            get_account_id_from_seed::<sr25519::Public>("Alice"),
            development_endowed_accounts(),
            vec![],
            vec![],
            content_config::testing_config(),
            storage_config::testing_config(),
        )
    }

    /// Local testnet config (single validator - Alice)
    pub fn integration_test_config_with_single_authority() -> ChainSpec {
        ChainSpec::from_genesis(
            "Integration Test",
            "test",
            ChainType::Development,
            local_testnet_genesis_instant_single,
            vec![],
            None,
            None,
            None,
            Some(joy_chain_spec_properties()),
            Default::default(),
        )
    }

    /// Local testnet config (multivalidator Alice + Bob)
    pub fn integration_test_config_with_two_authorities() -> ChainSpec {
        ChainSpec::from_genesis(
            "Integration Test",
            "test",
            ChainType::Development,
            local_testnet_genesis,
            vec![],
            None,
            None,
            None,
            Some(joy_chain_spec_properties()),
            Default::default(),
        )
    }

    #[test]
    #[ignore]
    fn test_connectivity() {
        sp_tracing::try_init_simple();

        sc_service_test::connectivity(integration_test_config_with_two_authorities(), |config| {
            let NewFullBase {
                task_manager,
                client,
                network,
                transaction_pool,
                ..
            } = new_full_base(config, false, |_, _| ())?;
            Ok(sc_service_test::TestNetComponents::new(
                task_manager,
                client,
                network,
                transaction_pool,
            ))
        });
    }

    #[test]
    fn test_create_development_chain_spec() {
        development_config().build_storage().unwrap();
    }

    #[test]
    fn test_create_local_testnet_chain_spec() {
        local_testnet_config().build_storage().unwrap();
    }
}
