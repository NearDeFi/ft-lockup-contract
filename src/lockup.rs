use crate::*;
use std::convert::TryInto;

pub type LockupIndex = u32;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Clone))]
pub struct BatchedUsers {
    pub beneficiary_id: Option<ValidAccountId>,
    pub batch: Vec<(ValidAccountId, U128)>,
}

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
    pub fn new_unlocked_since(
        account_id: AccountId,
        total_balance: Balance,
        timestamp: TimestampSec,
    ) -> Self {
        Self {
            account_id: account_id.try_into().unwrap(),
            schedule: Schedule::new_unlocked_since(total_balance, timestamp),
            claimed_balance: 0,
            termination_config: None,
        }
    }

    pub fn new_unlocked(account_id: AccountId, total_balance: Balance) -> Self {
        Self::new_unlocked_since(account_id, total_balance, 1)
    }

    pub fn claim(&mut self, index: LockupIndex, claim_amount: Balance) -> LockupClaim {
        let unlocked_balance = self.schedule.unlocked_balance(current_timestamp_sec());
        let balance_claimed_new = self
            .claimed_balance
            .checked_add(claim_amount)
            .expect("attempt to add with overflow");
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

#[cfg(not(target_arch = "wasm32"))]
impl LockupCreate {
    pub fn new_unlocked(account_id: ValidAccountId, total_balance: Balance) -> Self {
        Self {
            account_id,
            schedule: Schedule::new_unlocked(total_balance),
            vesting_schedule: None,
        }
    }
}

impl LockupCreate {
    pub fn into_lockup(&self, payer_id: &ValidAccountId) -> Lockup {
        let vesting_schedule = self.vesting_schedule.clone();
        Lockup {
            account_id: self.account_id.clone(),
            schedule: self.schedule.clone(),
            claimed_balance: 0,
            termination_config: match vesting_schedule {
                None => None,
                Some(vesting_schedule) => Some(TerminationConfig {
                    beneficiary_id: payer_id.clone(),
                    vesting_schedule,
                }),
            },
        }
    }
}
