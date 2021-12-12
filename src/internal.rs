use crate::*;

impl Contract {
    pub(crate) fn assert_deposit_whitelist(&self, account_id: &AccountId) {
        assert!(
            self.deposit_whitelist.contains(account_id),
            "Not in deposit whitelist"
        );
    }

    pub(crate) fn internal_add_lockup(&mut self, lockup: &Lockup) -> LockupIndex {
        let index = self.lockups.len() as LockupIndex;
        self.lockups.push(lockup);
        let mut indices = self
            .account_lockups
            .get(lockup.account_id.as_ref())
            .unwrap_or_default();
        indices.insert(index);
        self.internal_save_account_lockups(lockup.account_id.as_ref(), indices);
        index
    }

    pub(crate) fn internal_save_account_lockups(
        &mut self,
        account_id: &AccountId,
        indices: HashSet<LockupIndex>,
    ) {
        if indices.is_empty() {
            self.account_lockups.remove(account_id);
        } else {
            self.account_lockups.insert(account_id, &indices);
        }
    }

    pub(crate) fn internal_get_account_lockups(
        &self,
        account_id: &AccountId,
    ) -> Vec<(LockupIndex, Lockup)> {
        self.account_lockups
            .get(account_id)
            .unwrap_or_default()
            .into_iter()
            .map(|lockup_index| (lockup_index, self.lockups.get(lockup_index as _).unwrap()))
            .collect()
    }
}
