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

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Deserialize))]
pub struct LockupCreateView {
    pub account_id: ValidAccountId,
    pub schedule: Schedule,

    #[serde(default)]
    #[serde(with = "u128_dec_format")]
    pub claimed_balance: Balance,
    /// An optional configuration that allows vesting/lockup termination.
    pub vesting_schedule: Option<HashOrSchedule>,

    #[serde(with = "u128_dec_format")]
    pub total_balance: Balance,
    #[serde(with = "u128_dec_format")]
    pub unclaimed_balance: Balance,
    /// The current timestamp
    pub timestamp: TimestampSec,
}

impl From<LockupCreate> for LockupCreateView {
    fn from(lockup_create: LockupCreate) -> Self {
        let total_balance = lockup_create.schedule.total_balance();
        let timestamp = current_timestamp_sec();
        let unclaimed_balance = lockup_create.schedule.unlocked_balance(timestamp);
        let LockupCreate {
            account_id,
            schedule,
            vesting_schedule,
        } = lockup_create;
        Self {
            account_id,
            schedule,
            claimed_balance: 0,
            vesting_schedule,
            total_balance,
            unclaimed_balance,
            timestamp,
        }
    }
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Deserialize))]
pub struct DraftGroupView {
    #[serde(default)]
    #[serde(with = "u128_dec_format")]
    pub total_amount: Balance,
    pub funded: bool,
    pub draft_indices: Vec<DraftIndex>,
}

impl From<DraftGroup> for DraftGroupView {
    fn from(draft_group: DraftGroup) -> Self {
        Self {
            total_amount: draft_group.total_amount,
            funded: draft_group.payer_id.is_some(),
            draft_indices: draft_group.draft_indices.into_iter().collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Deserialize))]
pub struct DraftView {
    pub draft_group_id: DraftGroupIndex,
    pub lockup: LockupCreateView,
}

impl From<Draft> for DraftView {
    fn from(draft: Draft) -> Self {
        Self {
            draft_group_id: draft.draft_group_id,
            lockup: draft.lockup_create.into(),
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

    pub fn get_draft_group(&self, index: DraftGroupIndex) -> Option<DraftGroupView> {
        self.draft_groups.get(index as _).map(|group| group.into())
    }

    pub fn get_draft_groups_paged(
        &self,
        from_index: Option<DraftGroupIndex>,
        to_index: Option<DraftGroupIndex>,
    ) -> Vec<(DraftGroupIndex, DraftGroupView)> {
        let from_index = from_index.unwrap_or(0);
        let to_index = to_index.unwrap_or(self.draft_groups.len() as _);
        (from_index..std::cmp::min(self.draft_groups.len() as _, to_index))
            .filter_map(|index| self.get_draft_group(index).map(|group| (index, group)))
            .collect()
    }

    pub fn get_draft(&self, index: DraftIndex) -> Option<DraftView> {
        self.drafts.get(&index as _).map(|draft| draft.into())
    }

    pub fn get_drafts(&self, indices: Vec<DraftIndex>) -> Vec<(DraftIndex, DraftView)> {
        indices
            .into_iter()
            .filter_map(|index| self.get_draft(index).map(|draft| (index, draft)))
            .collect()
    }
}
