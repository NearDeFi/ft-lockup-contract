use crate::*;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct DraftGroupFunding {
    pub draft_group_id: DraftGroupIndex,
}

#[near_bindgen]
impl FungibleTokenReceiver for Contract {
    fn ft_on_transfer(
        &mut self,
        sender_id: ValidAccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        assert_eq!(
            env::predecessor_account_id(),
            self.token_account_id,
            "Invalid token ID"
        );
        let amount = amount.into();
        self.assert_deposit_whitelist(sender_id.as_ref());
        let lockup: Result<Lockup, _> = serde_json::from_str(&msg);
        if let Ok(lockup) = lockup {
            lockup.assert_new_valid(amount);
            let index = self.internal_add_lockup(&lockup);
            log!(
                "Created new lockup for {} with index {}",
                lockup.account_id.as_ref(),
                index
            );
            return PromiseOrValue::Value(0.into());
        }
        let funding: Result<DraftGroupFunding, _> = serde_json::from_str(&msg);
        if let Ok(funding) = funding {
            let draft_group_id = funding.draft_group_id;
            let mut draft_group = self
                .draft_groups
                .get(draft_group_id as _)
                .expect("draft group not found");
            assert_eq!(
                draft_group.total_amount, amount,
                "The draft group total balance doesn't match the transferred balance",
            );
            assert!(!draft_group.funded, "draft group already funded");
            draft_group.funded = true;
            self.draft_groups.replace(draft_group_id as _, &draft_group);
            log!("Funded draft group {}", draft_group_id);
            return PromiseOrValue::Value(0.into());
        }

        panic!("Expected Lockup or DraftGroupFunding as msg");
    }
}
