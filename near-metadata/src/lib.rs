use near_contract_standards::non_fungible_token::metadata::{
    NFTContractMetadata, NonFungibleTokenMetadataProvider,
};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{
    env, ext_contract, log, near_bindgen, AccountId, Balance, Gas, PanicOnDefault, Promise,
    PromiseResult,
};
use serde::Serialize;

const BALANCE: Balance = 0;
const GAS: Gas = 50_000_000_000_000;

#[derive(Debug, Eq, PartialEq, Serialize, BorshSerialize, BorshDeserialize)]
pub enum ResultType {
    Log {
        account_id: String,
        name: String,
        symbol: String,
        icon: Option<String>,
        base_uri: Option<String>,
    },
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct NearMetadata {}

#[ext_contract(ext_nft_metadata)]
pub trait NonFungibleTokenMetadata {
    fn nft_metadata(&self) -> NFTContractMetadata;
}

#[ext_contract(ext_self)]
pub trait ExtNearMetadata {
    fn finalize_nft_metadata(
        &self,
        #[callback] metadata: NFTContractMetadata,
        account_id: String,
    ) -> Promise;
}

#[near_bindgen]
impl NearMetadata {
    #[init]
    pub fn new() -> Self {
        assert!(!env::state_exists(), "Contract Already Initialized");
        NearMetadata {}
    }

    pub fn get_metadata_log(self, account_id: AccountId) -> Promise {
        ext_nft_metadata::nft_metadata(&account_id, BALANCE, GAS).then(
            ext_self::finalize_nft_metadata(
                account_id,
                &env::current_account_id(),
                BALANCE,
                GAS,
            ),
        )
    }

    pub fn finalize_nft_metadata(
        &self,
        #[callback] metadata: NFTContractMetadata,
        account_id: String,
    ) -> ResultType {
        self.check_promise_result(0, String::from("Failed to transfer the nft metadata."));
        ResultType::Log {
            account_id: account_id,
            name: metadata.name,
            symbol: metadata.symbol,
            icon: metadata.icon,
            base_uri: metadata.base_uri,
        }
    }

    fn check_promise_result(&self, index: u64, message: String) {
        let status = match env::promise_result(index) {
            PromiseResult::Successful(_) => true,
            _ => false,
        };
        assert!(status, "{}", message);
    }
}
