use crate::*;

const TGE_TIMESTAMP: u32 = 1663059600; // 2022-09-13T09:00:00 UTC
const FULL_UNLOCK_TIMESTAMP: u32 = 1726218000; // 2024-09-13T09:00:00 UTC

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
        let batched_users: BatchedUsers =
            serde_json::from_str(&msg).expect("Expected BatchedUsers as msg");

        let mut sum: u128 = 0;
        for (account_id, sweat) in batched_users.batch {
            let account_total = sweat.0;
            sum = sum + account_total;

            let user_lockup = Lockup {
                account_id: account_id,
                schedule: Schedule(vec![
                    Checkpoint {
                        timestamp: TGE_TIMESTAMP - 1,
                        balance: 0
                    },
                    Checkpoint {
                        timestamp: TGE_TIMESTAMP,
                        balance: 10 * account_total / 100,
                    },
                    Checkpoint {
                        timestamp: FULL_UNLOCK_TIMESTAMP,
                        balance: account_total,
                    },
                ]),
                claimed_balance: 0,
                termination_config: None,
            };
            user_lockup.assert_new_valid(account_total);
            let _index = self.internal_add_lockup(&user_lockup);
        }
        assert_eq!(amount.0, sum);
        PromiseOrValue::Value(0.into())
    }
}
