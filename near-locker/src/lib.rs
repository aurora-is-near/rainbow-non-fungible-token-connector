use admin_controlled::{AdminControlled, Mask};
use near_contract_standards::non_fungible_token::TokenId;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::ValidAccountId;
use near_sdk::{
    assert_one_yocto, env, ext_contract, near_bindgen, AccountId, Balance, Gas, PanicOnDefault,
    Promise, PromiseResult,
};
use serde::Serialize;
mod withdraw_event;
pub use withdraw_event::*;

mod prover;
pub use prover::{ext_prover, Proof};

pub type EthAddress = [u8; 20];

const NO_DEPOSIT: Balance = 0;

const TRANSFER_FROM_GAS: Gas = 50_000_000_000_000;
const FINISH_LOCK_GAS: Gas = 10_000_000_000_000;
const VERIFY_LOG_ENTRY_GAS: Gas = 50_000_000_000_000;

const PAUSE_LOCK_TOKEN: Mask = 1 << 0;

#[ext_contract(ext_nft_approval)]
pub trait ExtNFTContract {
    fn nft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
        msg: String,
    ) -> PromiseOrValue<bool>;

    fn nft_transfer(
        &mut self,
        receiver_id: AccountId,
        token_id: TokenId,
        approval_id: Option<u64>,
        memo: Option<String>,
    ) -> Promise;
}

#[ext_contract(ext_self)]
pub trait ExtLocker {
    fn finish_lock(
        &mut self,
        token_account_id: AccountId,
        token_id: String,
        eth_recipient: EthAddress,
    ) -> Promise;

    fn finish_unlock(
        &mut self,
        token_account_id: String,
        token_id: String,
        recipient: AccountId,
    ) -> Promise;
}

#[derive(Debug, Eq, PartialEq, Serialize, BorshSerialize, BorshDeserialize)]
pub enum ResultType {
    Unlock {
        token_account_id: AccountId,
        token_id: String,
        recipient: AccountId,
    },
    Lock {
        recipient: EthAddress,
        token_account_id: AccountId,
        token_id: String,
    },
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Locker {
    eth_factory_address: EthAddress,
    prover_account: ValidAccountId,
    paused: Mask
}

#[near_bindgen]
impl Locker {
    #[init]
    pub fn new(eth_factory_address: String, prover_account: ValidAccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self {
            eth_factory_address: validate_eth_address(eth_factory_address),
            prover_account: prover_account,
            paused: Mask::default(),
        }
    }

    #[payable]
    pub fn lock(
        &mut self,
        token_account_id: AccountId,
        token_id: String,
        eth_recipient: String,
    ) -> Promise {
        self.check_not_paused(PAUSE_LOCK_TOKEN);
        assert_one_yocto();
        let recipient = validate_eth_address(eth_recipient.clone());

        ext_nft_approval::nft_transfer(
            env::current_account_id(),
            token_id.clone(),
            None,
            None,
            &token_account_id,
            1,
            TRANSFER_FROM_GAS,
        )
        .then(ext_self::finish_lock(
            token_account_id,
            token_id,
            recipient,
            &env::current_account_id(),
            NO_DEPOSIT,
            FINISH_LOCK_GAS,
        ))
    }

    pub fn finish_lock(
        &mut self,
        token_account_id: AccountId,
        token_id: String,
        eth_recipient: EthAddress,
    ) -> ResultType {
        self.check_promise_result(0, String::from("Transfer token failed"));
        ResultType::Lock {
            token_account_id: token_account_id,
            token_id: token_id,
            recipient: eth_recipient,
        }
    }

    #[payable]
    pub fn unlock(&mut self, #[serializer(borsh)] proof: Proof) -> Promise {
        assert_one_yocto();

        let event = EthWithdrawEvent::from_log_entry_data(&proof.log_entry_data);

        ext_prover::verify_log_entry(
            proof.log_index,
            proof.log_entry_data,
            proof.receipt_index,
            proof.receipt_data,
            proof.header_data,
            proof.proof,
            false,
            &self.prover_account,
            NO_DEPOSIT,
            VERIFY_LOG_ENTRY_GAS,
        )
        .then(ext_nft_approval::nft_transfer(
            event.recipient.to_string().clone(),
            event.token_id.clone(),
            None,
            None,
            &event.token_account_id,
            1,
            TRANSFER_FROM_GAS,
        ))
        .then(ext_self::finish_unlock(
            event.token_account_id,
            event.token_id,
            event.recipient,
            &env::current_account_id(),
            NO_DEPOSIT,
            FINISH_LOCK_GAS,
        ))
    }

    pub fn finish_unlock(
        &mut self,
        token_account_id: String,
        token_id: String,
        recipient: AccountId,
    ) -> ResultType {
        self.check_promise_result(0, String::from("Failed to verify the nft_transfer_call."));
        ResultType::Unlock {
            token_account_id: token_account_id,
            token_id: token_id,
            recipient: recipient,
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

pub fn validate_eth_address(address: String) -> EthAddress {
    let data = hex::decode(address).expect("address should beg a valid hex string.");
    assert_eq!(data.len(), 20, "address should be 20 bytes long");
    let mut result = [0u8; 20];
    result.copy_from_slice(&data);
    result
}

pub fn is_valid_eth_address(address: String) {
    let mut valid: bool = true;
    if hex::decode(address.clone()).is_err() || hex::decode(address).unwrap().len() != 20 {
        valid = false;
    }
    assert!(valid, "Invalid ETH address")
}

admin_controlled::impl_admin_controlled!(Locker, paused);