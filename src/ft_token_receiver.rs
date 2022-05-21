use crate::*;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct DraftGroupFunding {
    pub draft_group_id: DraftGroupIndex,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
pub enum FtMessage {
    LockupCreate(LockupCreate),
    DraftGroupFunding(DraftGroupFunding),
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

        let ft_message: FtMessage = serde_json::from_str(&msg).unwrap();
        match ft_message {
            FtMessage::LockupCreate(lockup_create) => {
                let lockup = lockup_create.into_lockup(&sender_id);
                lockup.assert_new_valid(amount, &sender_id);
                let index = self.internal_add_lockup(&lockup);
                log!(
                    "Created new lockup for {} with index {}",
                    lockup.account_id.as_ref(),
                    index
                );
            }
            FtMessage::DraftGroupFunding(funding) => {
                let draft_group_id = funding.draft_group_id;
                let mut draft_group = self
                    .draft_groups
                    .get(&draft_group_id as _)
                    .expect("draft group not found");
                assert_eq!(
                    draft_group.total_amount, amount,
                    "The draft group total balance doesn't match the transferred balance",
                );
                draft_group.fund(&sender_id);
                self.draft_groups.insert(&draft_group_id as _, &draft_group);
                log!("Funded draft group {}", draft_group_id);
            }
        }

        PromiseOrValue::Value(0.into())
    }
}
