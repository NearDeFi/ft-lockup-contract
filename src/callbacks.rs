use crate::*;

trait SelfCallbacks {
    fn after_ft_transfer(
        &mut self,
        account_id: AccountId,
        lockup_claims: Vec<LockupClaim>,
    ) -> WrappedBalance;

    fn after_lockup_termination(
        &mut self,
        account_id: AccountId,
        amount: WrappedBalance,
    ) -> WrappedBalance;
}

#[near_bindgen]
impl SelfCallbacks for Contract {
    #[private]
    fn after_ft_transfer(
        &mut self,
        account_id: AccountId,
        lockup_claims: Vec<LockupClaim>,
    ) -> WrappedBalance {
        let promise_success = is_promise_success();
        let mut total_balance = 0;
        if promise_success {
            let mut remove_indices = vec![];
            for LockupClaim {
                index,
                is_final,
                unclaimed_balance,
            } in lockup_claims
            {
                if is_final {
                    remove_indices.push(index);
                }
                total_balance += unclaimed_balance.0;
            }
            if !remove_indices.is_empty() {
                let mut indices = self.account_lockups.get(&account_id).unwrap_or_default();
                for index in remove_indices {
                    indices.remove(&index);
                }
                self.internal_save_account_lockups(&account_id, indices);
            }
        } else {
            log!("Token transfer has failed. Refunding.");
            let mut modified = false;
            let mut indices = self.account_lockups.get(&account_id).unwrap_or_default();
            for LockupClaim {
                index,
                unclaimed_balance,
                ..
            } in lockup_claims
            {
                if indices.insert(index) {
                    modified = true;
                }
                let mut lockup = self.lockups.get(index as _).unwrap();
                lockup.claimed_balance -= unclaimed_balance.0;
                self.lockups.replace(index as _, &lockup);
            }

            if modified {
                self.internal_save_account_lockups(&account_id, indices);
            }
        }
        total_balance.into()
    }

    #[private]
    fn after_lockup_termination(
        &mut self,
        account_id: AccountId,
        amount: WrappedBalance,
    ) -> WrappedBalance {
        let promise_success = is_promise_success();
        if !promise_success {
            log!("Lockup termination transfer has failed.");
            // There is no internal balance, so instead we create a new lockup.
            let lockup = Lockup::new_unlocked(account_id, amount.0);
            let lockup_index = self.internal_add_lockup(&lockup);
            log!(
                "Generated a new lockup #{} as a refund of {} for account {}",
                lockup_index,
                amount.0,
                lockup.account_id.as_ref(),
            );
            0.into()
        } else {
            amount
        }
    }
}
