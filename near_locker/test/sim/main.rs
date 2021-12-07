use near_contract_standards::non_fungible_token::Token;
use near_sdk::json_types::ValidAccountId;
use near_sdk::test_utils::accounts;
use near_sdk::AccountId;
use near_sdk_sim::{call, deploy, init_simulator, to_yocto, view, ContractAccount, UserAccount};

use serde_json::json;
use std::convert::TryInto;
use uint::rustc_hex::FromHex;

use mock_prover::MockProverContract;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    PROVER_WASM_BYTES => "../res/mock_prover.wasm",
}

fn init() -> (
    UserAccount,
    ContractAccount<MockProverContract>,
) {
    let mut genesis = near_sdk_sim::runtime::GenesisConfig::default();
    genesis.gas_limit = u64::MAX;
    genesis.gas_price = 0;
    let master_account = init_simulator(Some(genesis));

    let prover = deploy! {
        contract: MockProverContract,
        contract_id: get_prover(),
        bytes: &PROVER_WASM_BYTES,
        signer_account: master_account
    };

    (master_account, prover)
}
