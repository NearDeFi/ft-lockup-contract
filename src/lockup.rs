use crate::*;
use std::convert::TryInto;

pub type LockupIndex = u32;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq))]
pub struct LockupClaim {
    pub index: LockupIndex,
    pub unclaimed_balance: WrappedBalance,
    pub is_final: bool,
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Clone))]
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

    pub fn claim(&mut self, index: LockupIndex) -> LockupClaim {
        let unlocked_balance = self.schedule.unlocked_balance(current_timestamp_sec());
        assert!(unlocked_balance >= self.claimed_balance, "Invariant");
        let unclaimed_balance = unlocked_balance - self.claimed_balance;
        self.claimed_balance = unlocked_balance;
        LockupClaim {
            index,
            unclaimed_balance: unclaimed_balance.into(),
            is_final: unlocked_balance == self.schedule.total_balance(),
        }
    }

    pub fn assert_new_valid(&self, total_balance: Balance) {
        assert_eq!(
            self.claimed_balance, 0,
            "The initial lockup claimed balance should be 0"
        );
        self.schedule.assert_valid(total_balance);

        if let Some(termination_config) = &self.termination_config {
            match &termination_config.vesting_schedule {
                VestingConditions::SameAsLockupSchedule => {
                    // Ok, using lockup schedule.
                }
                VestingConditions::Hash(_hash) => {
                    // Ok, using unknown hash. Can't verify.
                }
                VestingConditions::Schedule(schedule) => {
                    schedule.assert_valid(total_balance);
                    self.schedule.assert_valid_termination_schedule(&schedule);
                }
            }
        }
    }
}

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Clone))]
pub struct LockupCreate {
    pub account_id: ValidAccountId,
    pub schedule: Schedule,
    pub vesting_schedule: Option<VestingConditions>,
}

impl LockupCreate {
    pub fn new_unlocked(account_id: AccountId, total_balance: Balance) -> Self {
        Self {
            account_id: account_id.try_into().unwrap(),
            schedule: Schedule::new_unlocked(total_balance),
            vesting_schedule: None,
        }
    }

    pub fn into_lockup(&self, beneficiary_id: &ValidAccountId) -> Lockup {
        let vesting_schedule = self.vesting_schedule.clone();
        Lockup {
            account_id: self.account_id.clone(),
            schedule: self.schedule.clone(),
            claimed_balance: 0,
            termination_config: match vesting_schedule {
                None => None,
                Some(vesting_schedule) => Some(TerminationConfig {
                    beneficiary_id: beneficiary_id.clone(),
                    vesting_schedule,
                }),
            },
        }
    }
}
