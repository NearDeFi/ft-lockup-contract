use std::convert::TryInto;

use near_contract_standards::fungible_token::core_impl::ext_fungible_token;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::borsh::maybestd::collections::{HashMap, HashSet};
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedMap, UnorderedSet, Vector};
use near_sdk::json_types::{Base58CryptoHash, ValidAccountId, WrappedBalance, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    assert_one_yocto, env, ext_contract, is_promise_success, log, near_bindgen, serde_json,
    AccountId, Balance, BorshStorageKey, CryptoHash, Gas, PanicOnDefault, PromiseOrValue,
    Timestamp,
};

pub mod callbacks;
pub mod draft;
pub mod ft_token_receiver;
pub mod internal;
pub mod lockup;
pub mod schedule;
pub mod termination;
pub mod util;
pub mod view;

use crate::draft::*;
use crate::lockup::*;
use crate::schedule::*;
use crate::termination::*;
use crate::util::*;

near_sdk::setup_alloc!();

pub type TimestampSec = u32;
pub type TokenAccountId = AccountId;

const GAS_FOR_FT_TRANSFER: Gas = 15_000_000_000_000;
const GAS_FOR_AFTER_FT_TRANSFER: Gas = 20_000_000_000_000;

const ONE_YOCTO: Balance = 1;
const NO_DEPOSIT: Balance = 0;

uint::construct_uint! {
    pub struct U256(4);
}

#[ext_contract(ext_self)]
pub trait SelfCallbacks {
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
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    pub token_account_id: TokenAccountId,

    pub lockups: Vector<Lockup>,

    pub account_lockups: LookupMap<AccountId, HashSet<LockupIndex>>,

    /// account ids that can perform all actions:
    /// - manage operators_whitelist
    /// - manage drafts, draft_groups
    /// - create lockups, terminate lockups, fund draft_groups
    pub operators_whitelist: UnorderedSet<AccountId>,

    /// account ids that can perform all actions on drafts:
    /// - manage drafts, draft_groups
    pub draft_operators_whitelist: UnorderedSet<AccountId>,

    pub next_draft_id: DraftIndex,
    pub drafts: LookupMap<DraftIndex, Draft>,
    pub next_draft_group_id: DraftGroupIndex,
    pub draft_groups: UnorderedMap<DraftGroupIndex, DraftGroup>,
}

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Lockups,
    AccountLockups,
    OperatorsWhitelist,
    DraftOperatorsWhitelist,
    Drafts,
    DraftGroups,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(
        token_account_id: ValidAccountId,
        operators_whitelist: Vec<ValidAccountId>,
        draft_operators_whitelist: Option<Vec<ValidAccountId>>,
    ) -> Self {
        let mut operators_whitelist_set = UnorderedSet::new(StorageKey::OperatorsWhitelist);
        operators_whitelist_set.extend(operators_whitelist.into_iter().map(|a| a.into()));
        let mut draft_operators_whitelist_set =
            UnorderedSet::new(StorageKey::DraftOperatorsWhitelist);
        draft_operators_whitelist_set.extend(
            draft_operators_whitelist
                .unwrap_or(vec![])
                .into_iter()
                .map(|a| a.into()),
        );
        Self {
            lockups: Vector::new(StorageKey::Lockups),
            account_lockups: LookupMap::new(StorageKey::AccountLockups),
            token_account_id: token_account_id.into(),
            operators_whitelist: operators_whitelist_set,
            draft_operators_whitelist: draft_operators_whitelist_set,
            next_draft_id: 0,
            drafts: LookupMap::new(StorageKey::Drafts),
            next_draft_group_id: 0,
            draft_groups: UnorderedMap::new(StorageKey::DraftGroups),
        }
    }

    pub fn claim(
        &mut self,
        amounts: Option<Vec<(LockupIndex, Option<WrappedBalance>)>>,
    ) -> PromiseOrValue<WrappedBalance> {
        let account_id = env::predecessor_account_id();

        let (claim_amounts, mut lockups_by_id) = if let Some(amounts) = amounts {
            let lockups_by_id: HashMap<LockupIndex, Lockup> = self
                .internal_get_account_lockups_by_id(
                    &account_id,
                    &amounts.iter().map(|x| x.0).collect(),
                )
                .into_iter()
                .collect();
            let amounts: HashMap<LockupIndex, WrappedBalance> = amounts
                .into_iter()
                .map(|(lockup_id, amount)| {
                    (
                        lockup_id,
                        match amount {
                            Some(amount) => amount,
                            None => {
                                let lockup =
                                    lockups_by_id.get(&lockup_id).expect("lockup not found");
                                let unlocked_balance =
                                    lockup.schedule.unlocked_balance(current_timestamp_sec());
                                (unlocked_balance - lockup.claimed_balance).into()
                            }
                        },
                    )
                })
                .collect();
            (amounts, lockups_by_id)
        } else {
            let lockups_by_id: HashMap<LockupIndex, Lockup> = self
                .internal_get_account_lockups(&account_id)
                .into_iter()
                .collect();
            let amounts: HashMap<LockupIndex, WrappedBalance> = lockups_by_id
                .iter()
                .map(|(lockup_id, lockup)| {
                    let unlocked_balance =
                        lockup.schedule.unlocked_balance(current_timestamp_sec());
                    let amount: WrappedBalance = (unlocked_balance - lockup.claimed_balance).into();

                    (lockup_id.clone(), amount)
                })
                .collect();
            (amounts, lockups_by_id)
        };

        let account_id = env::predecessor_account_id();
        let mut lockup_claims = vec![];
        let mut total_claim_amount = 0;
        for (lockup_index, lockup_claim_amount) in claim_amounts {
            let lockup = lockups_by_id.get_mut(&lockup_index).unwrap();
            let lockup_claim = lockup.claim(lockup_index, lockup_claim_amount.0);

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

    #[payable]
    pub fn terminate(
        &mut self,
        lockup_index: LockupIndex,
        hashed_schedule: Option<Schedule>,
        termination_timestamp: Option<TimestampSec>,
    ) -> PromiseOrValue<WrappedBalance> {
        assert_one_yocto();
        let account_id = env::predecessor_account_id();
        let mut lockup = self
            .lockups
            .get(lockup_index as _)
            .expect("Lockup not found");
        let current_timestamp = current_timestamp_sec();
        let termination_timestamp = termination_timestamp.unwrap_or(current_timestamp);
        assert!(
            termination_timestamp >= current_timestamp,
            "expected termination_timestamp >= now",
        );
        let unvested_balance =
            lockup.terminate(&account_id, hashed_schedule, termination_timestamp);
        self.lockups.replace(lockup_index as _, &lockup);

        // no need to store empty lockup
        if lockup.schedule.total_balance() == 0 {
            let lockup_account_id: AccountId = lockup.account_id.into();
            let mut indices = self
                .account_lockups
                .get(&lockup_account_id)
                .unwrap_or_default();
            indices.remove(&lockup_index);
            self.internal_save_account_lockups(&lockup_account_id, indices);
        }

        if unvested_balance > 0 {
            ext_fungible_token::ft_transfer(
                account_id.clone(),
                unvested_balance.into(),
                Some(format!("Terminated lockup #{}", lockup_index)),
                &self.token_account_id,
                ONE_YOCTO,
                GAS_FOR_FT_TRANSFER,
            )
            .then(ext_self::after_lockup_termination(
                account_id,
                unvested_balance.into(),
                &env::current_account_id(),
                NO_DEPOSIT,
                GAS_FOR_AFTER_FT_TRANSFER,
            ))
            .into()
        } else {
            PromiseOrValue::Value(0.into())
        }
    }

    #[payable]
    pub fn add_to_operators_whitelist(&mut self, account_id: ValidAccountId) {
        assert_one_yocto();
        self.assert_operators_whitelist(&env::predecessor_account_id());
        self.operators_whitelist.insert(account_id.as_ref());
    }

    #[payable]
    pub fn remove_from_operators_whitelist(&mut self, account_id: ValidAccountId) {
        assert_one_yocto();
        self.assert_operators_whitelist(&env::predecessor_account_id());
        self.operators_whitelist.remove(account_id.as_ref());
    }

    #[payable]
    pub fn add_to_draft_operators_whitelist(&mut self, account_id: ValidAccountId) {
        assert_one_yocto();
        self.assert_operators_whitelist(&env::predecessor_account_id());
        self.draft_operators_whitelist.insert(account_id.as_ref());
    }

    #[payable]
    pub fn remove_from_draft_operators_whitelist(&mut self, account_id: ValidAccountId) {
        assert_one_yocto();
        self.assert_operators_whitelist(&env::predecessor_account_id());
        self.draft_operators_whitelist.remove(account_id.as_ref());
    }

    pub fn create_draft_group(&mut self) -> DraftGroupIndex {
        self.assert_draft_operators_whitelist(&env::predecessor_account_id());

        let index = self.next_draft_group_id;
        self.next_draft_group_id += 1;
        assert!(
            self.draft_groups
                .insert(&index, &DraftGroup::new())
                .is_none(),
            "Invariant"
        );

        index
    }

    pub fn create_draft(&mut self, draft: Draft) -> DraftIndex {
        self.create_drafts(vec![draft])[0]
    }

    pub fn create_drafts(&mut self, drafts: Vec<Draft>) -> Vec<DraftIndex> {
        self.assert_draft_operators_whitelist(&env::predecessor_account_id());
        let mut draft_group_lookup: HashMap<DraftGroupIndex, DraftGroup> = HashMap::new();
        let draft_ids: Vec<DraftIndex> = drafts
            .iter()
            .map(|draft| {
                let draft_group = draft_group_lookup
                    .entry(draft.draft_group_id)
                    .or_insert_with(|| {
                        self.draft_groups
                            .get(&draft.draft_group_id as _)
                            .expect("draft group not found")
                    });
                draft_group.assert_can_add_draft();
                draft.assert_new_valid();

                let index = self.next_draft_id;
                self.next_draft_id += 1;
                assert!(self.drafts.insert(&index, &draft).is_none(), "Invariant");
                draft_group.total_amount = draft_group
                    .total_amount
                    .checked_add(draft.total_balance())
                    .expect("attempt to add with overflow");
                draft_group.draft_indices.insert(index);

                index
            })
            .collect();

        draft_group_lookup
            .iter()
            .for_each(|(draft_group_id, draft_group)| {
                self.draft_groups.insert(&draft_group_id as _, &draft_group);
            });

        draft_ids
    }

    pub fn convert_draft(&mut self, draft_id: DraftIndex) -> LockupIndex {
        self.convert_drafts(vec![draft_id])[0]
    }

    pub fn convert_drafts(&mut self, draft_ids: Vec<DraftIndex>) -> Vec<LockupIndex> {
        let mut draft_group_lookup: HashMap<DraftGroupIndex, DraftGroup> = HashMap::new();
        let lockup_ids: Vec<LockupIndex> = draft_ids
            .iter()
            .map(|draft_id| {
                let draft = self.drafts.remove(&draft_id as _).expect("draft not found");
                let draft_group = draft_group_lookup
                    .entry(draft.draft_group_id)
                    .or_insert_with(|| {
                        self.draft_groups
                            .get(&draft.draft_group_id as _)
                            .expect("draft group not found")
                    });
                draft_group.assert_can_convert_draft();
                let payer_id = draft_group
                    .payer_id
                    .as_mut()
                    .expect("expected present payer_id");

                assert!(draft_group.draft_indices.remove(&draft_id), "Invariant");
                let amount = draft.total_balance();
                assert!(draft_group.total_amount >= amount, "Invariant");
                draft_group.total_amount -= amount;

                let lockup = draft.lockup_create.into_lockup(&payer_id);
                let index = self.internal_add_lockup(&lockup);
                log!(
                    "Created new lockup for {} with index {} from draft {}",
                    lockup.account_id.as_ref(),
                    index,
                    draft_id,
                );

                index
            })
            .collect();

        draft_group_lookup
            .iter()
            .for_each(|(draft_group_id, draft_group)| {
                if draft_group.draft_indices.is_empty() {
                    self.draft_groups.remove(&draft_group_id as _);
                } else {
                    self.draft_groups.insert(&draft_group_id as _, &draft_group);
                }
            });

        lockup_ids
    }

    pub fn discard_draft_group(&mut self, draft_group_id: DraftGroupIndex) {
        self.assert_draft_operators_whitelist(&env::predecessor_account_id());

        let mut draft_group = self
            .draft_groups
            .get(&draft_group_id as _)
            .expect("draft group not found");
        draft_group.discard();

        if draft_group.draft_indices.is_empty() {
            self.draft_groups.remove(&draft_group_id as _);
        } else {
            self.draft_groups.insert(&draft_group_id as _, &draft_group);
        }
    }

    pub fn delete_drafts(&mut self, draft_ids: Vec<DraftIndex>) {
        // no authorization required here since the draft group discard has been authorized
        let mut draft_group_lookup: HashMap<DraftGroupIndex, DraftGroup> = HashMap::new();
        for draft_id in &draft_ids {
            let draft = self.drafts.remove(&draft_id as _).expect("draft not found");
            let draft_group = draft_group_lookup
                .entry(draft.draft_group_id)
                .or_insert_with(|| {
                    self.draft_groups
                        .get(&draft.draft_group_id as _)
                        .expect("draft group not found")
                });

            draft_group.assert_can_delete_draft();
            let amount = draft.total_balance();
            assert!(draft_group.total_amount >= amount, "Invariant");
            draft_group.total_amount -= amount;

            assert!(draft_group.draft_indices.remove(draft_id), "Invariant");
        }

        for (draft_group_id, draft_group) in &draft_group_lookup {
            if draft_group.draft_indices.is_empty() {
                self.draft_groups.remove(&draft_group_id as _);
            } else {
                self.draft_groups.insert(&draft_group_id as _, &draft_group);
            }
        }
    }
}
