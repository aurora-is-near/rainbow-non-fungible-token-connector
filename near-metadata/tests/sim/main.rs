use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata,
};
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
use near_metadata::NearMetadataContract;

#[derive(Debug, Eq, PartialEq, Deserialize, BorshSerialize, BorshDeserialize)]
pub enum ResultType {
    Log {
        name: String,
        symbol: String,
        icon: Option<String>,
        uri: Option<String>,
    },
}

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    NEAR_METADATA_WASM_BYTES => "../res/near_metadata.wasm",
    NFT_WASM_BYTES => "../res/mock_nft.wasm",
}

fn init() -> (
    UserAccount,
    ContractAccount<MockNFTContract>,
    ContractAccount<NearMetadataContract>,
) {
    let mut genesis = near_sdk_sim::runtime::GenesisConfig::default();
    genesis.gas_limit = u64::MAX;
    genesis.gas_price = 0;
    let master_account = init_simulator(Some(genesis));

    let mock_nft = deploy! {
        contract: MockNFTContract,
        contract_id: get_nft(),
        bytes: &NFT_WASM_BYTES,
        signer_account: master_account
    };

    let near_metadata = deploy! {
        contract: NearMetadataContract,
        contract_id: get_metadata(),
        bytes: &NEAR_METADATA_WASM_BYTES,
        signer_account: master_account
    };

    // init mock nft contract
    call!(master_account, mock_nft.new()).assert_success();

    // init mock prover contract
    call!(master_account, near_metadata.new()).assert_success();

    (master_account, mock_nft, near_metadata)
}

#[test]
fn simulate_metadata_transfer() {
    let (master_account, mock_nft, near_metadata) = init();
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
    let mut token: Token = view!(mock_nft.nft_token(TokenId::from("2"))).unwrap_json();
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
    let metadata_acc: String = get_metadata().to_string();

    call!(
        master_account,
        near_metadata.get_metadata_log(metadata_acc),
        deposit = 1
    )
    .assert_success();
}

fn get_alice() -> ValidAccountId {
    accounts(0)
}

fn get_bob() -> ValidAccountId {
    accounts(1)
}

fn get_nft() -> AccountId {
    String::from("nft")
}

fn get_metadata() -> AccountId {
    String::from("metadata")
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
