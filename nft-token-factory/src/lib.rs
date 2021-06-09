/**
* Factory for deploying NFT contracts linked to NFTs bridged from Ethereum
*/
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::collections::UnorderedSet;
use near_sdk::{env, ext_contract, near_bindgen, AccountId, Balance, Gas, PanicOnDefault, Promise, PublicKey};
use near_sdk::json_types::{Base64VecU8, U64};

use admin_controlled::{AdminControlled, Mask};

near_sdk::setup_alloc!();

use prover::*;
pub use prover::{get_eth_address, is_valid_eth_address, EthAddress, Proof};
pub use locked_event::LockedEvent;

pub mod prover;
mod locked_event;

/// Gas to call finalise method.
const FINISH_FINALISE_GAS: Gas = 50_000_000_000_000; // todo correct this value to mainnet value
const BRIDGE_TOKEN_NEW: Gas = 50_000_000_000_000; // todo correct this value to mainnet value
const BRIDGE_TOKEN_INIT_BALANCE: Balance = 50_000_000_000_000; // todo correct this value to mainnet value

/// Gas to call mint method on bridge nft.
const MINT_GAS: Gas = 50_000_000_000_000; // todo correct this value to mainnet value

const NO_DEPOSIT: Balance = 0;

/// Gas to call verify_log_entry on prover.
const VERIFY_LOG_ENTRY_GAS: Gas = 50_000_000_000_000;

const PAUSE_DEPLOY_TOKEN: Mask = 1 << 0;
const PAUSE_ETH_TO_NEAR_TRANSFER: Mask = 1 << 1;
const PAUSE_NEAR_TO_ETH_TRANSFER: Mask = 1 << 1;

#[derive(Debug, Eq, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum ResultType {
    Withdraw {
        token: EthAddress,
        recipient: EthAddress,
        token_id: String,
    },
    Lock {
        token: String,
        token_id: String,
        recipient: EthAddress,
    },
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct NFTFactory {
    /// The account of the prover that we can use to prove
    pub prover_account: AccountId,

    /// Address of the Ethereum locker contract.
    pub locker_address: EthAddress,

    /// Set of created BridgeNFT contracts.
    pub tokens: UnorderedSet<String>,

    /// Hashes of the events that were already used.
    pub used_events: UnorderedSet<Vec<u8>>,

    /// Public key of the account deploying the factory.
    pub owner_pk: PublicKey,

    /// Balance required to register a new account in the BridgeNFT
    pub bridge_token_storage_deposit_required: Balance,

    /// Mask determining all paused functions
    paused: Mask,
}

#[ext_contract(ext_self)]
pub trait ExtNFTFactory {
    #[result_serializer(borsh)]
    fn finish_eth_to_near_transfer(
        &mut self,
        #[callback]
        #[serializer(borsh)]
        verification_success: bool,
        #[serializer(borsh)] token: String,
        #[serializer(borsh)] new_owner_id: AccountId,
        #[serializer(borsh)] token_id: String,
        #[serializer(borsh)] proof: Proof,
    ) -> Promise;
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
pub struct TokenMetadata {
    pub title: Option<String>, // ex. "Arch Nemesis: Mail Carrier" or "Parcel #5055"
    pub description: Option<String>, // free-form description
    pub media: Option<String>, // URL to associated media, preferably to decentralized, content-addressed storage
    pub media_hash: Option<Base64VecU8>, // Base64-encoded sha256 hash of content referenced by the `media` field. Required if `media` is included.
    pub copies: Option<U64>, // number of copies of this set of metadata in existence when token was minted.
    pub issued_at: Option<String>, // ISO 8601 datetime when token was issued or minted
    pub expires_at: Option<String>, // ISO 8601 datetime when token expires
    pub starts_at: Option<String>, // ISO 8601 datetime when token starts being valid
    pub updated_at: Option<String>, // ISO 8601 datetime when token was last updated
    pub extra: Option<String>, // anything extra the NFT wants to store on-chain. Can be stringified JSON.
    pub reference: Option<String>, // URL to an off-chain JSON file with more info.
    pub reference_hash: Option<Base64VecU8>, // Base64-encoded sha256 hash of JSON from reference field. Required if `reference` is included.
}

#[ext_contract(ext_bridge_nft)]
pub trait ExtBridgedNFT {
    fn nft_mint(&self, token_id: String, recipient: AccountId, metadata: TokenMetadata);
}

#[near_bindgen]
impl NFTFactory {
    #[init]
    pub fn new(prover_account: AccountId, locker_address: String) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            prover_account,
            locker_address: get_eth_address(locker_address),
            tokens: UnorderedSet::new(b"t".to_vec()),
            used_events: UnorderedSet::new(b"u".to_vec()),
            owner_pk: env::signer_account_pk(),
            bridge_token_storage_deposit_required: env::storage_byte_cost() * 237, // this can be verified by calling storage_cost_per_nft() in bridge_nft.wasm when deployed
            paused: Mask::default(),
        }
    }

    /// Return all registered tokens
    pub fn get_tokens(&self) -> Vec<String> {
        self.tokens.iter().collect::<Vec<_>>()
    }

    /// Finalise the withdraw to eth from a sub NFT contract. Only this bridge can emit the execution outcome to be processed on the Eth side
    /// Caller must be <token_address>.<current_account_id>, where <token_address> exists in the `tokens`.
    // todo: how much GAS is required to execute this method
    #[result_serializer(borsh)]
    pub fn finish_withdraw_to_eth(
        &mut self,
        #[serializer(borsh)] token_id: String,
        #[serializer(borsh)] recipient: String,
    ) -> ResultType {
        self.check_not_paused(PAUSE_NEAR_TO_ETH_TRANSFER);

        let token = env::predecessor_account_id();

        let parts: Vec<&str> = token.split(".").collect();
        assert_eq!(
            token,
            format!("{}.{}", parts[0], env::current_account_id()),
            "Only sub accounts of NFT Factory can call this method."
        );

        assert!(
            self.tokens.contains(&parts[0].to_string()),
            "Such Bridge NFT token does not exist."
        );

        let token_address = get_eth_address(parts[0].to_string());
        let recipient_address = get_eth_address(recipient);

        ResultType::Withdraw {
            token: token_address,
            recipient: recipient_address,
            token_id,
        }
    }

    #[payable]
    pub fn finalise_eth_to_near_transfer(&mut self, #[serializer(borsh)] proof: Proof) {
        self.check_not_paused(PAUSE_ETH_TO_NEAR_TRANSFER);

        let event = LockedEvent::from_log_entry_data(&proof.log_entry_data);
        assert_eq!(
            event.locker_address,
            self.locker_address,
            "Event's address {} does not match locker address of this token {}",
            hex::encode(&event.locker_address),
            hex::encode(&self.locker_address),
        );

        assert!(
            self.tokens.contains(&event.token),
            "Bridge NFT for {} is not deployed yet",
            event.token
        );

        let proof_1 = proof.clone();

        ext_prover::verify_log_entry(
            proof.log_index,
            proof.log_entry_data,
            proof.receipt_index,
            proof.receipt_data,
            proof.header_data,
            proof.proof,
            false, // Do not skip bridge call. This is only used for development and diagnostics.
            &self.prover_account,
            NO_DEPOSIT,
            VERIFY_LOG_ENTRY_GAS,
        )
        .then(ext_self::finish_eth_to_near_transfer(
            event.token,
            event.recipient,
            event.token_id,
            proof_1,
            &env::current_account_id(),
            env::attached_deposit(),
            FINISH_FINALISE_GAS + MINT_GAS,
        ));
    }

    /// Finish depositing once the proof was successfully validated. Can only be called by the contract
    /// itself.
    #[payable]
    pub fn finish_eth_to_near_transfer(
        &mut self,
        #[callback]
        #[serializer(borsh)]
        verification_success: bool,
        #[serializer(borsh)] token: String,
        #[serializer(borsh)] new_owner_id: AccountId,
        #[serializer(borsh)] token_id: String,
        #[serializer(borsh)] proof: Proof,
    ) {
        near_sdk::assert_self();
        assert!(verification_success, "Failed to verify the proof");

        let required_deposit = self.record_proof(&proof);
        if env::attached_deposit() < required_deposit + self.bridge_token_storage_deposit_required {
            env::panic(b"Attached deposit is not sufficient to record proof");
        }

        ext_bridge_nft::nft_mint(
            token_id,
            new_owner_id,
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
            }, // todo we could store token URI at a minimum from an ETH event or have an update method
            &self.get_nft_token_account_id(token),
            env::attached_deposit() - required_deposit,
            MINT_GAS,
        );
    }

    #[payable]
    pub fn deploy_bridge_token(&mut self, address: String) -> Promise {
        self.check_not_paused(PAUSE_DEPLOY_TOKEN);

        let address = address.to_lowercase();

        assert!(
            is_valid_eth_address(address.clone()),
            "Invalid ETH address"
        );

        assert!(
            !self.tokens.contains(&address),
            "Bridge NFT contract already exists."
        );

        let initial_storage = env::storage_usage() as u128;
        self.tokens.insert(&address);
        let current_storage = env::storage_usage() as u128;

        assert!(
            env::attached_deposit()
                >= BRIDGE_TOKEN_INIT_BALANCE
                + env::storage_byte_cost() * (current_storage - initial_storage),
            "Not enough attached deposit to complete bridge nft token creation"
        );

        let bridge_token_account_id = format!("{}.{}", address, env::current_account_id());
        Promise::new(bridge_token_account_id)
            .create_account()
            .transfer(BRIDGE_TOKEN_INIT_BALANCE)
            .add_full_access_key(self.owner_pk.clone())
            .deploy_contract(include_bytes!("../../res/bridge_nft.wasm").to_vec())
            .function_call(
                b"new".to_vec(),
                b"{}".to_vec(),
                NO_DEPOSIT,
                BRIDGE_TOKEN_NEW,
            )
    }

    pub fn get_nft_token_account_id(&self, address: String) -> AccountId {
        let address = address.to_lowercase();

        assert!(
            is_valid_eth_address(address.clone()),
            "Invalid ETH address"
        );

        assert!(
            self.tokens.contains(&address),
            "NFTToken with such address does not exist."
        );

        format!("{}.{}", address, env::current_account_id())
    }

    /// Record proof to make sure it is not re-used later for anther deposit.
    fn record_proof(&mut self, proof: &Proof) -> Balance {
        // TODO: Instead of sending the full proof (clone only relevant parts of the Proof)
        //       log_index / receipt_index / header_data
        near_sdk::assert_self();
        let initial_storage = env::storage_usage();
        let mut data = proof.log_index.try_to_vec().unwrap();
        data.extend(proof.receipt_index.try_to_vec().unwrap());
        data.extend(proof.header_data.clone());
        let key = env::sha256(&data);
        assert!(
            !self.used_events.contains(&key),
            "Event cannot be reused for depositing."
        );
        self.used_events.insert(&key);
        let current_storage = env::storage_usage();

        let required_deposit =
            Balance::from(current_storage - initial_storage) * env::storage_byte_cost();

        required_deposit
    }
}

admin_controlled::impl_admin_controlled!(NFTFactory, paused);

// todo - add in tests
