use crate::*;

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
pub struct DraftGroupFunding {
    pub draft_group_id: DraftGroupIndex,
    // use remaining gas to try converting drafts
    pub try_convert: Option<bool>,
}

#[derive(Serialize, Deserialize)]
#[serde(crate = "near_sdk::serde")]
#[serde(untagged)]
pub enum FtMessage {
    LockupCreate(LockupCreate),
    DraftGroupFunding(DraftGroupFunding),
    BatchedUsers(BatchedUsers),
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
            FtMessage::BatchedUsers(batched_users) => {
                let mut sum: u128 = 0;
                let termination_config = batched_users
                    .beneficiary_id
                    .map(|value| TerminationConfig {
                        beneficiary_id: value,
                        vesting_schedule: VestingConditions::SameAsLockupSchedule,
                    });

                for (account_id, sweat) in batched_users.batch {
                    let account_total = sweat.0;
                    sum = sum + account_total;

                    let user_lockup = Lockup {
                        account_id,
                        schedule: Schedule::new_on_tge(account_total),
                        claimed_balance: 0,
                        termination_config: termination_config.clone(),
                    };
                    user_lockup.assert_new_valid(account_total);
                    let _index = self.internal_add_lockup(&user_lockup);
                }
                assert_eq!(amount, sum);
            }
            FtMessage::LockupCreate(lockup_create) => {
                let lockup = lockup_create.into_lockup(&sender_id);
                lockup.assert_new_valid(amount);
                let index = self.internal_add_lockup(&lockup);
                log!(
                    "Created new lockup for {} with index {}",
                    lockup.account_id.as_ref(),
                    index
                );
                let event: FtLockupCreateLockup = (index, lockup, None).into();
                emit(EventKind::FtLockupCreateLockup(vec![event]));
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

                if funding.try_convert.unwrap_or(false) {
                    // Using remaining gas to try convert drafts, not waiting for results
                    if let Some(remaining_gas) =
                        env::prepaid_gas().checked_sub(env::used_gas() + GAS_EXT_CALL_COST)
                    {
                        if remaining_gas > GAS_MIN_FOR_CONVERT {
                            ext_self::convert_drafts(
                                draft_group.draft_indices.into_iter().collect(),
                                &env::current_account_id(),
                                NO_DEPOSIT,
                                remaining_gas,
                            );
                        }
                    }
                }
                let event = FtLockupFundDraftGroup {
                    id: draft_group_id,
                    amount: amount.into(),
                };
                emit(EventKind::FtLockupFundDraftGroup(vec![event]));
            }
        }

        PromiseOrValue::Value(0.into())
    }
}
