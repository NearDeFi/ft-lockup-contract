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
        let lockup_ids = self.account_lockups.get(account_id).unwrap_or_default();
        self.internal_get_account_lockups_by_id(&account_id, &lockup_ids)
    }

    pub(crate) fn internal_get_account_lockups_by_id(
        &self,
        account_id: &AccountId,
        lockup_ids: &HashSet<LockupIndex>,
    ) -> Vec<(LockupIndex, Lockup)> {
        let account_lockup_ids = self.account_lockups.get(account_id).unwrap_or_default();

        lockup_ids
            .iter()
            .map(|&lockup_index| {
                assert!(
                    account_lockup_ids.contains(&lockup_index),
                    "lockup not found for account: {}",
                    lockup_index,
                );
                let lockup = self.lockups.get(lockup_index as _).unwrap();
                (lockup_index.clone(), lockup)
            })
            .collect()
    }

    pub(crate) fn internal_claim_lockups(
        &mut self,
        amounts: HashMap<LockupIndex, WrappedBalance>,
        mut lockups_by_id: HashMap<LockupIndex, Lockup>,
    ) -> PromiseOrValue<WrappedBalance> {
        let account_id = env::predecessor_account_id();
        let mut lockup_claims = vec![];
        let mut total_claim_amount = 0;
        for (lockup_index, lockup_amount) in amounts {
            let lockup = lockups_by_id.get_mut(&lockup_index).unwrap();
            let lockup_claim = lockup.claim(lockup_index, lockup_amount.0);

            if lockup_claim.claim_amount.0 > 0 {
                log!(
                    "Claiming {} form lockup #{}",
                    lockup_claim.claim_amount.0,
                    lockup_index
                );
                total_claim_amount += lockup_claim.claim_amount.0;
                self.lockups.replace(lockup_index as _, &lockup);
                lockup_claims.push(lockup_claim);
            }
        }
        log!("Total claim {}", total_claim_amount);

        if total_claim_amount > 0 {
            ext_fungible_token::ft_transfer(
                account_id.clone(),
                total_claim_amount.into(),
                Some(format!(
                    "Claiming unlocked {} balance from {}",
                    total_claim_amount,
                    env::current_account_id()
                )),
                &self.token_account_id,
                ONE_YOCTO,
                GAS_FOR_FT_TRANSFER,
            )
            .then(ext_self::after_ft_transfer(
                account_id,
                lockup_claims,
                &env::current_account_id(),
                NO_DEPOSIT,
                GAS_FOR_AFTER_FT_TRANSFER,
            ))
            .into()
        } else {
            PromiseOrValue::Value(0.into())
        }
    }
}
