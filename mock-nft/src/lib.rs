use near_contract_standards::non_fungible_token::metadata::TokenMetadata;
use near_contract_standards::non_fungible_token::{NonFungibleToken, Token, TokenId};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider,
};
use near_sdk::json_types::ValidAccountId;
use near_sdk::{
    env, near_bindgen, AccountId, BorshStorageKey, PanicOnDefault, Promise, PromiseOrValue,
};
use std::convert::TryInto;

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct MockNFT {
    tokens: NonFungibleToken,
}

#[derive(BorshSerialize, BorshStorageKey)]
enum StorageKey {
    NonFungibleToken,
    Metadata,
    TokenMetadata,
    Enumeration,
    Approval,
}

#[near_bindgen]
impl MockNFT {
    #[init]
    pub fn new() -> Self {
        MockNFT {
            tokens: NonFungibleToken::new(
                StorageKey::NonFungibleToken,
                env::predecessor_account_id().try_into().unwrap(),
                Some(StorageKey::TokenMetadata),
                Some(StorageKey::Enumeration),
                Some(StorageKey::Approval),
            ),
        }
    }

    #[payable]
    pub fn nft_mint(&mut self, token_id: TokenId, receiver_id: ValidAccountId) -> Token {
        self.tokens.mint(
            token_id,
            receiver_id.try_into().unwrap(),
            Some(TokenMetadata {
                title: None,
                description: None,
                media: Some(String::from("aaa")),
                media_hash: None,
                copies: None,
                issued_at: None,
                expires_at: None,
                starts_at: None,
                updated_at: None,
                extra: None,
                reference: None,
                reference_hash: None,
            }),
        )
    }
}

#[near_bindgen]
impl NonFungibleTokenMetadataProvider for MockNFT {
    fn nft_metadata(&self) -> NFTContractMetadata {
        NFTContractMetadata{
            spec: String::from("aaa"),
            name: String::from("bbb"),
            symbol: String::from("ccc"),
            icon: None,
            base_uri: None,
            reference: None,
            reference_hash: None,
        }
    }
}

near_contract_standards::impl_non_fungible_token_core!(MockNFT, tokens);
near_contract_standards::impl_non_fungible_token_approval!(MockNFT, tokens);
near_contract_standards::impl_non_fungible_token_enumeration!(MockNFT, tokens);
