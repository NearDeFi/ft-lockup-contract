#![allow(dead_code)]

use near_contract_standards::fungible_token::metadata::{FungibleTokenMetadata, FT_METADATA_SPEC};
pub use near_sdk::json_types::{Base58CryptoHash, ValidAccountId, WrappedBalance};
use near_sdk::serde_json::json;
use near_sdk::{env, serde_json, AccountId, Balance, Gas, Timestamp};
use near_sdk_sim::runtime::GenesisConfig;
use near_sdk_sim::{
    deploy, init_simulator, to_yocto, ContractAccount, ExecutionResult, UserAccount, ViewResult,
};

pub use ft_lockup::lockup::{Lockup, LockupIndex, BatchedUsers};
pub use ft_lockup::schedule::{Checkpoint, Schedule};
pub use ft_lockup::termination::{HashOrSchedule, TerminationConfig};
use ft_lockup::view::LockupView;
pub use ft_lockup::{ContractContract as FtLockupContract, TimestampSec};

near_sdk_sim::lazy_static_include::lazy_static_include_bytes! {
    FT_LOCKUP_WASM_BYTES => "res/ft_lockup.wasm",
    FUNGIBLE_TOKEN_WASM_BYTES => "res/fungible_token.wasm",
}

pub const ONE_DAY_SEC: TimestampSec = 24 * 60 * 60;
pub const ONE_YEAR_SEC: TimestampSec = 365 * ONE_DAY_SEC;

pub const GENESIS_TIMESTAMP_SEC: TimestampSec = 1_600_000_000;

pub const NEAR: &str = "near";
pub const TOKEN_ID: &str = "token.near";
pub const FT_LOCKUP_ID: &str = "ft-lockup.near";
pub const OWNER_ID: &str = "owner.near";

pub const T_GAS: Gas = 10u64.pow(12);
pub const DEFAULT_GAS: Gas = 15 * T_GAS;
pub const MAX_GAS: Gas = 300 * T_GAS;
pub const CLAIM_GAS: Gas = 100 * T_GAS;
pub const TERMINATE_GAS: Gas = 100 * T_GAS;

pub const TOKEN_DECIMALS: u8 = 18;
pub const TOKEN_TOTAL_SUPPLY: Balance = d(1_000_000, TOKEN_DECIMALS);

pub struct Env {
    pub root: UserAccount,
    pub near: UserAccount,
    pub owner: UserAccount,
    pub contract: ContractAccount<FtLockupContract>,
    pub token: UserAccount,
}

pub struct Users {
    pub alice: UserAccount,
    pub bob: UserAccount,
    pub charlie: UserAccount,
    pub dude: UserAccount,
    pub eve: UserAccount,
}

pub fn lockup_vesting_schedule(amount: u128) -> (Schedule, Schedule) {
    let lockup_schedule = Schedule(vec![
        Checkpoint {
            timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2,
            balance: 0,
        },
        Checkpoint {
            timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 4,
            balance: amount * 3 / 4,
        },
        Checkpoint {
            timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 4 + 1,
            balance: amount,
        },
    ]);
    let vesting_schedule = Schedule(vec![
        Checkpoint {
            timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC - 1,
            balance: 0,
        },
        Checkpoint {
            timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC,
            balance: amount / 4,
        },
        Checkpoint {
            timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 4,
            balance: amount,
        },
    ]);
    (lockup_schedule, vesting_schedule)
}

pub fn lockup_vesting_schedule_2(amount: u128) -> (Schedule, Schedule) {
    let lockup_schedule = Schedule(vec![
        Checkpoint {
            timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2,
            balance: 0,
        },
        Checkpoint {
            timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 4,
            balance: amount * 3 / 4,
        },
        Checkpoint {
            timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 4 + 1,
            balance: amount,
        },
    ]);
    let vesting_schedule = Schedule(vec![
        Checkpoint {
            timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC - 1,
            balance: 0,
        },
        Checkpoint {
            timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC,
            balance: amount / 4,
        },
        Checkpoint {
            timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 4,
            balance: amount,
        },
    ]);
    (lockup_schedule, vesting_schedule)
}

pub fn storage_deposit(
    user: &UserAccount,
    contract_id: &str,
    account_id: &str,
    attached_deposit: Balance,
) {
    user.call(
        contract_id.to_string(),
        "storage_deposit",
        &json!({ "account_id": account_id }).to_string().into_bytes(),
        DEFAULT_GAS,
        attached_deposit,
    )
    .assert_success();
}

pub fn storage_force_unregister(user: &UserAccount, contract_id: &str) {
    user.call(
        contract_id.to_string(),
        "storage_unregister",
        &json!({ "force": true }).to_string().into_bytes(),
        DEFAULT_GAS,
        1,
    )
    .assert_success();
}

pub fn ft_storage_deposit(user: &UserAccount, token_account_id: &str, account_id: &str) {
    storage_deposit(
        user,
        token_account_id,
        account_id,
        125 * env::STORAGE_PRICE_PER_BYTE,
    );
}

pub fn to_nano(timestamp: u32) -> Timestamp {
    Timestamp::from(timestamp) * 10u64.pow(9)
}

impl Env {
    pub fn init(deposit_whitelist: Option<Vec<ValidAccountId>>) -> Self {
        let mut genesis_config = GenesisConfig::default();
        genesis_config.block_prod_time = 0;
        let root = init_simulator(Some(genesis_config));
        let near = root.create_user(NEAR.to_string(), to_yocto("1000000"));
        let owner = near.create_user(OWNER_ID.to_string(), to_yocto("10000"));

        let token = near.deploy_and_init(
            &FUNGIBLE_TOKEN_WASM_BYTES,
            TOKEN_ID.to_string(),
            "new",
            &json!({
                "owner_id": owner.valid_account_id(),
                "total_supply": WrappedBalance::from(TOKEN_TOTAL_SUPPLY),
                "metadata": FungibleTokenMetadata {
                    spec: FT_METADATA_SPEC.to_string(),
                    name: "Token".to_string(),
                    symbol: "TOKEN".to_string(),
                    icon: None,
                    reference: None,
                    reference_hash: None,
                    decimals: TOKEN_DECIMALS,
                }
            })
            .to_string()
            .into_bytes(),
            to_yocto("10"),
            DEFAULT_GAS,
        );

        let contract = deploy!(
            contract: FtLockupContract,
            contract_id: FT_LOCKUP_ID.to_string(),
            bytes: &FT_LOCKUP_WASM_BYTES,
            signer_account: near,
            deposit: to_yocto("10"),
            gas: DEFAULT_GAS,
            init_method: new(
                token.valid_account_id(),
                deposit_whitelist.unwrap_or_else(|| vec![owner.valid_account_id()])
            )
        );

        ft_storage_deposit(&owner, TOKEN_ID, FT_LOCKUP_ID);

        Self {
            root,
            near,
            owner,
            contract,
            token,
        }
    }

    pub fn ft_transfer_call(
        &self,
        user: &UserAccount,
        amount: Balance,
        msg: &str,
    ) -> ExecutionResult {
        user.call(
            self.token.account_id.clone(),
            "ft_transfer_call",
            &json!({
                "receiver_id": self.contract.user_account.valid_account_id(),
                "amount": WrappedBalance::from(amount),
                "msg": msg,
            })
            .to_string()
            .into_bytes(),
            MAX_GAS,
            1,
        )
    }

    pub fn add_lockup(
        &self,
        user: &UserAccount,
        amount: Balance,
        lockup: &Lockup,
    ) -> ExecutionResult {
        self.ft_transfer_call(user, amount, &serde_json::to_string(lockup).unwrap())
    }

    pub fn add_batched_lockup(
        &self,
        user: &UserAccount,
        amount: Balance,
        batch: &BatchedUsers,
    ) -> ExecutionResult {
        self.ft_transfer_call(user, amount, &serde_json::to_string(batch).unwrap())
    }

    pub fn claim(&self, user: &UserAccount) -> ExecutionResult {
        user.function_call(self.contract.contract.claim(), CLAIM_GAS, 0)
    }

    pub fn terminate(&self, user: &UserAccount, lockup_index: LockupIndex) -> ExecutionResult {
        user.function_call(
            self.contract.contract.terminate(lockup_index, None),
            TERMINATE_GAS,
            0,
        )
    }

    pub fn terminate_with_schedule(
        &self,
        user: &UserAccount,
        lockup_index: LockupIndex,
        hashed_schedule: Schedule,
    ) -> ExecutionResult {
        user.function_call(
            self.contract
                .contract
                .terminate(lockup_index, Some(hashed_schedule)),
            TERMINATE_GAS,
            0,
        )
    }

    pub fn remove_from_deposit_whitelist(
        &self,
        user: &UserAccount,
        account_id: &ValidAccountId,
    ) -> ExecutionResult {
        user.function_call(
            self.contract
                .contract
                .remove_from_deposit_whitelist(account_id.clone()),
            DEFAULT_GAS,
            1,
        )
    }

    pub fn add_to_deposit_whitelist(
        &self,
        user: &UserAccount,
        account_id: &ValidAccountId,
    ) -> ExecutionResult {
        user.function_call(
            self.contract
                .contract
                .add_to_deposit_whitelist(account_id.clone()),
            DEFAULT_GAS,
            1,
        )
    }

    pub fn get_num_lockups(&self) -> u32 {
        self.near
            .view_method_call(self.contract.contract.get_num_lockups())
            .unwrap_json()
    }

    pub fn get_lockups(&self, indices: &Vec<LockupIndex>) -> Vec<(LockupIndex, LockupView)> {
        self.near
            .view_method_call(self.contract.contract.get_lockups(indices.clone()))
            .unwrap_json()
    }

    pub fn get_lockups_paged(
        &self,
        from_index: Option<LockupIndex>,
        limit: Option<LockupIndex>,
    ) -> Vec<(LockupIndex, LockupView)> {
        self.near
            .view_method_call(self.contract.contract.get_lockups_paged(from_index, limit))
            .unwrap_json()
    }

    pub fn get_deposit_whitelist(&self) -> Vec<AccountId> {
        self.near
            .view_method_call(self.contract.contract.get_deposit_whitelist())
            .unwrap_json()
    }

    pub fn hash_schedule(&self, schedule: &Schedule) -> Base58CryptoHash {
        self.near
            .view_method_call(self.contract.contract.hash_schedule(schedule.clone()))
            .unwrap_json()
    }

    pub fn validate_schedule(
        &self,
        schedule: &Schedule,
        total_balance: WrappedBalance,
        termination_schedule: Option<&Schedule>,
    ) -> ViewResult {
        self.near
            .view_method_call(self.contract.contract.validate_schedule(
                schedule.clone(),
                total_balance,
                termination_schedule.map(|x| x.clone()),
            ))
    }

    pub fn get_token_account_id(&self) -> ValidAccountId {
        self.near
            .view_method_call(self.contract.contract.get_token_account_id())
            .unwrap_json()
    }

    pub fn get_account_lockups(&self, user: &UserAccount) -> Vec<(LockupIndex, LockupView)> {
        self.near
            .view_method_call(
                self.contract
                    .contract
                    .get_account_lockups(user.valid_account_id()),
            )
            .unwrap_json()
    }

    pub fn get_lockup(&self, lockup_index: LockupIndex) -> LockupView {
        let lockup: Option<LockupView> = self
            .near
            .view_method_call(self.contract.contract.get_lockup(lockup_index))
            .unwrap_json();
        lockup.unwrap()
    }

    pub fn ft_balance_of(&self, user: &UserAccount) -> Balance {
        let balance: WrappedBalance = self
            .near
            .view(
                self.token.account_id.clone(),
                "ft_balance_of",
                &json!({
                    "account_id": user.valid_account_id(),
                })
                .to_string()
                .into_bytes(),
            )
            .unwrap_json();
        balance.0
    }

    pub fn set_time_sec(&self, timestamp_sec: TimestampSec) {
        self.near.borrow_runtime_mut().cur_block.block_timestamp = to_nano(timestamp_sec);
    }
}

impl Users {
    pub fn init(e: &Env) -> Self {
        Self {
            alice: e
                .near
                .create_user("alice.near".to_string(), to_yocto("10000")),
            bob: e
                .near
                .create_user("bob.near".to_string(), to_yocto("10000")),
            charlie: e
                .near
                .create_user("charlie.near".to_string(), to_yocto("10000")),
            dude: e
                .near
                .create_user("dude.near".to_string(), to_yocto("10000")),
            eve: e
                .near
                .create_user("eve.near".to_string(), to_yocto("10000")),
        }
    }
}

pub const fn d(value: Balance, decimals: u8) -> Balance {
    value * 10u128.pow(decimals as _)
}
