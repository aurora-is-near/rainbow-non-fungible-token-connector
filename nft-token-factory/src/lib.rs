use admin_controlled::{AdminControlled, Mask};
use near_contract_standards::non_fungible_token::metadata::{NFTContractMetadata, TokenMetadata};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedSet;
use near_sdk::json_types::ValidAccountId;
use near_sdk::{
    env, ext_contract, near_bindgen, AccountId, Balance, Gas, PanicOnDefault, Promise, PublicKey,
};

pub use locked_event::EthLockedEvent;
use prover::*;
pub use prover::{is_valid_eth_address, validate_eth_address, EthAddress, Proof};

mod locked_event;
pub mod prover;

near_sdk::setup_alloc!();

const NO_DEPOSIT: Balance = 0;

/// Initial balance for the BridgeNFT contract to cover storage and related.
const BRIDGE_TOKEN_INIT_BALANCE: Balance = 4_500_000_000_000_000_000_000_000; // 3e24yN, 4.5N

/// Gas to initialize BridgeNFT contract.
const BRIDGE_TOKEN_NEW: Gas = 50_000_000_000_000;

/// Gas to call mint method on bridge nft.
const MINT_GAS: Gas = 50_000_000_000_000; // todo correct this value to mainnet value

/// Gas to call finalise method.
const FINISH_FINALISE_GAS: Gas = 50_000_000_000_000; // todo correct this value to mainnet value

const SET_METADATA_GAS: Gas = 10_000_000_000_000; // todo correct this value to mainnet value
const UPDATE_TOKEN_OWNER_GAS: Gas = 10_000_000_000_000; // todo correct this value to mainnet value

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
}

#[ext_contract(ext_bridge_nft)]
pub trait ExtBridgedNFT {
    fn nft_mint(&self, token_id: String, receiver_id: AccountId, token_metadata: TokenMetadata);
    fn set_metadata(&mut self, metadata: NFTContractMetadata);
    fn set_token_owner_account_id(&mut self, new_owner: ValidAccountId);
    fn is_controller(&mut self) -> bool;
    fn is_owner(&mut self) -> bool;
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
            bridge_token_storage_deposit_required: env::storage_byte_cost() * 237, // this can be verified by calling storage_cost_per_nft() in bridge_nft.wasm when deployed
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
        // let owner_pk: ValidAccountId = new_owner_pk.try_into().unwrap();
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
            &self.get_nft_token_account_id(token),
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

        let bridge_token_account_id = format!("{}.{}", address, env::current_account_id());
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
            &self.get_nft_token_account_id(address),
            env::attached_deposit(),
            SET_METADATA_GAS,
        )
    }

    pub fn update_token_owner_account_id(&mut self, address: String, new_owner: ValidAccountId) {
        self.is_owner();

        ext_bridge_nft::set_token_owner_account_id(
            new_owner,
            &self.get_nft_token_account_id(address),
            env::attached_deposit(),
            UPDATE_TOKEN_OWNER_GAS,
        );
    }

    pub fn add_full_key_to_bridge_nft_account(&mut self, eth_address: String, key: PublicKey) {
        self.is_owner();
        let address = eth_address.to_lowercase();
        is_valid_eth_address(address.clone());
        self.token_exists(address.clone());

        let bridge_token_account_id = format!("{}.{}", address, env::current_account_id());
        Promise::new(bridge_token_account_id).add_full_access_key(key);
    }

    pub fn delete_full_key_from_bridge_nft_account(&mut self, eth_address: String, key: PublicKey) {
        self.is_owner();
        let address = eth_address.to_lowercase();
        is_valid_eth_address(address.clone());
        self.token_exists(address.clone());

        let bridge_token_account_id = format!("{}.{}", address, env::current_account_id());
        Promise::new(bridge_token_account_id).delete_key(key);
    }

    pub fn get_nft_token_account_id(&self, eth_address: String) -> AccountId {
        let address = eth_address.to_lowercase();
        is_valid_eth_address(address.clone());
        self.token_exists(address.clone());
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

admin_controlled::impl_admin_controlled!(BridgeNFTFactory, paused);

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::env::sha256;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};
    use std::convert::TryInto;
    use uint::rustc_hex::{FromHex, ToHex};

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
            contract.get_nft_token_account_id(token_locker()),
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
}
