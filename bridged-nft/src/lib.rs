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

/// Gas to call finish withdraw method on factory.
const FINISH_WITHDRAW_GAS: Gas = 50_000_000_000_000;

const PAUSE_WITHDRAW: Mask = 1 << 0;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct BridgeNFT {
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
        #[serializer(borsh)] recipient: AccountId,
    ) -> Promise;
}

#[near_bindgen]
impl BridgeNFT {
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
        // Only owner can change the metadata
        assert!(self.controller_or_self());
        self.metadata = LazyOption::new(StorageKey::Metadata.try_to_vec().unwrap(), Some(&metadata))
    }

    pub fn set_owner_account_id(&mut self, new_owner: ValidAccountId) {
        // Only owner can change the owner account_id
        assert!(self.controller_or_self());

        self.tokens.owner_id = new_owner.into();
    }

    #[payable]
    pub fn nft_mint(
        &mut self,
        token_id: TokenId,
        receiver_id: AccountId,
        token_metadata: TokenMetadata,
    ) -> Token {
        assert_eq!(
            env::predecessor_account_id(),
            self.controller,
            "Only controller can call mint"
        );
        self.tokens.mint(
            token_id,
            receiver_id.try_into().unwrap(),
            Some(token_metadata),
        )
    }

    pub fn account_storage_usage(&self) -> StorageUsage {
        self.tokens.extra_storage_in_bytes_per_token
    }

    /// Return true if the caller is either controller or self
    pub fn controller_or_self(&self) -> bool {
        let caller = env::predecessor_account_id();
        caller == self.controller || caller == env::current_account_id()
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

        let predecessor_account_id = env::predecessor_account_id();
        if let Some(approvals_by_id) = &mut self.tokens.approvals_by_id {
            let is_authorized: bool = approvals_by_id.contains_key(&predecessor_account_id)
                || &predecessor_account_id == &self.tokens.owner_by_id.get(&token_id).unwrap();

            if !is_authorized {
                env::panic(b"Unauthorized");
            }
        }
        // burn the token
        self.tokens.owner_by_id.remove(&token_id);
        if let Some(token_metadata_by_id) = &mut self.tokens.token_metadata_by_id {
            token_metadata_by_id.remove(&token_id);
        }

        if let Some(tokens_per_owner) = &mut self.tokens.tokens_per_owner {
            let mut tokens_set = tokens_per_owner.get(&predecessor_account_id).unwrap();
            tokens_set.remove(&token_id);
            tokens_per_owner.insert(&predecessor_account_id, &tokens_set);
        }

        // call the nft factory to finish the withdrawal to eth
        ext_bridge_nft_factory::finish_withdraw_to_eth(
            token_id,
            recipient,
            &self.controller,
            NO_DEPOSIT,
            FINISH_WITHDRAW_GAS,
        )
    }
}

near_contract_standards::impl_non_fungible_token_core!(BridgeNFT, tokens);
near_contract_standards::impl_non_fungible_token_approval!(BridgeNFT, tokens);
near_contract_standards::impl_non_fungible_token_enumeration!(BridgeNFT, tokens);
admin_controlled::impl_admin_controlled!(BridgeNFT, paused);

#[near_bindgen]
impl NonFungibleTokenMetadataProvider for BridgeNFT {
    fn nft_metadata(&self) -> NFTContractMetadata {
        self.metadata.get().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::serde::export::TryFrom;
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, VMContext};

    fn alice() -> ValidAccountId {
        ValidAccountId::try_from("alice.near").unwrap()
    }
    fn bob() -> ValidAccountId {
        ValidAccountId::try_from("bob.near").unwrap()
    }
    fn nft() -> ValidAccountId {
        ValidAccountId::try_from("nft.near").unwrap()
    }

    fn get_context(predecessor_account_id: ValidAccountId, attached_deposit: Balance) -> VMContext {
        VMContext {
            current_account_id: "alice_near".to_string(),
            signer_account_id: "bob_near".to_string(),
            signer_account_pk: vec![0, 1, 2],
            predecessor_account_id: predecessor_account_id.to_string(),
            input: vec![],
            block_index: 0,
            block_timestamp: 0,
            account_balance: 1000 * 10u128.pow(24),
            account_locked_balance: 0,
            storage_usage: 10u64.pow(6),
            attached_deposit,
            prepaid_gas: 2 * 10u64.pow(14),
            random_seed: vec![0, 1, 2],
            is_view: false,
            output_data_receivers: vec![],
            epoch_height: 19,
        }
    }

    fn helper_contract_metadata() -> NFTContractMetadata {
        NFTContractMetadata {
            spec: "".to_string(),
            name: "".to_string(),
            symbol: "".to_string(),
            icon: None,
            base_uri: None,
            reference: None,
            reference_hash: None,
        }
    }

    fn helper_token_metadata() -> TokenMetadata {
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

    fn helper_mint(recipient: ValidAccountId) -> (BridgeNFT, VMContext) {
        let context = get_context(nft(), 11u128.pow(24));
        testing_env!(context.clone());
        let mut contract = BridgeNFT::new();
        contract.nft_mint(
            "0".to_string(),
            recipient.to_string(),
            helper_token_metadata(),
        );
        (contract, context)
    }

    #[test]
    fn basic_mint_from_owner() {
        let (contract, _) = helper_mint(nft());
        let token_info = &contract.nft_token("0".to_string());
        assert!(
            token_info.is_some(),
            "Expected to find newly minted token, got None."
        );
    }

    #[test]
    #[should_panic(expected = "Only controller can call mint")]
    fn failed_mint_from_non_contract_owner() {
        let context = get_context(alice(), 8460000000000000000000);
        testing_env!(context);
        let mut contract = BridgeNFT::new();

        let context = get_context(bob(), 8460000000000000000000);
        testing_env!(context);
        contract.nft_mint("0".to_string(), nft().to_string(), helper_token_metadata());
    }

    #[test]
    #[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
    fn failed_withdraw_no_owner() {
        let (mut contract, _) = helper_mint(nft());

        let context = get_context(bob(), 8460000000000000000000);
        testing_env!(context);

        contract.withdraw("0".to_string(), "0xfaaf".to_string());
    }

    #[test]
    #[should_panic(expected = "Requires attached deposit of exactly 1 yoctoNEAR")]
    fn failed_withdraw_needs_one_yocto() {
        let (mut contract, _) = helper_mint(nft());
        contract.withdraw("0".to_string(), "0xfaaf".to_string());
    }

    #[test]
    #[should_panic(expected = "Token not found")]
    fn failed_withdraw_when_token_not_found() {
        let (mut contract, mut context) = helper_mint(bob());

        context.predecessor_account_id = alice().to_string();
        context.attached_deposit = 1;
        testing_env!(context.clone());

        contract.withdraw("3".to_string(), "0xfaaf".to_string());
    }

    #[test]
    fn success_withdraw_from_token_owner() {
        let (mut contract, mut context) = helper_mint(bob());

        context.attached_deposit = 1;
        context.predecessor_account_id = bob().to_string();
        testing_env!(context.clone());

        contract.withdraw("0".to_string(), "0xfaaf".to_string());
    }

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn failed_withdraw_from_non_token_owner() {
        let (mut contract, mut context) = helper_mint(bob());

        context.predecessor_account_id = alice().to_string();
        context.attached_deposit = 1;
        testing_env!(context.clone());

        contract.withdraw("0".to_string(), "0xfaaf".to_string());
    }

    #[test]
    #[should_panic(expected = "Sender not approved")]
    fn failed_transfer_using_unauthorized_approver() {
        let (mut contract, mut context) = helper_mint(nft());
        contract.nft_approve(
            "0".to_string(),
            ValidAccountId::try_from(alice()).unwrap(),
            None,
        );
        // Bob tries to transfer when only alice should be allowed to
        context.predecessor_account_id = bob().to_string();
        context.attached_deposit = 1;
        testing_env!(context.clone());
        contract.nft_transfer(
            ValidAccountId::try_from(bob()).unwrap(),
            "0".to_string(),
            Some(u64::from(1u64)),
            Some("I am trying to hack you.".to_string()),
        );
    }
}
