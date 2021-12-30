use admin_controlled::{AdminControlled, Mask};
use near_contract_standards::non_fungible_token::metadata::{NFTContractMetadata, TokenMetadata};
use near_sdk::collections::{UnorderedMap, UnorderedSet};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::ValidAccountId;
use near_sdk::{
    env, ext_contract, near_bindgen, AccountId, Balance, Gas, PanicOnDefault, Promise, PublicKey
};

pub use locked_event::EthLockedEvent;
pub use log_metadata_event::TokenMetadataEvent;
use prover::*;
pub use prover::{is_valid_eth_address, validate_eth_address, EthAddress, Proof};

mod locked_event;
pub mod prover;
mod log_metadata_event;

near_sdk::setup_alloc!();

const NO_DEPOSIT: Balance = 0;

/// Controller storage key.
const CONTROLLER_STORAGE_KEY: &[u8] = b"aCONTROLLER";

/// Metadata connector address storage key.
const METADATA_CONNECTOR_ETH_ADDRESS_STORAGE_KEY: &[u8] = b"aM_CONNECTOR";

/// Prefix used to store a map between tokens and timestamp `t`, where `t` stands for the
/// block on Ethereum where the metadata for given token was emitted.
/// The prefix is made specially short since it becomes more expensive with larger prefixes.
const TOKEN_TIMESTAMP_MAP_PREFIX: &[u8] = b"aTT";

/// Initial balance for the BridgeNFT contract to cover storage and related.
const BRIDGE_TOKEN_INIT_BALANCE: Balance = 4_500_000_000_000_000_000_000_000; // 3e24yN, 4.5N

/// Gas to initialize BridgeToken contract.
const BRIDGE_TOKEN_NEW: Gas = 50_000_000_000_000;

/// Gas to call mint method on bridge nft.
const MINT_GAS: Gas = 50_000_000_000_000;

/// Gas to call finalise method.
const FINISH_FINALISE_GAS: Gas = 50_000_000_000_000;

/// Amount of gas used by bridge token to set the metadata.
const SET_METADATA_GAS: Gas = 10_000_000_000_000;

/// Amount of gas used to update token owner.
const UPDATE_TOKEN_OWNER_GAS: Gas = 10_000_000_000_000;

/// Gas to call verify_log_entry on prover.
const VERIFY_LOG_ENTRY_GAS: Gas = 50_000_000_000_000;

/// Gas to call finish update_metadata method.
const FINISH_UPDATE_METADATA_GAS: Gas = 10_000_000_000_000;

const UNPAUSE_ALL: Mask = 0;
const PAUSE_DEPLOY_TOKEN: Mask = 1 << 0;
const PAUSE_ETH_TO_NEAR_TRANSFER: Mask = 1 << 1;

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
pub struct BridgeNFTFactory {
    /// The account of the prover that we can use to prove
    pub prover_account: ValidAccountId,
    /// Address of the Ethereum locker contract.
    pub locker_address: EthAddress,
    /// Set of created BridgeNFT contracts.
    pub tokens: UnorderedSet<String>,
    /// Hashes of the events that were already used.
    pub used_events: UnorderedSet<Vec<u8>>,
    /// Public key of the account deploying the factory.
    pub owner_pk: PublicKey,
    /// Account ID of the account deploying the factory.
    pub owner_id: AccountId,
    /// Balance required to register a new account in the BridgeNFT
    pub bridge_token_storage_deposit_required: Balance,
    /// Mask determining all paused functions
    paused: Mask,
}

#[ext_contract(ext_self)]
pub trait ExtBridgeNFTFactory {
    #[result_serializer(borsh)]
    fn finish_deposit(
        &mut self,
        #[callback]
        #[serializer(borsh)]
        verification_success: bool,
        #[serializer(borsh)] token: String,
        #[serializer(borsh)] new_owner_pk: AccountId,
        #[serializer(borsh)] token_id: String,
        #[serializer(borsh)] proof: Proof,
    ) -> Promise;

    #[result_serializer(borsh)]
    fn finish_updating_metadata(
        &mut self,
        #[callback]
        #[serializer(borsh)]
        verification_success: bool,
        #[serializer(borsh)] token: String,
        #[serializer(borsh)] name: String,
        #[serializer(borsh)] symbol: String,
        #[serializer(borsh)] timestamp: u64,
    ) -> Promise;
}

#[ext_contract(ext_bridge_nft)]
pub trait ExtBridgedNFT {
    fn nft_mint(&self, token_id: String, receiver_id: AccountId, token_metadata: TokenMetadata);
    fn set_metadata(&mut self, metadata: NFTContractMetadata);
    fn set_token_owner_account_id(&mut self, new_owner: ValidAccountId);
    fn is_controller(&mut self) -> bool;
    fn is_owner(&mut self) -> bool;
}

pub fn assert_self() {
    assert_eq!(env::predecessor_account_id(), env::current_account_id());
}

#[near_bindgen]
impl BridgeNFTFactory {
    #[init]
    pub fn new(prover_account: ValidAccountId, locker_address: String) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            prover_account,
            locker_address: validate_eth_address(locker_address),
            tokens: UnorderedSet::new(b"t".to_vec()),
            used_events: UnorderedSet::new(b"u".to_vec()),
            owner_pk: env::signer_account_pk(),
            owner_id: env::signer_account_id(),
            bridge_token_storage_deposit_required:
                near_contract_standards::fungible_token::FungibleToken::new(b"t".to_vec())
                    .account_storage_usage as Balance
                    * env::storage_byte_cost(),
            paused: Mask::default(),
        }
    }

    /// Return all registered tokens
    pub fn get_tokens(&self) -> Vec<String> {
        self.tokens.iter().collect::<Vec<_>>()
    }

    fn is_owner(&self) {
        assert_eq!(
            &env::predecessor_account_id(),
            &self.owner_id,
            "Owner's method"
        );
    }

    fn token_exists(&self, address: String) {
        assert!(
            self.tokens.contains(&address),
            "Bridged NFT contract doen't exist."
        );
    }

    /// Ethereum Metadata Connector. This is the address where the contract that emits metadata from tokens
    /// on ethereum is deployed. Address is encoded as hex.
    pub fn metadata_connector(&self) -> Option<String> {
        env::storage_read(METADATA_CONNECTOR_ETH_ADDRESS_STORAGE_KEY)
            .map(|value| String::from_utf8(value).expect("Invalid metadata connector address"))
    }

    pub fn set_metadata_connector(&mut self, metadata_connector: String) {
        assert!(self.controller_or_self());
        validate_eth_address(metadata_connector.clone());
        env::storage_write(
            METADATA_CONNECTOR_ETH_ADDRESS_STORAGE_KEY,
            metadata_connector.as_bytes(),
        );
    }

    /// Map between tokens and timestamp `t`, where `t` stands for the
    /// block on Ethereum where the metadata for given token was emitted.
    fn token_metadata_last_update(&mut self) -> UnorderedMap<String, u64> {
        UnorderedMap::new(TOKEN_TIMESTAMP_MAP_PREFIX.to_vec())
    }

    fn set_token_metadata_timestamp(&mut self, token: &String, timestamp: u64) -> Balance {
        let initial_storage = env::storage_usage();
        self.token_metadata_last_update().insert(&token, &timestamp);
        let current_storage = env::storage_usage();
        let required_deposit =
            Balance::from(current_storage - initial_storage) * env::storage_byte_cost();
        required_deposit 
    }

    #[payable]
    pub fn update_metadata(&mut self, #[serializer(borsh)] proof: Proof) -> Promise {
        let event = TokenMetadataEvent::from_log_entry_data(&proof.log_entry_data);

        let expected_metadata_connector = self.metadata_connector();

        assert_eq!(
            Some(hex::encode(event.metadata_connector)),
            expected_metadata_connector,
            "Event's address {} does not match contract address of this token {:?}",
            hex::encode(&event.metadata_connector),
            expected_metadata_connector,
        );

        assert!(
            self.tokens.contains(&event.token),
            "Bridge token for {} is not deployed yet",
            event.token
        );

        let last_timestamp = self
            .token_metadata_last_update()
            .get(&event.token)
            .unwrap_or_default();

        // Note that it is allowed for event.timestamp to be equal to last_timestamp.
        // This disallow replacing the metadata with old information, but allows replacing with information
        // from the same block. This is useful in case there is a failure in the cross-contract to the
        // bridge token with storage but timestamp in this contract is updated. In those cases the call
        // can be made again, to make the replacement effective.
        assert!(event.timestamp >= last_timestamp);

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
        .then(ext_self::finish_updating_metadata(
            event.token,
            event.name,
            event.symbol,
            event.timestamp,
            &env::current_account_id(),
            env::attached_deposit(),
            FINISH_UPDATE_METADATA_GAS + SET_METADATA_GAS,
        ))
    }

    /// Finish updating token metadata once the proof was successfully validated.
    /// Can only be called by the contract itself.
    #[payable]
    pub fn finish_updating_metadata(
        &mut self,
        #[callback]
        #[serializer(borsh)]
        verification_success: bool,
        #[serializer(borsh)] token: String,
        #[serializer(borsh)] name: String,
        #[serializer(borsh)] symbol: String,
        #[serializer(borsh)] timestamp: u64,
    ) -> Promise {
        assert_self();
        assert!(verification_success, "Failed to verify the proof");

        let required_deposit = self.set_token_metadata_timestamp(&token, timestamp);
        assert!(env::attached_deposit() >= required_deposit);

        env::log(
            format!(
                "Finish updating metadata. Name: {} Symbol: {:?} at: {:?}",
                name, symbol, timestamp
            )
            .as_bytes(),
        );

        let reference = None;
        let reference_hash = None;
        let base_uri = None;
        let icon = None;

        ext_bridge_nft::set_metadata(
            NFTContractMetadata{
                spec: String::from(""),
                name: name,
                symbol: symbol,
                icon: icon,
                base_uri: base_uri,
                reference: reference,
                reference_hash: reference_hash,
            },
            &self.get_bridge_nft_token_account_id(token),
            NO_DEPOSIT,
            SET_METADATA_GAS,
        )
    }

    /// Finalise the withdraw to eth from a sub NFT contract. Only this bridge can emit
    /// the execution outcome to be processed on the Eth side Caller must be <token_address>.
    /// <current_account_id>, where <token_address> exists in the `tokens`.
    // todo: how much GAS is required to execute this method
    #[result_serializer(borsh)]
    pub fn finish_withdraw_to_eth(
        &mut self,
        #[serializer(borsh)] token_id: String,
        #[serializer(borsh)] token_address: EthAddress,
        #[serializer(borsh)] recipient_address: EthAddress,
    ) -> ResultType {
        let caller = env::predecessor_account_id();
        let address: String = hex::encode(token_address).to_string();

        assert_eq!(
            caller,
            format!("{}.{}", &address, env::current_account_id()),
            "Only sub accounts of NFT Factory can call this method."
        );

        assert!(
            self.tokens.contains(&address),
            "Such Bridge NFT token does not exist."
        );

        ResultType::Withdraw {
            token: token_address,
            recipient: recipient_address,
            token_id,
        }
    }

    #[payable]
    pub fn finalise_eth_to_near_transfer(&mut self, #[serializer(borsh)] proof: Proof) -> Promise {
        self.check_not_paused(PAUSE_ETH_TO_NEAR_TRANSFER);

        let event = EthLockedEvent::from_log_entry_data(&proof.log_entry_data);
        assert_eq!(
            event.locker_address,
            self.locker_address,
            "Event's address {} does not match locker address of this token {}",
            hex::encode(&event.locker_address),
            hex::encode(&self.locker_address),
        );

        assert!(
            self.tokens.contains(&event.token),
            "Bridged NFT for {} is not deployed yet",
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
        .then(ext_self::finish_deposit(
            event.token,
            event.recipient,
            event.token_id,
            proof_1,
            &env::current_account_id(),
            env::attached_deposit(),
            FINISH_FINALISE_GAS + MINT_GAS,
        ))
    }

    /// Finish depositing once the proof was successfully validated. Can only be called by the contract
    /// itself.
    #[payable]
    pub fn finish_deposit(
        &mut self,
        #[callback]
        #[serializer(borsh)]
        verification_success: bool,
        #[serializer(borsh)] token: String,
        #[serializer(borsh)] new_owner_pk: AccountId,
        #[serializer(borsh)] token_id: String,
        #[serializer(borsh)] proof: Proof,
    ) -> Promise {
        near_sdk::assert_self();
        assert!(verification_success, "Failed to verify the proof");
        let required_deposit = self.record_proof(&proof);

        if env::attached_deposit() < required_deposit + self.bridge_token_storage_deposit_required {
            env::panic(b"Attached deposit is not sufficient to record proof");
        }
        let log = EthLockedEvent::from_log_entry_data(&proof.log_entry_data);

        ext_bridge_nft::nft_mint(
            token_id,
            new_owner_pk,
            TokenMetadata {
                title: None,
                description: None,
                media: Some(log.token_uri),
                media_hash: None,
                copies: None,
                issued_at: None,
                expires_at: None,
                starts_at: None,
                updated_at: None,
                extra: None,
                reference: None,
                reference_hash: None,
            },
            &self.get_bridge_nft_token_account_id(token),
            env::attached_deposit() - required_deposit,
            MINT_GAS,
        )
    }

    #[payable]
    pub fn deploy_bridged_token(&mut self, address: String) -> Promise {
        self.check_not_paused(PAUSE_DEPLOY_TOKEN);

        let address = address.to_lowercase();

        // check if the eth address is valid.
        is_valid_eth_address(address.clone());

        // check if the token was already deployed.
        assert!(
            !self.tokens.contains(&address),
            "Bridged NFT contract was already deployed."
        );

        let initial_storage = env::storage_usage() as u128;
        self.tokens.insert(&address);
        let current_storage = env::storage_usage() as u128;

        // check if the total deposit can cover the contract deployment.
        let min_required_deposit = BRIDGE_TOKEN_INIT_BALANCE
            + env::storage_byte_cost() * (current_storage - initial_storage);
        assert!(
            env::attached_deposit() >= min_required_deposit,
            "Not enough attached deposit to complete bridge nft token creation {}-{}",
            env::attached_deposit(),
            min_required_deposit
        );

        let bridge_token_account_id = self.get_bridge_nft_token_account_id(address);
        Promise::new(bridge_token_account_id)
            .create_account()
            .transfer(BRIDGE_TOKEN_INIT_BALANCE)
            .add_full_access_key(self.owner_pk.clone())
            .deploy_contract(include_bytes!("../../res/bridged_nft.wasm").to_vec())
            .function_call(
                b"new".to_vec(),
                b"{}".to_vec(),
                NO_DEPOSIT,
                BRIDGE_TOKEN_NEW,
            )
    }

    pub fn set_nft_contract_metadata(
        &mut self,
        address: String,
        metadata: NFTContractMetadata,
    ) -> Promise {
        self.is_owner();

        ext_bridge_nft::set_metadata(
            metadata,
            &self.get_bridge_nft_token_account_id(address),
            env::attached_deposit(),
            SET_METADATA_GAS,
        )
    }

    pub fn update_token_owner_account_id(&mut self, address: String, new_owner: ValidAccountId) {
        self.is_owner();

        ext_bridge_nft::set_token_owner_account_id(
            new_owner,
            &self.get_bridge_nft_token_account_id(address),
            env::attached_deposit(),
            UPDATE_TOKEN_OWNER_GAS,
        );
    }

    pub fn get_bridge_nft_token_account_id(&self, eth_address: String) -> AccountId {
        let address = eth_address.to_lowercase();
        is_valid_eth_address(address.clone());
        self.token_exists(address.clone());
        format!("{}.{}", address, env::current_account_id())
    }

     /// Checks whether the provided proof is already used
     pub fn is_used_proof(&self, #[serializer(borsh)] proof: Proof) -> bool {
        self.used_events.contains(&proof.get_key())
    }

    /// Record proof to make sure it is not re-used later for anther deposit.
    fn record_proof(&mut self, proof: &Proof) -> Balance {
        // TODO: Instead of sending the full proof (clone only relevant parts of the Proof)
        //       log_index / receipt_index / header_data
        near_sdk::assert_self();
        let initial_storage = env::storage_usage();
        let key = &proof.get_key();
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

    /// Factory Controller. Controller has extra privileges inside this contract.
    pub fn controller(&self) -> Option<AccountId> {
        env::storage_read(CONTROLLER_STORAGE_KEY)
            .map(|value| String::from_utf8(value).expect("Invalid controller account id"))
    }

    pub fn set_controller(&mut self, controller: AccountId) {
        assert!(self.controller_or_self(), "Not Controller: {}", self.controller_or_self());
        assert!(env::is_valid_account_id(controller.as_bytes()));
        env::storage_write(CONTROLLER_STORAGE_KEY, controller.as_bytes());
    }

    pub fn controller_or_self(&self) -> bool {
        let caller = env::predecessor_account_id();
        self.owner_id == caller
            || self
                .controller()
                .map(|controller| controller == caller)
                .unwrap_or(false)
    }
}

admin_controlled::impl_admin_controlled!(BridgeNFTFactory, paused);

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::env::sha256;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};
    use std::convert::TryInto;
    use std::panic;
    use uint::rustc_hex::{FromHex, ToHex};

    fn alice() -> ValidAccountId {
        accounts(0)
    }

    fn mock_prover() -> ValidAccountId {
        accounts(3)
    }

    fn bridge_token_factory() -> ValidAccountId {
        accounts(4)
    }

    fn mock_eth_locker_address() -> String {
        // no 0x needed
        String::from("57f1887a8bf19b14fc0df6fd9b2acc9af147ea85")
    }

    fn mock_eth_nft_address_one() -> String {
        // no 0x needed
        String::from("629a673a8242c2ac4b7b8c5d8735fbeac21a6205")
    }

    fn token_locker() -> String {
        "6b175474e89094c44da98b954eedeac495271d0f".to_string()
    }

    fn metadata_connector() -> String {
        "6b175474e89094c77da98b954eedeac495271d0f".to_string()
    }

    fn ethereum_address_from_id(id: u8) -> String {
        let mut buffer = vec![id];
        sha256(buffer.as_mut())
            .into_iter()
            .take(20)
            .collect::<Vec<_>>()
            .to_hex()
    }

    fn get_context(current_account_id: ValidAccountId, attached_deposit: Balance) -> VMContext {
        VMContextBuilder::new()
            .current_account_id(current_account_id)
            .attached_deposit(attached_deposit)
            .build()
    }

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

    fn mock_proof(locker: String, token: String, token_id: String) -> Proof {
        let event_data = EthLockedEvent {
            locker_address: locker
                .from_hex::<Vec<_>>()
                .unwrap()
                .as_slice()
                .try_into()
                .unwrap(),

            token,
            sender: "00005474e89094c44da98b954eedeac495271d0f".to_string(),
            token_id,
            recipient: "123".to_string(),
            token_uri: "".to_string(),
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
    fn success_create_factory_contract() {
        testing_env!(get_context(bridge_token_factory(), 30u128.pow(24)));
        let contract = BridgeNFTFactory::new(mock_prover(), mock_eth_locker_address());

        assert_eq!(
            contract.locker_address,
            validate_eth_address(mock_eth_locker_address()),
            "Locker address not valid"
        );
        assert_eq!(
            contract.prover_account,
            mock_prover(),
            "Prover account not valid"
        );
    }

    #[test]
    fn success_deploy_bridged_nft() {
        testing_env!(get_context(bridge_token_factory(), 30u128.pow(24)));
        let mut contract = BridgeNFTFactory::new(mock_prover(), mock_eth_locker_address());
        contract.deploy_bridged_token(token_locker());

        assert_eq!(
            contract.get_bridge_nft_token_account_id(token_locker()),
            format!("{}.{}", token_locker(), bridge_token_factory())
        )
    }

    #[test]
    #[should_panic(expected = "Invalid ETH address")]
    fn fail_deploy_bridged_nft_invalid_address() {
        testing_env!(get_context(bridge_token_factory(), 30u128.pow(24)));
        let mut contract = BridgeNFTFactory::new(mock_prover(), mock_eth_locker_address());
        let invalid_address = "".to_string();
        contract.deploy_bridged_token(invalid_address);
    }

    #[test]
    #[should_panic(expected = "Bridged NFT contract was already deployed.")]
    fn fail_deploy_bridged_nft_already_deployed() {
        testing_env!(get_context(bridge_token_factory(), 30u128.pow(24)));
        let mut contract = BridgeNFTFactory::new(mock_prover(), mock_eth_locker_address());
        contract.deploy_bridged_token(token_locker());
        contract.deploy_bridged_token(token_locker());
    }

    #[test]
    #[should_panic(
        expected = "Not enough attached deposit to complete bridge nft token creation 0-4501880000000000000000000"
    )]
    fn fail_deploy_bridged_nft_not_enough_attached_deposit() {
        testing_env!(get_context(bridge_token_factory(), 30u128.pow(24)));
        let mut contract = BridgeNFTFactory::new(mock_prover(), mock_eth_locker_address());
        testing_env!(get_context(bridge_token_factory(), 0u128.pow(24)));
        contract.deploy_bridged_token(token_locker());
    }
    #[test]
    fn success_finalise_eth_to_near_transfer() {
        testing_env!(get_context(bridge_token_factory(), 30u128.pow(24)));
        let mut contract = BridgeNFTFactory::new(mock_prover(), mock_eth_locker_address());
        contract.deploy_bridged_token(token_locker());

        let proof = mock_proof(
            mock_eth_locker_address(),
            token_locker().clone(),
            String::from("0"),
        );
        contract.finalise_eth_to_near_transfer(proof);
    }

    #[test]
    #[should_panic]
    fn fail_finalise_eth_to_near_transfer_invalid_locker() {
        // should_panic: "Event's address {} does not match locker address of this token {}"
        testing_env!(get_context(bridge_token_factory(), 30u128.pow(24)));
        let mut contract = BridgeNFTFactory::new(mock_prover(), mock_eth_locker_address());
        contract.deploy_bridged_token(token_locker());
        let invalid_locker = token_locker();
        let proof = mock_proof(invalid_locker, token_locker().clone(), String::from("0"));
        contract.finalise_eth_to_near_transfer(proof);
    }

    #[test]
    #[should_panic]
    fn fail_finalise_eth_to_near_transfer_bridged_nft_not_deployed() {
        // should_panic: "Bridged NFT for {} is not deployed yet"
        testing_env!(get_context(bridge_token_factory(), 30u128.pow(24)));
        let mut contract = BridgeNFTFactory::new(mock_prover(), mock_eth_locker_address());
        let proof = mock_proof(
            mock_eth_locker_address(),
            token_locker().clone(),
            String::from("0"),
        );
        contract.finalise_eth_to_near_transfer(proof);
    }

    #[test]
    #[should_panic(expected = "Method is private")]
    fn success_finish_deposit() {
        testing_env!(get_context(bridge_token_factory(), 30u128.pow(24)));
        let mut contract = BridgeNFTFactory::new(mock_prover(), mock_eth_locker_address());
        contract.deploy_bridged_token(token_locker());

        let proof = mock_proof(
            mock_eth_locker_address(),
            token_locker().clone(),
            String::from("0"),
        );
        let event = EthLockedEvent::from_log_entry_data(&proof.log_entry_data);
        contract.finish_deposit(true, event.token, event.recipient, event.token_id, proof);
    }
    
    #[test]
    fn only_admin_can_pause() {
        set_env!(predecessor_account_id: alice());
        let mut contract = BridgeNFTFactory::new(mock_prover(), mock_eth_locker_address());

        // Admin can pause
        set_env!(
            current_account_id: bridge_token_factory(),
            predecessor_account_id: bridge_token_factory(),
        );
        contract.set_paused(0b1111);

        // Alice can't pause
        set_env!(
            current_account_id: bridge_token_factory(),
            predecessor_account_id: alice(),
        );

        panic::catch_unwind(move || {
            contract.set_paused(0);
        })
        .unwrap_err();
    }

    #[test]
    fn deposit_paused() {
        set_env!(predecessor_account_id: alice());
        let mut contract = BridgeNFTFactory::new(mock_prover(), mock_eth_locker_address());

        set_env!(
            current_account_id: bridge_token_factory(),
            predecessor_account_id: alice(),
            attached_deposit: BRIDGE_TOKEN_INIT_BALANCE * 2
        );
        let erc20_address = ethereum_address_from_id(0);
        contract.deploy_bridged_token(erc20_address.clone());

        let proof = mock_proof(
            mock_eth_locker_address(),
            erc20_address.clone(),
            String::from("0"),
        );
        // Check it is possible to use deposit while the contract is NOT paused
        contract.finalise_eth_to_near_transfer(proof);

        // Pause deposit
        set_env!(
            current_account_id: bridge_token_factory(),
            predecessor_account_id: bridge_token_factory(),
            attached_deposit: BRIDGE_TOKEN_INIT_BALANCE * 2
        );
        contract.set_paused(PAUSE_ETH_TO_NEAR_TRANSFER);

        set_env!(
            current_account_id: bridge_token_factory(),
            predecessor_account_id: alice(),
            attached_deposit: BRIDGE_TOKEN_INIT_BALANCE * 2
        );

        let proof2 = mock_proof(
            mock_eth_locker_address(),
            erc20_address.clone(),
            String::from("0"),
        );
        // Check it is NOT possible to use deposit while the contract is paused
        panic::catch_unwind(move || {
            contract.finalise_eth_to_near_transfer(proof2);
        })
        .unwrap_err();
    }

    /// Check after all is paused deposit is not available
    #[test]
    fn all_paused() {
        set_env!(predecessor_account_id: alice());
        let mut contract = BridgeNFTFactory::new(mock_prover(), mock_eth_locker_address());

        set_env!(
            current_account_id: bridge_token_factory(),
            predecessor_account_id: alice(),
            attached_deposit: BRIDGE_TOKEN_INIT_BALANCE * 2
        );
        let erc20_address = ethereum_address_from_id(0);
        contract.deploy_bridged_token(erc20_address.clone());

        let proof = mock_proof(
            mock_eth_locker_address(),
            erc20_address.clone(),
            String::from("0"),
        );
        // Check it is possible to use deposit while the contract is NOT paused
        contract.finalise_eth_to_near_transfer(proof);

        // Pause everything
        set_env!(
            current_account_id: bridge_token_factory(),
            predecessor_account_id: bridge_token_factory(),
            attached_deposit: BRIDGE_TOKEN_INIT_BALANCE * 2
        );
        contract.set_paused(PAUSE_DEPLOY_TOKEN | PAUSE_ETH_TO_NEAR_TRANSFER);

        set_env!(
            current_account_id: bridge_token_factory(),
            predecessor_account_id: alice(),
            attached_deposit: BRIDGE_TOKEN_INIT_BALANCE * 2
        );
        let proof2 = mock_proof(
            mock_eth_locker_address(),
            erc20_address.clone(),
            String::from("0"),
        );
        // Check it is NOT possible to use deposit while the contract is paused
        panic::catch_unwind(move || {
            contract.finalise_eth_to_near_transfer(proof2);
        })
        .unwrap_err();
    }

    /// Check after all is paused and unpaused deposit works
    #[test]
    fn no_paused() {
        set_env!(predecessor_account_id: alice());
        let mut contract = BridgeNFTFactory::new(mock_prover(), mock_eth_locker_address());

        set_env!(
            current_account_id: bridge_token_factory(),
            predecessor_account_id: alice(),
            attached_deposit: BRIDGE_TOKEN_INIT_BALANCE * 2
        );
        let erc20_address = ethereum_address_from_id(0);
        contract.deploy_bridged_token(erc20_address.clone());

        let proof = mock_proof(
            mock_eth_locker_address(),
            erc20_address.clone(),
            String::from("0"),
        );

        // Check it is possible to use deposit while the contract is NOT paused
        contract.finalise_eth_to_near_transfer(proof);

        // Pause everything
        set_env!(
            current_account_id: bridge_token_factory(),
            predecessor_account_id: bridge_token_factory(),
            attached_deposit: BRIDGE_TOKEN_INIT_BALANCE * 2
        );

        contract.set_paused(PAUSE_DEPLOY_TOKEN | PAUSE_ETH_TO_NEAR_TRANSFER);
        contract.set_paused(UNPAUSE_ALL);

        set_env!(
            current_account_id: bridge_token_factory(),
            predecessor_account_id: alice(),
            attached_deposit: BRIDGE_TOKEN_INIT_BALANCE * 2
        );

        let proof2 = mock_proof(
            mock_eth_locker_address(),
            erc20_address.clone(),
            String::from("0"),
        );
        // Check the deposit works after pausing and unpausing everything
        contract.finalise_eth_to_near_transfer(proof2);
    }

    #[test]
    fn success_set_metadata_connector() {
        set_env!(
            current_account_id: bridge_token_factory(),
            predecessor_account_id: bridge_token_factory(),
            signer_account_id: bridge_token_factory()
        );
        let mut contract = BridgeNFTFactory::new(mock_prover(), mock_eth_locker_address());        
        contract.set_metadata_connector(metadata_connector());
        assert_eq!(contract.metadata_connector().unwrap(), metadata_connector(), "metadata_connector not valid!")
    }
}
