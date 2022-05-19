use crate::*;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub enum HashOrSchedule {
    Hash(Base58CryptoHash),
    Schedule(Schedule),
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Clone))]
pub struct TerminationConfig {
    /// The account ID that can terminate vesting.
    pub payer_id: ValidAccountId,
    /// An optional vesting schedule
    pub vesting_schedule: HashOrSchedule,
}

impl Lockup {
    pub fn terminate(&mut self, hashed_schedule: Option<Schedule>) -> (Balance, ValidAccountId) {
        let termination_config = self
            .termination_config
            .take()
            .expect("No termination config");
        let payer_id = termination_config.payer_id;
        let total_balance = self.schedule.total_balance();
        let current_timestamp = current_timestamp_sec();
        let vested_balance = match &termination_config.vesting_schedule {
            HashOrSchedule::Hash(hash) => {
                let schedule = hashed_schedule
                    .as_ref()
                    .expect("Revealed schedule required for the termination");
                let hash: CryptoHash = (*hash).into();
                assert_eq!(
                    hash,
                    schedule.hash(),
                    "The revealed schedule hash doesn't match"
                );
                schedule.assert_valid(total_balance);
                self.schedule.assert_valid_termination_schedule(schedule);
                schedule
            }
            HashOrSchedule::Schedule(schedule) => &schedule,
        }
        .unlocked_balance(current_timestamp);
        let unvested_balance = total_balance - vested_balance;
        if unvested_balance > 0 {
            self.schedule.terminate(vested_balance);
        }
        (unvested_balance, payer_id)
    }
}
