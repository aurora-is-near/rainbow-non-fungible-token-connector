use near_contract_standards::non_fungible_token::metadata::{ TokenMetadata };
use near_sdk::test_utils::accounts;
use near_sdk::AccountId;
use near_sdk_sim::{call, deploy, init_simulator, to_yocto, view, ContractAccount, UserAccount};

use bridged_nft::BridgedNFTContract;

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    BRIDGED_NFT_WASM_BYTES => "../res/bridged_nft.wasm",
}

fn init() -> (UserAccount, ContractAccount<BridgedNFTContract>) {
    let mut genesis = near_sdk_sim::runtime::GenesisConfig::default();
    genesis.gas_limit = u64::MAX;
    genesis.gas_price = 0;
    let master_account = init_simulator(Some(genesis));

    let bridged_nft = deploy! {
        contract: BridgedNFTContract,
        contract_id: get_contract_id(),
        bytes: &BRIDGED_NFT_WASM_BYTES,
        signer_account: master_account
    };

    (master_account, bridged_nft)
}

#[test]
fn simulate() {
    let (master_account, bridged_nft) = init();
    const BRIDGE_TOKEN_INIT_BALANCE: near_sdk::Balance = 6_000_000_000_000_000_000_000_000;

    let alice = master_account.create_user(get_alice(), to_yocto("100"));
    let bob = master_account.create_user(get_bob(), to_yocto("100"));

    // init the bridged_nft contract.
    call!(master_account, bridged_nft.new()).assert_success();
    call!(
        master_account,
        bridged_nft.nft_mint("0".into(), alice.account_id(), token_metadata()),
        deposit = BRIDGE_TOKEN_INIT_BALANCE
    )
    .assert_success();

    call!(
        alice,
        bridged_nft.nft_approve("0".into(), bob.valid_account_id(), None),
        deposit = BRIDGE_TOKEN_INIT_BALANCE
    )
    .assert_success();

    let is_approved: bool =
        view!(bridged_nft.nft_is_approved("0".into(), bob.valid_account_id(), None)).unwrap_json();
    assert!(is_approved, "Token was not approved")
}

fn get_contract_id() -> String {
    String::from("bridged_nft")
}

fn get_alice() -> AccountId {
    accounts(1).into()
}

fn get_bob() -> AccountId {
    accounts(2).into()
}

fn token_metadata() -> TokenMetadata {
    TokenMetadata {
        title: Some("Mochi Rising".to_string()),
        description: Some("Limited edition canvas".to_string()),
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
