use crate::*;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct Checkpoint {
    /// The unix-timestamp in seconds since the epoch.
    pub timestamp: TimestampSec,
    #[serde(with = "u128_dec_format")]
    pub balance: Balance,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Clone))]
pub struct Schedule(pub Vec<Checkpoint>);

impl Schedule {
    pub fn new_unlocked(total_balance: Balance) -> Self {
        Self(vec![
            Checkpoint {
                timestamp: 0,
                balance: 0,
            },
            Checkpoint {
                timestamp: 1,
                balance: total_balance,
            },
        ])
    }

    pub fn assert_valid(&self, total_balance: Balance) {
        assert!(self.0.len() >= 2, "At least two checkpoints is required");
        assert_eq!(
            self.0.first().unwrap().balance,
            0,
            "The first checkpoint balance should be 0"
        );
        for i in 1..self.0.len() {
            assert!(self.0[i - 1].timestamp < self.0[i].timestamp, "The timestamp of checkpoint #{} should be less than the timestamp of the next checkpoint", i - 1);
            assert!(self.0[i - 1].balance <= self.0[i].balance, "The balance of checkpoint #{} should be not greater than the balance of the next checkpoint", i - 1);
        }
        assert_eq!(
            self.total_balance(),
            total_balance,
            "The schedule's total balance doesn't match the transferred balance"
        );
    }

    /// Verifies that this schedule is ahead of the given termination schedule at any point of time.
    /// Assumes they have equal total balance and both schedules are valid.
    pub fn assert_valid_termination_schedule(&self, termination_schedule: &Schedule) {
        for checkpoint in &self.0 {
            assert!(
                checkpoint.balance <= termination_schedule.unlocked_balance(checkpoint.timestamp),
                "The lockup schedule is ahead of the termination schedule at timestamp {}",
                checkpoint.timestamp
            );
        }
        for checkpoint in &termination_schedule.0 {
            assert!(
                checkpoint.balance >= self.unlocked_balance(checkpoint.timestamp),
                "The lockup schedule is ahead of the termination schedule at timestamp {}",
                checkpoint.timestamp
            );
        }
    }

    pub fn unlocked_balance(&self, current_timestamp: TimestampSec) -> Balance {
        // Using binary search by time to find the current checkpoint.
        let index = match self
            .0
            .binary_search_by_key(&current_timestamp, |checkpoint| checkpoint.timestamp)
        {
            // Exact timestamp found
            Ok(index) => index,
            // No match, the next index is given.
            Err(index) => {
                if index == 0 {
                    // Not started
                    return 0;
                }
                index - 1
            }
        };
        let checkpoint = &self.0[index];
        if index + 1 == self.0.len() {
            // The last checkpoint. Fully unlocked.
            return checkpoint.balance;
        }
        let next_checkpoint = &self.0[index + 1];

        let total_duration = next_checkpoint.timestamp - checkpoint.timestamp;
        let passed_duration = current_timestamp - checkpoint.timestamp;
        checkpoint.balance
            + (U256::from(passed_duration)
                * U256::from(next_checkpoint.balance - checkpoint.balance)
                / U256::from(total_duration))
            .as_u128()
    }

    pub fn total_balance(&self) -> Balance {
        self.0.last().unwrap().balance
    }

    /// Terminates the lockup schedule earlier.
    /// Assumes new_total_balance is not greater than the current total balance.
    pub fn terminate(&mut self, new_total_balance: Balance) {
        if new_total_balance == 0 {
            self.0 = Self::new_unlocked(0).0;
            return;
        }
        assert!(
            new_total_balance <= self.0.last().unwrap().balance,
            "Invariant"
        );
        while let Some(checkpoint) = self.0.pop() {
            if self.0.last().unwrap().balance < new_total_balance {
                let prev_checkpoint = self.0.last().unwrap().clone();
                let timestamp_diff = checkpoint.timestamp - prev_checkpoint.timestamp;
                let balance_diff = checkpoint.balance - prev_checkpoint.balance;
                let required_balance_diff = new_total_balance - prev_checkpoint.balance;
                // Computing the new timestamp rounding up
                let new_timestamp = prev_checkpoint.timestamp
                    + ((U256::from(timestamp_diff) * U256::from(required_balance_diff)
                        + U256::from(balance_diff - 1))
                        / U256::from(balance_diff))
                    .as_u32();
                self.0.push(Checkpoint {
                    timestamp: new_timestamp,
                    balance: new_total_balance,
                });
                return;
            }
        }
        unreachable!();
    }

    pub fn hash(&self) -> CryptoHash {
        let value_hash = env::sha256(&self.try_to_vec().unwrap());
        let mut res = CryptoHash::default();
        res.copy_from_slice(&value_hash);

        res
    }
}
