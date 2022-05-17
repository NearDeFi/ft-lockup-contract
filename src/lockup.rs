use crate::*;
use std::convert::TryInto;

pub type LockupIndex = u32;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct LockupClaim {
    pub index: LockupIndex,
    pub claim_amount: WrappedBalance,
    pub is_final: bool,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(
    not(target_arch = "wasm32"),
    derive(Debug, PartialEq, Clone, Serialize)
)]
pub struct Lockup {
    pub account_id: ValidAccountId,
    pub schedule: Schedule,

    #[serde(default)]
    #[serde(with = "u128_dec_format")]
    pub claimed_balance: Balance,
    /// An optional configuration that allows vesting/lockup termination.
    pub termination_config: Option<TerminationConfig>,
}

impl Lockup {
    pub fn new_unlocked(account_id: AccountId, total_balance: Balance) -> Self {
        Self {
            account_id: account_id.try_into().unwrap(),
            schedule: Schedule::new_unlocked(total_balance),
            claimed_balance: 0,
            termination_config: None,
        }
    }

    pub fn claim_balance(&mut self, index: LockupIndex, claim_amount: Balance) -> LockupClaim {
        let unlocked_balance = self.schedule.unlocked_balance(current_timestamp_sec());
        let balance_claimed_new = self.claimed_balance + claim_amount;
        assert!(
            unlocked_balance >= balance_claimed_new,
            "too big claim_amount for lockup {}",
            index,
        );

        self.claimed_balance = balance_claimed_new;
        LockupClaim {
            index,
            claim_amount: claim_amount.into(),
            is_final: balance_claimed_new == self.schedule.total_balance(),
        }
    }

    pub fn claim(&mut self, index: LockupIndex) -> LockupClaim {
        let unlocked_balance = self.schedule.unlocked_balance(current_timestamp_sec());
        assert!(unlocked_balance >= self.claimed_balance, "Invariant");
        let claim_amount = unlocked_balance - self.claimed_balance;

        self.claim_balance(index, claim_amount)
    }

    pub fn assert_new_valid(&self, total_balance: Balance) {
        assert_eq!(
            self.claimed_balance, 0,
            "The initial lockup claimed balance should be 0"
        );
        self.schedule.assert_valid(total_balance);

        if let Some(termination_config) = &self.termination_config {
            match &termination_config.vesting_schedule {
                None => {
                    // Ok, using lockup schedule.
                }
                Some(HashOrSchedule::Hash(_hash)) => {
                    // Ok, using unknown hash. Can't verify.
                }
                Some(HashOrSchedule::Schedule(schedule)) => {
                    schedule.assert_valid(total_balance);
                    self.schedule.assert_valid_termination_schedule(&schedule);
                }
            }
        }
    }
}
