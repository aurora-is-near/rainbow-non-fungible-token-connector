use crate::prover::{EthAddress, EthEvent, EthEventParams};
use ethabi::{ParamType, Token};
use hex::ToHex;
use near_sdk::AccountId;

/// Data that was emitted by the Ethereum Withdraw event.
#[derive(Debug, Eq, PartialEq)]
pub struct EthWithdrawEvent {
    pub withdraw_address: EthAddress,
    pub token_address: String,
    pub sender: String,
    pub token_account_id: String,
    pub token_id: String,
    pub recipient: AccountId,
}

impl EthWithdrawEvent {
    fn event_params() -> EthEventParams {
        vec![
            ("token_address".to_string(), ParamType::Address, true),
            ("sender".to_string(), ParamType::Address, true),
            ("token_account_id".to_string(), ParamType::String, false),
            ("token_id".to_string(), ParamType::String, false),
            ("account_id".to_string(), ParamType::String, false),
        ]
    }

    pub fn from_log_entry_data(data: &[u8]) -> Self {
        let event = EthEvent::from_log_entry_data("Withdraw", EthWithdrawEvent::event_params(), data);
        // let withdraw_address = event.log.params[0].value.clone().to_address().unwrap().0;
        let token_address = event.log.params[0]
            .value
            .clone()
            .to_address()
            .unwrap()
            .0
            .encode_hex::<String>();
        let sender = event.log.params[1]
            .value
            .clone()
            .to_address()
            .unwrap()
            .0
            .encode_hex::<String>();
        let token_account_id = event.log.params[2].value.clone().to_string().unwrap();
        let token_id = event.log.params[3].value.clone().to_string().unwrap();
        let recipient = event.log.params[4].value.clone().to_string().unwrap();
        EthWithdrawEvent {
            withdraw_address: event.withdraw_address,
            token_address: token_address,
            token_account_id: token_account_id,
            sender: sender,
            token_id: token_id,
            recipient: recipient,
        }
    }

    pub fn to_log_entry_data(&self) -> Vec<u8> {
        EthEvent::to_log_entry_data(
            "Withdraw",
            EthWithdrawEvent::event_params(),
            self.withdraw_address,
            vec![
                hex::decode(self.token_address.clone()).unwrap(),
                hex::decode(self.sender.clone()).unwrap(),
            ],
            vec![
                Token::String(self.token_account_id.to_string()),
                Token::String(self.token_id.to_string()),
                Token::String(self.recipient.clone()),
            ],
        )
    }
}
