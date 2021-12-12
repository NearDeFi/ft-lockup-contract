use crate::*;

#[near_bindgen]
impl Contract {
    pub fn get_account_lockups(&self, account_id: ValidAccountId) -> Vec<(LockupIndex, Lockup)> {
        self.internal_get_account_lockups(account_id.as_ref())
    }

    pub fn get_lockup(&self, index: LockupIndex) -> Option<Lockup> {
        self.lockups.get(index as _)
    }

    pub fn get_lockups(&self, indices: Vec<LockupIndex>) -> Vec<(LockupIndex, Lockup)> {
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
    ) -> Vec<(LockupIndex, Lockup)> {
        let from_index = from_index.unwrap_or(0);
        let limit = limit.unwrap_or(self.get_num_lockups());
        (from_index..std::cmp::min(self.get_num_lockups(), limit))
            .filter_map(|index| self.get_lockup(index).map(|lockup| (index, lockup)))
            .collect()
    }

    pub fn get_deposit_whitelist(&self) -> Vec<AccountId> {
        self.deposit_whitelist.to_vec()
    }

    pub fn hash_schedule(schedule: Schedule) -> Base58CryptoHash {
        schedule.hash().into()
    }

    pub fn validate_schedule(
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
