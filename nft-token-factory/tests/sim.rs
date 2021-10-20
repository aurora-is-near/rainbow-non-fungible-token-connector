use near_contract_standards::non_fungible_token::Token;
use near_sdk_sim::{
    call, deploy, init_simulator, to_yocto, view, ContractAccount, UserAccount,
};
use serde_json::json;
use std::convert::TryInto;
use uint::rustc_hex::{FromHex};

use mock_prover::MockProverContract;
use nft_token_factory::BridgeNFTFactoryContract;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    NFT_FACTORY_WASM_BYTES => "../res/nft_token_factory.wasm",
    PROVER_WASM_BYTES => "../res/mock_prover.wasm",
}

fn init() -> (
    UserAccount,
    ContractAccount<BridgeNFTFactoryContract>,
    ContractAccount<MockProverContract>,
) {
    let mut genesis = near_sdk_sim::runtime::GenesisConfig::default();
    genesis.gas_limit = u64::MAX;
    genesis.gas_price = 0;
    let master_account = init_simulator(Some(genesis));

    let factory = deploy! {
        contract: BridgeNFTFactoryContract,
        contract_id: get_factory(),
        bytes: &NFT_FACTORY_WASM_BYTES,
        signer_account: master_account
    };

    let prover = deploy! {
        contract: MockProverContract,
        contract_id: get_prover(),
        bytes: &PROVER_WASM_BYTES,
        signer_account: master_account
    };

    (master_account, factory, prover)
}

#[test]
fn simulate() {
    let (master_account, factory, _) = init();
    let alice = master_account.create_user(get_alice(), to_yocto("100"));
    const BRIDGE_TOKEN_INIT_BALANCE: near_sdk::Balance = 6_000_000_000_000_000_000_000_000;
    call!(
        master_account,
        factory.new(get_prover(), mock_eth_locker_address())
    )
    .assert_success();

    call!(
        master_account,
        factory.deploy_bridge_token(mock_eth_nft_address_one()),
        deposit = BRIDGE_TOKEN_INIT_BALANCE
    )
    .assert_success();

    let proof = create_proof(
        mock_eth_locker_address(),
        mock_eth_nft_address_one(),
        String::from("0"),
    );

    call!(
        master_account,
        factory.finalise_eth_to_near_transfer(proof),
        deposit = BRIDGE_TOKEN_INIT_BALANCE
    )
    .assert_success();

    let nft_account_id: String =
        view!(factory.get_nft_token_account_id(mock_eth_nft_address_one())).unwrap_json();

    assert_eq!(
        nft_account_id,
        format!("{}.{}", mock_eth_nft_address_one(), get_factory())
    );

    let token: Token =
        call_json!(master_account, nft_account_id.nft_token({"token_id": "0"})).unwrap_json();
    assert!(token.token_id == String::from("0"), "Invalid token id");
    assert!(token.owner_id == get_alice(), "Invalid token owner");

    call_json!(
        alice,
        nft_account_id.withdraw({"token_id": token.token_id, "recipient":mock_eth_nft_address_one()}),
        deposit = 1
    )
    .assert_success();

    let token: Option<Token> =
        call_json!(master_account, nft_account_id.nft_token({"token_id": "0"})).unwrap_json();
    assert!(token == None, "Token should be None");
}

fn create_proof(locker: String, token: String, token_id: String) -> nft_token_factory::Proof {
    let event_data = nft_token_factory::EthLockedEvent {
        locker_address: locker
            .from_hex::<Vec<_>>()
            .unwrap()
            .as_slice()
            .try_into()
            .unwrap(),

        token,
        sender: "00005474e89094c44da98b954eedeac495271d0f".to_string(),
        token_id,
        recipient: get_alice(),
        token_uri: "".to_string(),
    };

    nft_token_factory::Proof {
        log_index: 0,
        log_entry_data: event_data.to_log_entry_data(),
        receipt_index: 0,
        receipt_data: vec![],
        header_data: vec![],
        proof: vec![],
    }
}

fn get_alice() -> String {
    "123".to_string()
}

fn mock_eth_nft_address_one() -> String {
    // no 0x needed
    String::from("629a673a8242c2ac4b7b8c5d8735fbeac21a6205")
}

fn get_factory() -> String {
    String::from("bridge")
}
fn get_prover() -> String {
    String::from("prover")
}

fn mock_eth_locker_address() -> String {
    // no 0x needed
    String::from("57f1887a8bf19b14fc0df6fd9b2acc9af147ea85")
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
