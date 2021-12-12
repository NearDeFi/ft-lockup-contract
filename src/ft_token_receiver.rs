use crate::*;

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
        self.assert_deposit_whitelist(sender_id.as_ref());
        let lockup: Lockup = serde_json::from_str(&msg).expect("Expected Lockup as msg");
        let amount = amount.into();
        lockup.assert_new_valid(amount);
        let index = self.internal_add_lockup(&lockup);
        log!(
            "Created new lockup for {} with index {}",
            lockup.account_id.as_ref(),
            index
        );
        PromiseOrValue::Value(0.into())
    }
}
