/**
* Factory for deploying NFT contracts linked to NFTs bridged from Ethereum
*/
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedSet;
use near_sdk::{env, ext_contract, near_bindgen, AccountId, Balance, Gas, PanicOnDefault, Promise, PublicKey};

use admin_controlled::{AdminControlled, Mask};

near_sdk::setup_alloc!();

use prover::*;
pub use prover::{get_eth_address, is_valid_eth_address, EthAddress, Proof};
pub use locked_event::LockedEvent;

pub mod prover;
mod locked_event;

/// Gas to call finalise method.
const FINISH_FINALISE_GAS: Gas = 50_000_000_000_000;

/// Gas to call mint method on bridge nft.
/// todo - this is for FT and needs to be updated for NFT
const MINT_GAS: Gas = 50_000_000_000_000;

const NO_DEPOSIT: Balance = 0;

/// Gas to call verify_log_entry on prover.
const VERIFY_LOG_ENTRY_GAS: Gas = 50_000_000_000_000;

const PAUSE_DEPLOY_TOKEN: Mask = 1 << 0;
const PAUSE_ETH_TO_NEAR_TRANSFER: Mask = 1 << 1;
const PAUSE_NEAR_TO_ETH_TRANSFER: Mask = 1 << 1;

#[derive(Debug, Eq, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum ResultType {
    Withdraw {
        token_id: String,
        token: EthAddress,
        recipient: EthAddress,
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

// #[ext_contract(ext_fungible_token)]
// pub trait NFT {
//    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
// }

#[ext_contract(ext_bridge_nft)]
pub trait ExtBridgedNFT {
    fn mint(&self, account_id: AccountId, token_id: String);
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
            bridge_token_storage_deposit_required:
            near_contract_standards::fungible_token::FungibleToken::new(b"t".to_vec())
                .account_storage_usage as Balance
                * env::storage_byte_cost(),
            // todo storage needs to be based on a nft - https://github.com/near/near-sdk-rs/blob/c968730425c22be796566fbfc490ac8e38eafcaf/near-contract-standards/src/fungible_token/core_impl.rs#L87
            paused: Mask::default(),
        }
    }

    /// Return all registered tokens
    pub fn get_tokens(&self) -> Vec<String> {
        self.tokens.iter().collect::<Vec<_>>()
    }

    /// todo docs
    // #[payable]
    // #[result_serializer(borsh)]
    // // todo: how much GAS is required to execute this method with sending the tokens back and ensure we have enough
    // pub fn migrate_to_ethereum(&mut self, eth_recipient: String) -> ResultType {
    //     // Predecessor must attach Near to migrate to ETH
    //     let attached_deposit = env::attached_deposit();
    //     if attached_deposit == 0 {
    //         env::panic(b"Attached deposit must be greater than zero");
    //     }
    //
    //     // If the method is paused or the eth recipient address is invalid, then we need to:
    //     //  1) Return the attached deposit
    //     //  2) Panic and tell the user why
    //     let eth_recipient_clone = eth_recipient.clone();
    //     if self.is_paused(PAUSE_MIGRATE_TO_ETH) || !is_valid_eth_address(eth_recipient_clone) {
    //         env::panic(b"Method is either paused or ETH address is invalid");
    //     }
    //
    //     ResultType::Withdraw {
    //         amount: attached_deposit,
    //         recipient: get_eth_address(eth_recipient),
    //     }
    // }

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
            FINISH_FINALISE_GAS, // todo + mint GAS
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

        ext_bridge_nft::mint(
            new_owner_id,
            token_id,
            &self.get_nft_token_account_id(token),
            env::attached_deposit() - required_deposit,
            MINT_GAS,
        );
    }

    // todo
    // #[payable]
    // pub fn deploy_bridge_token(&mut self, address: String) -> Promise {
    //     self.check_not_paused(PAUSE_DEPLOY_TOKEN);
    //     let address = address.to_lowercase();
    //     let _ = validate_eth_address(address.clone());
    //     assert!(
    //         !self.tokens.contains(&address),
    //         "BridgeToken contract already exists."
    //     );
    //     let initial_storage = env::storage_usage() as u128;
    //     self.tokens.insert(&address);
    //     let current_storage = env::storage_usage() as u128;
    //     assert!(
    //         env::attached_deposit()
    //             >= BRIDGE_TOKEN_INIT_BALANCE
    //             + STORAGE_PRICE_PER_BYTE * (current_storage - initial_storage),
    //         "Not enough attached deposit to complete bridge token creation"
    //     );
    //     let bridge_token_account_id = format!("{}.{}", address, env::current_account_id());
    //     Promise::new(bridge_token_account_id)
    //         .create_account()
    //         .transfer(BRIDGE_TOKEN_INIT_BALANCE)
    //         .add_full_access_key(self.owner_pk.clone())
    //         .deploy_contract(include_bytes!("../../res/bridge_token.wasm").to_vec())
    //         .function_call(
    //             b"new".to_vec(),
    //             b"{}".to_vec(),
    //             NO_DEPOSIT,
    //             BRIDGE_TOKEN_NEW,
    //         )
    // }

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

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
mod tests {
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::{testing_env, MockedBlockchain};

    use super::*;
    use near_sdk::env::sha256;
    use std::convert::TryInto;
    use std::panic;
    use uint::rustc_hex::{FromHex, ToHex};

    const UNPAUSE_ALL: Mask = 0;

    macro_rules! inner_set_env {
        ($builder:ident) => {
            $builder
        };

        ($builder:ident, $key:ident:$value:expr $(,$key_tail:ident:$value_tail:expr)*) => {
            {
               $builder.$key($value.try_into().unwrap());
               inner_set_env!($builder $(,$key_tail:$value_tail)*)
            }
        };
    }

    macro_rules! set_env {
        ($($key:ident:$value:expr),* $(,)?) => {
            let mut builder = VMContextBuilder::new();
            let mut builder = &mut builder;
            builder = inner_set_env!(builder, $($key: $value),*);
            testing_env!(builder.build());
        };
    }

    fn alice_near_account() -> AccountId {
        "alice.near".to_string()
    }
    fn prover_near_account() -> AccountId {
        "prover".to_string()
    }
    fn e_near_eth_address() -> String {
        "68a3637ba6e75c0f66b61a42639c4e9fcd3d4824".to_string()
    }
    fn alice_eth_address() -> String {
        "25ac31a08eba29067ba4637788d1dbfb893cebf1".to_string()
    }
    fn invalid_eth_address() -> String {
        "25Ac31A08EBA29067Ba4637788d1DbFB893cEBf".to_string()
    }

    /// Generate a valid ethereum address
    fn ethereum_address_from_id(id: u8) -> String {
        let mut buffer = vec![id];
        sha256(buffer.as_mut())
            .into_iter()
            .take(20)
            .collect::<Vec<_>>()
            .to_hex()
    }

    fn sample_proof() -> Proof {
        Proof {
            log_index: 0,
            log_entry_data: vec![],
            receipt_index: 0,
            receipt_data: vec![],
            header_data: vec![],
            proof: vec![],
        }
    }

    fn create_proof(e_near: String) -> Proof {
        let event_data = TransferToNearInitiatedEvent {
            e_near_address: e_near
                .from_hex::<Vec<_>>()
                .unwrap()
                .as_slice()
                .try_into()
                .unwrap(),
            sender: "00005474e89094c44da98b954eedeac495271d0f".to_string(),
            amount: 1000,
            recipient: "123".to_string(),
        };

        Proof {
            log_index: 0,
            log_entry_data: event_data.to_log_entry_data(),
            receipt_index: 0,
            receipt_data: vec![],
            header_data: vec![],
            proof: vec![],
        }
    }

    #[test]
    fn can_migrate_near_to_eth_with_valid_params() {
        set_env!(predecessor_account_id: alice_near_account());

        let mut contract = NearBridge::new(prover_near_account(), e_near_eth_address());

        // lets deposit 1 Near
        let deposit_amount = 1_000_000_000_000_000_000_000_000u128;
        set_env!(
            predecessor_account_id: alice_near_account(),
            attached_deposit: deposit_amount,
        );

        contract.migrate_to_ethereum(alice_eth_address());
    }

    #[test]
    #[should_panic]
    fn migrate_near_to_eth_panics_when_attached_deposit_is_zero() {
        set_env!(predecessor_account_id: alice_near_account());

        let mut contract = NearBridge::new(prover_near_account(), e_near_eth_address());

        contract.migrate_to_ethereum(alice_eth_address());
    }

    #[test]
    #[should_panic]
    fn migrate_near_to_eth_panics_when_contract_is_paused() {
        set_env!(predecessor_account_id: alice_near_account());

        let mut contract = NearBridge::new(prover_near_account(), e_near_eth_address());

        contract.set_paused(PAUSE_MIGRATE_TO_ETH);

        // lets deposit 1 Near
        let deposit_amount = 1_000_000_000_000_000_000_000_000u128;
        set_env!(
            predecessor_account_id: alice_near_account(),
            attached_deposit: deposit_amount,
        );

        contract.migrate_to_ethereum(alice_eth_address());
    }

    #[test]
    #[should_panic]
    fn migrate_near_to_eth_panics_when_eth_address_is_invalid() {
        set_env!(predecessor_account_id: alice_near_account());

        let mut contract = NearBridge::new(prover_near_account(), e_near_eth_address());

        contract.migrate_to_ethereum(invalid_eth_address());
    }

    #[test]
    #[should_panic]
    fn finalise_eth_to_near_transfer_panics_when_contract_is_paused() {
        set_env!(predecessor_account_id: alice_near_account());

        let mut contract = NearBridge::new(prover_near_account(), e_near_eth_address());

        contract.set_paused(PAUSE_ETH_TO_NEAR_TRANSFER);

        contract.finalise_eth_to_near_transfer(sample_proof());
    }

    #[test]
    #[should_panic]
    fn finalise_eth_to_near_transfer_panics_when_event_originates_from_wrong_contract() {
        set_env!(predecessor_account_id: alice_near_account());

        let mut contract = NearBridge::new(prover_near_account(), e_near_eth_address());

        contract.finalise_eth_to_near_transfer(create_proof(alice_eth_address()));
    }

    #[test]
    #[should_panic]
    fn finish_eth_to_near_transfer_panics_if_attached_deposit_is_not_sufficient_to_record_proof() {
        set_env!(predecessor_account_id: alice_near_account());

        let mut contract = NearBridge::new(prover_near_account(), e_near_eth_address());

        contract.finalise_eth_to_near_transfer(create_proof(e_near_eth_address()));
    }

    #[test]
    fn finalise_eth_to_near_transfer_works_with_valid_params() {
        set_env!(predecessor_account_id: alice_near_account());

        let mut contract = NearBridge::new(prover_near_account(), e_near_eth_address());

        // Alice deposit 1 Near to migrate to eth
        let deposit_amount = 1_000_000_000_000_000_000_000_000u128;
        set_env!(
            predecessor_account_id: alice_near_account(),
            attached_deposit: deposit_amount,
        );

        contract.migrate_to_ethereum(alice_eth_address());

        // todo adjust attached deposit down

        // Lets suppose Alice migrates back
        contract.finalise_eth_to_near_transfer(create_proof(e_near_eth_address()))

        // todo asserts i.e. that alice has received the 1 near back etc.
    }
}
