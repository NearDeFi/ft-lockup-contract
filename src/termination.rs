use crate::*;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Clone))]
pub enum HashOrSchedule {
    Hash(Base58CryptoHash),
    Schedule(Schedule),
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Clone))]
pub struct TerminationConfig {
    /// The account ID that can terminate vesting.
    pub terminator_id: ValidAccountId,
    /// An optional vesting schedule
    pub vesting_schedule: Option<HashOrSchedule>,
}

impl Lockup {
    pub fn terminate(
        &mut self,
        initiator_id: &AccountId,
        hashed_schedule: Option<Schedule>,
    ) -> Balance {
        let termination_config = self
            .termination_config
            .take()
            .expect("No termination config");
        assert_eq!(
            termination_config.terminator_id.as_ref(),
            initiator_id,
            "Unauthorized"
        );
        let total_balance = self.schedule.total_balance();
        let current_timestamp = current_timestamp_sec();
        let vested_balance = match &termination_config.vesting_schedule {
            None => &self.schedule,
            Some(HashOrSchedule::Hash(hash)) => {
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
            Some(HashOrSchedule::Schedule(schedule)) => &schedule,
        }
        .unlocked_balance(current_timestamp);
        let unvested_balance = total_balance - vested_balance;
        if unvested_balance > 0 {
            self.schedule.terminate(vested_balance);
        }
        unvested_balance
    }
}
