use std::collections::HashMap;

use near_contract_standards::fungible_token::core_impl::ext_fungible_token;
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::borsh::maybestd::collections::HashSet;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{LookupMap, UnorderedSet, Vector};
use near_sdk::json_types::{Base58CryptoHash, ValidAccountId, WrappedBalance, U128};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    assert_one_yocto, env, ext_contract, is_promise_success, log, near_bindgen, serde_json,
    AccountId, Balance, BorshStorageKey, CryptoHash, Gas, PanicOnDefault, PromiseOrValue,
    Timestamp,
};

pub mod callbacks;
pub mod ft_token_receiver;
pub mod internal;
pub mod lockup;
pub mod schedule;
pub mod termination;
pub mod util;
pub mod view;

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

    /// Account IDs that can create new lockups.
    pub deposit_whitelist: UnorderedSet<AccountId>,
}

#[derive(BorshStorageKey, BorshSerialize)]
pub(crate) enum StorageKey {
    Lockups,
    AccountLockups,
    DepositWhitelist,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new(token_account_id: ValidAccountId, deposit_whitelist: Vec<ValidAccountId>) -> Self {
        let mut deposit_whitelist_set = UnorderedSet::new(StorageKey::DepositWhitelist);
        deposit_whitelist_set.extend(deposit_whitelist.into_iter().map(|a| a.into()));
        Self {
            lockups: Vector::new(StorageKey::Lockups),
            account_lockups: LookupMap::new(StorageKey::AccountLockups),
            token_account_id: token_account_id.into(),
            deposit_whitelist: deposit_whitelist_set,
        }
    }

    pub fn claim(
        &mut self,
        amounts: Option<Vec<(LockupIndex, WrappedBalance)>>,
    ) -> PromiseOrValue<WrappedBalance> {
        let account_id = env::predecessor_account_id();

        let (claim_amounts, mut lockups_by_id) = if let Some(amounts) = amounts {
            let amounts: HashMap<LockupIndex, WrappedBalance> = amounts.into_iter().collect();
            let lockups_by_id: HashMap<LockupIndex, Lockup> = self
                .internal_get_account_lockups_by_id(&account_id, &amounts.keys().cloned().collect())
                .into_iter()
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

    pub fn terminate(
        &mut self,
        lockup_index: LockupIndex,
        hashed_schedule: Option<Schedule>,
        termination_timestamp: Option<TimestampSec>,
    ) -> PromiseOrValue<WrappedBalance> {
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
    pub fn add_to_deposit_whitelist(&mut self, account_id: ValidAccountId) {
        assert_one_yocto();
        self.assert_deposit_whitelist(&env::predecessor_account_id());
        self.deposit_whitelist.insert(account_id.as_ref());
    }

    #[payable]
    pub fn remove_from_deposit_whitelist(&mut self, account_id: ValidAccountId) {
        assert_one_yocto();
        self.assert_deposit_whitelist(&env::predecessor_account_id());
        self.deposit_whitelist.remove(account_id.as_ref());
    }
}
