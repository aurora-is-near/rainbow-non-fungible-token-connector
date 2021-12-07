use admin_controlled::Mask;
use near_contract_standards::non_fungible_token::core::NonFungibleToken;
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider, TokenMetadata,
};
use near_contract_standards::non_fungible_token::{Token, TokenId};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LazyOption;
use near_sdk::json_types::ValidAccountId;
use near_sdk::{
    assert_one_yocto, env, ext_contract, near_bindgen, AccountId, Balance, BorshStorageKey, Gas,
    PanicOnDefault, Promise, PromiseOrValue, StorageUsage,
};
use std::convert::TryInto;

near_sdk::setup_alloc!();

const NO_DEPOSIT: Balance = 0;

pub type EthAddress = [u8; 20];

/// Gas to call finish withdraw method on factory.
const FINISH_WITHDRAW_GAS: Gas = 50_000_000_000_000;

const PAUSE_WITHDRAW: Mask = 1 << 0;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct BridgedNFT {
    controller: AccountId,
    tokens: NonFungibleToken,
    metadata: LazyOption<NFTContractMetadata>,
    paused: Mask,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    NonFungibleToken,
    Metadata,
    TokenMetadata,
    Enumeration,
    Approval,
}

#[ext_contract(ext_bridge_nft_factory)]
pub trait ExtBridgeNFTFactory {
    #[result_serializer(borsh)]
    fn finish_withdraw_to_eth(
        &self,
        #[serializer(borsh)] token_id: String,
        #[serializer(borsh)] token_address: EthAddress,
        #[serializer(borsh)] recipient_address: EthAddress,
    ) -> Promise;
}

#[near_bindgen]
impl BridgedNFT {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            controller: env::predecessor_account_id(),
            tokens: NonFungibleToken::new(
                StorageKey::NonFungibleToken,
                env::predecessor_account_id().try_into().unwrap(),
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
            metadata: LazyOption::new(StorageKey::Metadata, None),
            paused: Mask::default(),
        }
    }

    pub fn set_metadata(&mut self, metadata: NFTContractMetadata) {
        self.is_controller();
        self.metadata = LazyOption::new(StorageKey::Metadata.try_to_vec().unwrap(), Some(&metadata))
    }

    pub fn set_token_owner_account_id(&mut self, new_owner: ValidAccountId) {
        self.is_token_owner();
        self.tokens.owner_id = new_owner.into();
    }

    pub fn set_controller(&mut self, new_controller: ValidAccountId) {
        self.is_controller();
        self.controller = new_controller.into();
    }

    #[payable]
    pub fn nft_mint(
        &mut self,
        token_id: TokenId,
        receiver_id: AccountId,
        token_metadata: TokenMetadata,
    ) -> Token {
        self.is_controller();
        self.tokens.mint(
            token_id,
            receiver_id.try_into().unwrap(),
            Some(token_metadata),
        )
    }

    pub fn account_storage_usage(&self) -> StorageUsage {
        env::storage_usage()
    }

    pub fn is_controller(&self) {
        assert_eq!(
            env::predecessor_account_id(),
            self.controller,
            "Caller isn't the contract controller"
        );
    }

    pub fn is_token_owner(&self) {
        assert_eq!(
            self.tokens.owner_id,
            env::predecessor_account_id(),
            "Caller isn't the token owner"
        );
    }

    #[payable]
    pub fn withdraw(&mut self, token_id: String, recipient: String) -> Promise {
        self.check_not_paused(PAUSE_WITHDRAW);
        // Not returning as its going to cost too much GAS
        assert_one_yocto();

        // check the token exists and that the caller is the owner
        self.tokens
            .owner_by_id
            .get(&token_id)
            .expect("Token not found");

        let account = env::current_account_id();
        let parts: Vec<&str> = account.split(".").collect();

        let predecessor_account_id = env::predecessor_account_id();
        if let Some(approvals_by_id) = &mut self.tokens.approvals_by_id {
            let is_authorized: bool = approvals_by_id.contains_key(&predecessor_account_id)
                || &predecessor_account_id == &self.tokens.owner_by_id.get(&token_id).unwrap();
            assert!(is_authorized, "Unauthorized");
        }

        let token_address = validate_eth_address(parts[0].to_string());
        let recipient_address = validate_eth_address(recipient);

        // burn the token and delete metadata
        self.tokens.owner_by_id.remove(&token_id);
        if let Some(token_metadata_by_id) = &mut self.tokens.token_metadata_by_id {
            token_metadata_by_id.remove(&token_id);
        }

        if let Some(tokens_per_owner) = &mut self.tokens.tokens_per_owner {
            let mut tokens_set = tokens_per_owner.get(&predecessor_account_id).unwrap();
            tokens_set.remove(&token_id);
        }

        // call the nft factory to finish the withdrawal to eth
        ext_bridge_nft_factory::finish_withdraw_to_eth(
            token_id,
            token_address,
            recipient_address,
            &self.controller,
            NO_DEPOSIT,
            FINISH_WITHDRAW_GAS,
        )
    }
}

pub fn validate_eth_address(address: String) -> EthAddress {
    let data = hex::decode(address).expect("address should beg a valid hex string.");
    assert_eq!(data.len(), 20, "address should be 20 bytes long");
    let mut result = [0u8; 20];
    result.copy_from_slice(&data);
    result
}

near_contract_standards::impl_non_fungible_token_core!(BridgedNFT, tokens);
near_contract_standards::impl_non_fungible_token_approval!(BridgedNFT, tokens);
near_contract_standards::impl_non_fungible_token_enumeration!(BridgedNFT, tokens);
admin_controlled::impl_admin_controlled!(BridgedNFT, paused);

#[near_bindgen]
impl NonFungibleTokenMetadataProvider for BridgedNFT {
    fn nft_metadata(&self) -> NFTContractMetadata {
        self.metadata.get().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};
    use std::convert::TryFrom;

    fn alice() -> ValidAccountId {
        accounts(0)
    }
    fn bob() -> ValidAccountId {
        accounts(1)
    }
    fn nft() -> ValidAccountId {
        accounts(2)
    }

    fn get_context(predecessor_account_id: ValidAccountId, attached_deposit: Balance) -> VMContext {
        VMContextBuilder::new()
            .predecessor_account_id(predecessor_account_id)
            .attached_deposit(attached_deposit)
            .build()
    }
    fn contract_metadata() -> NFTContractMetadata {
        NFTContractMetadata {
            spec: "Mock NFT".to_string(),
            name: "Mock NFT".to_string(),
            symbol: "MNFT".to_string(),
            icon: None,
            base_uri: None,
            reference: None,
            reference_hash: None,
        }
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

    fn mock_eth_address() -> String {
        String::from("57f1887a8bf19b14fc0df6fd9b2acc9af147ea85")
    }

    #[test]
    fn success_mint_nft_from_owner() {
        testing_env!(get_context(nft(), 11u128.pow(24)).clone());

        let mut contract = BridgedNFT::new();
        contract.nft_mint("0".into(), alice().into(), token_metadata());

        let token_info = contract.nft_token("0".to_string());
        assert!(
            token_info.is_some(),
            "Expected to find newly minted token, got None."
        );

        assert_eq!(
            token_info.unwrap().owner_id,
            alice().to_string(),
            "Owner is not valid"
        );
    }

    #[test]
    #[should_panic(expected = "Caller isn't the contract controller")]
    fn fail_mint_nft_from_owner() {
        testing_env!(get_context(nft(), 11u128.pow(24)).clone());
        let mut contract = BridgedNFT::new();

        testing_env!(get_context(alice(), 11u128.pow(24)).clone());
        contract.nft_mint("0".into(), alice().into(), token_metadata());
    }

    #[test]
    fn success_set_contract_metadata() {
        testing_env!(get_context(nft(), 11u128.pow(24)).clone());
        let mut contract = BridgedNFT::new();

        let meta = contract_metadata();
        contract.set_metadata(meta);

        let meta = contract.metadata.get().unwrap();
        assert_eq!(meta.name, meta.name, "Invalid contract metadata");
    }

    #[test]
    #[should_panic(expected = "Caller isn't the contract controller")]
    fn fail_set_contract_metadata() {
        testing_env!(get_context(nft(), 11u128.pow(24)).clone());
        let mut contract = BridgedNFT::new();
        testing_env!(get_context(bob(), 11u128.pow(24)).clone());
        contract.set_metadata(contract_metadata());
    }

    #[test]
    fn success_set_token_owner_id() {
        testing_env!(get_context(nft(), 11u128.pow(24)).clone());
        let mut contract = BridgedNFT::new();
        contract.set_token_owner_account_id(bob());
        assert_eq!(
            contract.tokens.owner_id,
            bob().to_string(),
            "Invalid controller"
        );
    }

    #[test]
    #[should_panic(expected = "Caller isn't the token owner")]
    fn fail_set_token_owner_account_id() {
        testing_env!(get_context(nft(), 11u128.pow(24)).clone());
        let mut contract = BridgedNFT::new();
        testing_env!(get_context(bob(), 11u128.pow(24)).clone());
        contract.set_token_owner_account_id(bob());
    }

    #[test]
    fn success_set_controller() {
        testing_env!(get_context(nft(), 11u128.pow(24)).clone());
        let mut contract = BridgedNFT::new();
        contract.set_controller(bob());
        assert_eq!(contract.controller, bob().to_string(), "Invalid controller");
    }

    #[test]
    #[should_panic(expected = "Caller isn't the contract controller")]
    fn fail_set_controller() {
        testing_env!(get_context(nft(), 11u128.pow(24)).clone());
        let mut contract = BridgedNFT::new();
        testing_env!(get_context(bob(), 11u128.pow(24)).clone());
        contract.set_controller(bob());
    }

    #[test]
    #[should_panic(expected = "address should beg a valid hex string.: OddLength")]
    fn fail_withdraw_invalid_eth_address() {
        testing_env!(get_context(nft(), 11u128.pow(24)).clone());
        let mut contract = BridgedNFT::new();
        contract.nft_mint("0".into(), alice().into(), token_metadata());
        contract.nft_mint("1".into(), alice().into(), token_metadata());

        // deposit 1 yocoto near
        testing_env!(get_context(alice(), 1u128.pow(1)).clone());
        contract.withdraw("0".into(), mock_eth_address());
        assert_eq!(
            contract.tokens.nft_token("0".into()),
            None,
            "Invalid controller"
        );
    }

    #[test]
    #[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
    fn fail_withdraw_1_yocto_near() {
        testing_env!(get_context(nft(), 11u128.pow(24)).clone());
        let mut contract = BridgedNFT::new();
        testing_env!(get_context(nft(), 0u128.pow(1)).clone());
        contract.withdraw("0".into(), mock_eth_address());
    }

    #[test]
    #[should_panic(expected = "Token not found")]
    fn fail_withdraw_token_not_found() {
        testing_env!(get_context(nft(), 11u128.pow(24)).clone());
        let mut contract = BridgedNFT::new();
        testing_env!(get_context(nft(), 1u128.pow(1)).clone());
        contract.withdraw("0".into(), mock_eth_address());
    }

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn fail_withdraw_token_not_owner() {
        testing_env!(get_context(nft(), 11u128.pow(24)).clone());
        let mut contract = BridgedNFT::new();
        contract.nft_mint("0".into(), alice().into(), token_metadata());
        testing_env!(get_context(nft(), 1u128.pow(1)).clone());
        contract.withdraw("0".into(), mock_eth_address());
    }
}
