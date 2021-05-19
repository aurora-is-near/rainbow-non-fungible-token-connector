use crate::prover::{EthAddress, EthEvent, EthEventParams};
use ethabi::{ParamType, Token};
use hex::ToHex;
use near_sdk::{AccountId, Balance};

/// Data that was emitted by the Ethereum Locked event.
#[derive(Debug, Eq, PartialEq)]
pub struct LockedEvent {
    pub locker_address: EthAddress,
    pub token: String,
    pub sender: String,
    pub token_id: String,
    pub recipient: AccountId,
}

impl LockedEvent {
    fn event_params() -> EthEventParams {
        vec![
            ("token".to_string(), ParamType::Address, true),
            ("sender".to_string(), ParamType::Address, true),
            ("token_id".to_string(), ParamType::String, false),
            ("account_id".to_string(), ParamType::String, false),
        ]
    }

    /// Parse raw log entry data.
    pub fn from_log_entry_data(data: &[u8]) -> Self {
        let event = EthEvent::from_log_entry_data(
            "Locked",
            LockedEvent::event_params(),
            data,
        );
        let token = event.log.params[0].value.clone().to_address().unwrap().0;
        let token = (&token).encode_hex::<String>();

        let sender = event.log.params[1].value.clone().to_address().unwrap().0;
        let sender = (&sender).encode_hex::<String>();

        let token_id = event.log.params[2].value.clone().to_string().unwrap();
        let recipient = event.log.params[3].value.clone().to_string().unwrap();

        Self {
            locker_address: event.locker_address,
            token,
            sender,
            token_id,
            recipient,
        }
    }

    pub fn to_log_entry_data(&self) -> Vec<u8> {
        EthEvent::to_log_entry_data(
            "LockedEvent",
            LockedEvent::event_params(),
            self.locker_address,
            vec![
                hex::decode(self.token.clone()).unwrap()
                hex::decode(self.sender.clone()).unwrap()
            ],
            vec![
                Token::String(self.token_id.into()),
                Token::String(self.recipient.clone()),
            ],
        )
    }
}

// impl std::fmt::Display for LockedEvent {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(
//             f,
//             "sender: {}; amount: {}; recipient: {}",
//             self.sender, self.amount, self.recipient
//         )
//     }
// }
//
// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn test_event_data() {
//         let event_data = LockedEvent {
//             locker_address: [0u8; 20],
//             sender: "00005474e89094c44da98b954eedeac495271d0f".to_string(),
//             amount: 1000,
//             recipient: "123".to_string(),
//         };
//         let data = event_data.to_log_entry_data();
//         let result = LockedEvent::from_log_entry_data(&data);
//         assert_eq!(result, event_data);
//     }
// }
