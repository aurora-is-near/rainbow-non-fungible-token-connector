use near_contract_standards::non_fungible_token::TokenId;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::{
    assert_one_yocto, env, ext_contract, near_bindgen, AccountId, Balance, Gas, PanicOnDefault,
    Promise, PromiseOrValue,
};

mod withdraw_event;
pub use withdraw_event::*;

mod prover;
pub use prover::{ext_prover, Proof};

pub type EthAddress = [u8; 20];

const NO_DEPOSIT: Balance = 0;

const TRANSFER_FROM_GAS: Gas = 10_000_000_000_000;
const FINISH_LOCK_GAS: Gas = 10_000_000_000_000;
const VERIFY_LOG_ENTRY_GAS: Gas = 50_000_000_000_000;

#[ext_contract(ext_nft_approval)]
pub trait ExtNFTContract {
    #[result_serializer(borsh)]
    fn nft_transfer_call(
        &mut self,
        #[serializer(borsh)] receiver_id: AccountId,
        #[serializer(borsh)] token_id: TokenId,
        #[serializer(borsh)] approval_id: Option<u64>,
        #[serializer(borsh)] memo: Option<String>,
        #[serializer(borsh)] msg: String,
    ) -> PromiseOrValue<bool>;
}

#[ext_contract(ext_self)]
pub trait ExtLocker {
    #[result_serializer(borsh)]
    fn finish_lock(
        &mut self,
        #[callback]
        #[serializer(borsh)]
        verification_success: bool,
        #[serializer(borsh)] token_account_id: AccountId,
        #[serializer(borsh)] token_id: String,
        #[serializer(borsh)] eth_recipient: String,
    ) -> ResultType;

    #[result_serializer(borsh)]
    fn transfer_token_to_owner(
        &mut self,
        #[callback]
        #[serializer(borsh)]
        verification_success: bool,
        #[serializer(borsh)] withdraw_address: EthAddress,
        #[serializer(borsh)] token_address: EthAddress,
        #[serializer(borsh)] sender: String,
        #[serializer(borsh)] token_id: String,
        #[serializer(borsh)] recipient: AccountId,
    ) -> Promise<bool>;

    #[result_serializer(borsh)]
    fn finish_unlock(
        &mut self,
        #[callback]
        #[serializer(borsh)]
        verification_success: bool,
        #[serializer(borsh)] withdraw_address: EthAddress,
        #[serializer(borsh)] token_account_id: String,
        #[serializer(borsh)] token_address: EthAddress,
        #[serializer(borsh)] sender: String,
        #[serializer(borsh)] token_id: String,
        #[serializer(borsh)] recipient: AccountId,
    ) -> ResultType;
}

#[derive(Debug, Eq, PartialEq, BorshSerialize, BorshDeserialize)]
pub enum ResultType {
    Unlock {
        token_account_id: AccountId,
        token_id: String,
        recipient: AccountId,
    },
    Lock {
        token_account_id: AccountId,
        token_id: String,
        recipient: AccountId,
    },
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Locker {
    eth_factory_address: EthAddress,
    prover_account: AccountId,
}

#[near_bindgen]
impl Locker {
    #[init]
    pub fn new(eth_factory_address: String, prover_account: AccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Locker {
            eth_factory_address: validate_eth_address(eth_factory_address),
            prover_account: prover_account,
        }
    }

    pub fn lock(
        &mut self,
        token_account_id: AccountId,
        token_id: String,
        eth_recipient: String,
    ) -> Promise {
        is_valid_eth_address(eth_recipient.clone());

        ext_nft_approval::nft_transfer_call(
            env::current_account_id(),
            token_id.clone(),
            None,
            None,
            String::from("lock token"),
            &token_account_id,
            NO_DEPOSIT,
            TRANSFER_FROM_GAS,
        )
        .then(ext_self::finish_lock(
            token_account_id,
            token_id,
            eth_recipient,
            &env::current_account_id(),
            NO_DEPOSIT,
            FINISH_LOCK_GAS,
        ))
    }

    pub fn finish_lock(
        &mut self,
        #[callback]
        #[serializer(borsh)]
        verification_success: bool,
        #[serializer(borsh)] token_account_id: AccountId,
        #[serializer(borsh)] token_id: String,
        #[serializer(borsh)] eth_recipient: String,
    ) -> ResultType {
        assert!(
            verification_success,
            "Failed to verify the nft_transfer_call."
        );
        ResultType::Lock {
            token_account_id: token_account_id,
            token_id: token_id,
            recipient: eth_recipient,
        }
    }

    pub fn unlock(&mut self, #[serializer(borsh)] proof: Proof) -> Promise {
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
        .then(ext_self::transfer_token_to_owner(
            event.withdraw_address,
            event.token_address,
            event.sender,
            event.token_id,
            event.recipient,
            &self.prover_account,
            NO_DEPOSIT,
            VERIFY_LOG_ENTRY_GAS,
        ))
    }

    fn transfer_token_to_owner(
        &mut self,
        #[callback]
        #[serializer(borsh)]
        verification_success: bool,
        #[serializer(borsh)] withdraw_address: EthAddress,
        #[serializer(borsh)] token_address: EthAddress,
        #[serializer(borsh)] token_account_id: String,
        #[serializer(borsh)] sender: String,
        #[serializer(borsh)] token_id: String,
        #[serializer(borsh)] recipient: AccountId,
    ) -> Promise {
        assert!(verification_success, "Failed to verify the proof.");

        ext_nft_approval::nft_transfer_call(
            recipient.to_string().clone(),
            token_id.clone(),
            None,
            None,
            String::from("lock token"),
            &token_account_id,
            NO_DEPOSIT,
            TRANSFER_FROM_GAS,
        )
        .then(ext_self::finish_unlock(
            withdraw_address,
            token_account_id,
            token_address,
            sender,
            token_id,
            recipient,
            &env::current_account_id(),
            NO_DEPOSIT,
            FINISH_LOCK_GAS,
        ))
    }

    pub fn finish_unlock(
        &mut self,
        #[callback]
        #[serializer(borsh)]
        verification_success: bool,
        #[serializer(borsh)] withdraw_address: EthAddress,
        #[serializer(borsh)] token_account_id: String,
        #[serializer(borsh)] token_address: EthAddress,
        #[serializer(borsh)] sender: String,
        #[serializer(borsh)] token_id: String,
        #[serializer(borsh)] recipient: AccountId,
    ) -> ResultType {
        assert!(
            verification_success,
            "Failed to verify the nft_transfer_call."
        );

        ResultType::Unlock {
            token_account_id: token_account_id,
            token_id: token_id,
            recipient: recipient,
        }
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

#[cfg(test)]
mod tests {
    #[test]
    fn simple() {}
}
