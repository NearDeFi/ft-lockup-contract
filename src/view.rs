use crate::*;
use std::convert::TryInto;

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Deserialize))]
pub struct LockupView {
    pub account_id: ValidAccountId,
    pub schedule: Schedule,

    #[serde(default)]
    #[serde(with = "u128_dec_format")]
    pub claimed_balance: Balance,
    /// An optional configuration that allows vesting/lockup termination.
    pub termination_config: Option<TerminationConfig>,

    #[serde(with = "u128_dec_format")]
    pub total_balance: Balance,
    #[serde(with = "u128_dec_format")]
    pub unclaimed_balance: Balance,
    /// The current timestamp
    pub timestamp: TimestampSec,
}

impl From<Lockup> for LockupView {
    fn from(lockup: Lockup) -> Self {
        let total_balance = lockup.schedule.total_balance();
        let timestamp = current_timestamp_sec();
        let unclaimed_balance =
            lockup.schedule.unlocked_balance(timestamp) - lockup.claimed_balance;
        let Lockup {
            account_id,
            schedule,
            claimed_balance,
            termination_config,
        } = lockup;
        Self {
            account_id,
            schedule,
            claimed_balance,
            termination_config,
            total_balance,
            unclaimed_balance,
            timestamp,
        }
    }
}

#[near_bindgen]
impl Contract {
    pub fn get_token_account_id(&self) -> ValidAccountId {
        self.token_account_id.clone().try_into().unwrap()
    }

    pub fn get_account_lockups(
        &self,
        account_id: ValidAccountId,
    ) -> Vec<(LockupIndex, LockupView)> {
        self.internal_get_account_lockups(account_id.as_ref())
            .into_iter()
            .map(|(lockup_index, lockup)| (lockup_index, lockup.into()))
            .collect()
    }

    pub fn get_lockup(&self, index: LockupIndex) -> Option<LockupView> {
        self.lockups.get(index as _).map(|lockup| lockup.into())
    }

    pub fn get_lockups(&self, indices: Vec<LockupIndex>) -> Vec<(LockupIndex, LockupView)> {
        indices
            .into_iter()
            .filter_map(|index| self.get_lockup(index).map(|lockup| (index, lockup)))
            .collect()
    }

    pub fn get_num_lockups(&self) -> u32 {
        self.lockups.len() as _
    }

    pub fn get_lockups_paged(
        &self,
        from_index: Option<LockupIndex>,
        limit: Option<LockupIndex>,
    ) -> Vec<(LockupIndex, LockupView)> {
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(self.get_num_lockups());
        (from_index..std::cmp::min(self.get_num_lockups(), limit))
            .filter_map(|index| self.get_lockup(index).map(|lockup| (index, lockup)))
            .collect()
    }

    pub fn get_deposit_whitelist(&self) -> Vec<AccountId> {
        self.deposit_whitelist.to_vec()
    }

    pub fn hash_schedule(&self, schedule: Schedule) -> Base58CryptoHash {
        schedule.hash().into()
    }

    pub fn validate_schedule(
        &self,
        schedule: Schedule,
        total_balance: WrappedBalance,
        termination_schedule: Option<Schedule>,
    ) {
        schedule.assert_valid(total_balance.0);
        if let Some(termination_schedule) = termination_schedule {
            termination_schedule.assert_valid(total_balance.0);
            schedule.assert_valid_termination_schedule(&termination_schedule);
        }
    }
}
