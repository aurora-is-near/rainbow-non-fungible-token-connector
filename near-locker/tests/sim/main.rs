use near_contract_standards::non_fungible_token::metadata::TokenMetadata;
use near_contract_standards::non_fungible_token::{Token, TokenId};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::ValidAccountId;
use near_sdk::serde::Deserialize;
use near_sdk::test_utils::accounts;
use near_sdk::{AccountId, Balance};
use near_sdk_sim::{call, deploy, init_simulator, to_yocto, view, ContractAccount, UserAccount};

use serde_json::json;
use std::convert::TryInto;
use uint::rustc_hex::FromHex;

use mock_nft::MockNFTContract;
use mock_prover::MockProverContract;
use near_locker::LockerContract;

#[derive(Debug, Eq, PartialEq, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum ResultType {
    Unlock {
        token_account_id: String,
        token_id: String,
        recipient: String,
    },
    Lock {
        recipient: EthAddress,
        token_account_id: AccountId,
        token_id: String,
        token_uri: String,
    },
}

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    NEAR_LOCKER_WASM_BYTES => "../res/near_locker.wasm",
    PROVER_WASM_BYTES => "../res/mock_prover.wasm",
    NFT_WASM_BYTES => "../res/mock_nft.wasm",
}

fn init() -> (
    UserAccount,
    ContractAccount<MockNFTContract>,
    ContractAccount<LockerContract>,
    ContractAccount<MockProverContract>,
) {
    let mut genesis = near_sdk_sim::runtime::GenesisConfig::default();
    genesis.gas_limit = u64::MAX;
    genesis.gas_price = 0;
    let master_account = init_simulator(Some(genesis));

    let mock_prover = deploy! {
        contract: MockProverContract,
        contract_id: get_prover(),
        bytes: &PROVER_WASM_BYTES,
        signer_account: master_account
    };

    let mock_nft = deploy! {
        contract: MockNFTContract,
        contract_id: get_nft(),
        bytes: &NFT_WASM_BYTES,
        signer_account: master_account
    };

    let near_locker = deploy! {
        contract: LockerContract,
        contract_id: get_locker(),
        bytes: &NEAR_LOCKER_WASM_BYTES,
        signer_account: master_account
    };

    // init mock nft contract
    call!(master_account, mock_nft.new()).assert_success();

    // init mock prover contract
    call!(master_account, mock_prover.new()).assert_success();

    // init near locker contract
    call!(
        master_account,
        near_locker.new(mock_eth_factory_address(), mock_prover.valid_account_id())
    )
    .assert_success();

    (master_account, mock_nft, near_locker, mock_prover)
}

#[test]
fn simulate_lock() {
    let (master_account, mock_nft, near_locker, mock_prover) = init();
    let alice = master_account.create_user(get_alice().into(), to_yocto("100"));
    let bob = master_account.create_user(get_bob().into(), to_yocto("100"));
    const DEPOSIT: Balance = 6_000_000_000_000_000_000_000_000;
    // mint nft for alice
    call!(
        master_account,
        mock_nft.nft_mint(TokenId::from("1"), get_alice()),
        deposit = DEPOSIT
    )
    .assert_success();

    // mint nft for alice
    call!(
        master_account,
        mock_nft.nft_mint(TokenId::from("2"), get_alice()),
        deposit = DEPOSIT
    )
    .assert_success();

    call!(
        alice,
        mock_nft.nft_transfer(get_bob(), TokenId::from("2"), None, None),
        deposit = 1
    )
    .assert_success();

    // check if the nft exists
    let token: Token = view!(mock_nft.nft_token(TokenId::from("2"))).unwrap_json();
    assert_eq!(token.token_id, TokenId::from("2"), "Invalid token id");
    assert_eq!(token.owner_id, get_bob().to_string(), "Invalid token owner");

    // check if the nft exists
    let mut token: Token = view!(mock_nft.nft_token(TokenId::from("1"))).unwrap_json();
    assert_eq!(token.token_id, TokenId::from("1"), "Invalid token id");
    assert_eq!(
        token.owner_id,
        get_alice().to_string(),
        "Invalid token owner"
    );

    // alice approve the nft for the locker contract
    call!(
        alice,
        mock_nft.nft_approve(TokenId::from("1"), get_locker(), None),
        deposit = DEPOSIT
    )
    .assert_success();

    // check if the nft was approved to the locker.
    token = view!(mock_nft.nft_token(TokenId::from("1"))).unwrap_json();
    assert_eq!(
        token
            .approved_account_ids
            .unwrap()
            .get(&get_locker().to_string())
            .unwrap(),
        &(1 as u64),
        "Token not approved"
    );

    // alice call lock function
    let locker: String = get_locker().to_string();
    let result: ResultType = call_json!(
        alice,
        locker.lock(
            {
                "token_account_id": get_nft(),
                "token_id": TokenId::from("1"),
                "eth_recipient": mock_eth_factory_address()
            }
        ),
        deposit = 1
    )
    .unwrap_json();

    // check the result of the lock.
    let expected: ResultType = ResultType::Lock {
        token_account_id: get_nft(),
        token_id: TokenId::from("1"),
        recipient: validate_eth_address(mock_eth_factory_address()),
        token_uri: String::from("aaa"),
    };

    assert_eq!(result, expected, "Invalid lock result type");

    // check if the locker is the new owner.
    token = view!(mock_nft.nft_token(TokenId::from("1"))).unwrap_json();
    assert_eq!(
        token.owner_id,
        get_locker().to_string(),
        "Invalid token owner"
    );

    // call unlock nft.
    call!(
        master_account,
        near_locker.unlock(mock_proof(
            mock_eth_factory_address(),
            mock_eth_factory_address(),
            String::from("1"),
        )),
        deposit = 1
    )
    .assert_success();

    // check if the locker is the new owner.
    token = view!(mock_nft.nft_token(TokenId::from("1"))).unwrap_json();
    assert_eq!(
        token.owner_id,
        get_alice().to_string(),
        "Invalid token owner"
    );
}

fn get_alice() -> ValidAccountId {
    accounts(0)
}

fn get_bob() -> ValidAccountId {
    accounts(1)
}

fn get_locker() -> ValidAccountId {
    accounts(2)
}

fn get_prover() -> AccountId {
    String::from("prover")
}

fn get_nft() -> AccountId {
    String::from("nft")
}

fn default_nft_metadata() -> TokenMetadata {
    TokenMetadata {
        title: None,
        description: None,
        media: None,
        media_hash: None,
        copies: None,
        issued_at: None,
        expires_at: None,
        starts_at: None,
        updated_at: None,
        extra: None,
        reference: None,
        reference_hash: None,
    }
}

fn mock_eth_factory_address() -> AccountId {
    // no 0x needed
    String::from("629a673a8242c2ac4b7b8c5d8735fbeac21a6205")
}

fn mock_proof(withdrawer: String, token: String, token_id: String) -> near_locker::Proof {
    let event_data = near_locker::EthWithdrawEvent {
        withdraw_address: withdrawer
            .from_hex::<Vec<_>>()
            .unwrap()
            .as_slice()
            .try_into()
            .unwrap(),
        token_address: "00005474e89094c44da98b954eedeac495271d0f".to_string(),
        token_account_id: get_nft().to_string(),
        sender: "00005474e89094c44da98b954eedeac495271d0f".to_string(),
        token_id,
        recipient: get_alice().to_string(),
    };

    near_locker::Proof {
        log_index: 0,
        log_entry_data: event_data.to_log_entry_data(),
        receipt_index: 0,
        receipt_data: vec![],
        header_data: vec![],
        proof: vec![],
    }
}

pub type EthAddress = [u8; 20];

pub fn validate_eth_address(address: String) -> EthAddress {
    let data = hex::decode(address).expect("address should beg a valid hex string.");
    assert_eq!(data.len(), 20, "address should be 20 bytes long");
    let mut result = [0u8; 20];
    result.copy_from_slice(&data);
    result
}

#[macro_export]
macro_rules! call_json {
    ($signer:expr, $contract:ident, $method:ident, $arg:tt, $gas:expr, $deposit:expr) => {
        $signer.call(
            $contract.clone(),
            stringify!($method),
            json!($arg).to_string().into_bytes().as_ref(),
            $gas,
            $deposit,
        )
    };
    ($signer:expr, $contract:ident.$method:ident($arg:tt), $gas:expr, $deposit:expr) => {
        call_json!($signer, $contract, $method, $arg, $gas, $deposit)
    };
    ($signer:expr, $contract:ident.$method:ident($arg:tt)) => {
        call_json!(
            $signer,
            $contract,
            $method,
            $arg,
            near_sdk_sim::DEFAULT_GAS,
            near_sdk_sim::STORAGE_AMOUNT
        )
    };
    ($signer:expr, $contract:ident.$method:ident($arg:tt), deposit=$deposit:expr) => {
        call_json!(
            $signer,
            $contract,
            $method,
            $arg,
            near_sdk_sim::DEFAULT_GAS,
            $deposit
        )
    };
}
