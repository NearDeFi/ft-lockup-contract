mod setup;

use crate::setup::*;

#[test]
fn test_lockup_terminate_no_vesting_schedule() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());
    let lockup = Lockup {
        account_id: users.alice.valid_account_id(),
        schedule: Schedule(vec![
            Checkpoint {
                timestamp: GENESIS_TIMESTAMP_SEC,
                balance: 0,
            },
            Checkpoint {
                timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC,
                balance: amount,
            },
        ]),
        claimed_balance: 0,
        termination_config: Some(TerminationConfig {
            terminator_id: users.eve.valid_account_id(),
            vesting_schedule: None,
        }),
    };

    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup).unwrap_json();
    assert_eq!(balance.0, amount);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // 1/3 unlock
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC / 3);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 3);

    // Claim tokens
    ft_storage_deposit(&users.alice, TOKEN_ID, &users.alice.account_id);
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount / 3);
    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, amount / 3);

    // Check lockup after claim
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, amount / 3);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // 1/2 unlock
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC / 2);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, amount / 3);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 6);

    let lockup_index = lockups[0].0;

    // TERMINATE
    ft_storage_deposit(&users.eve, TOKEN_ID, &users.eve.account_id);
    let res: WrappedBalance = e.terminate(&users.eve, lockup_index).unwrap_json();
    assert_eq!(res.0, amount / 2);

    let terminator_balance = e.ft_balance_of(&users.eve);
    assert_eq!(terminator_balance, amount / 2);

    // full unlock 2 / 3 period after termination before initial timestamp
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2 / 3);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.claimed_balance, amount / 3);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 6);

    // Final claim
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount / 6);
    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, amount / 2);

    // User's lockups should be empty, since fully claimed.
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    // Manually checking the lockup by index
    let lockup = e.get_lockup(0);
    assert_eq!(lockup.claimed_balance, amount / 2);
    assert_eq!(lockup.unclaimed_balance, 0);
}

#[test]
fn test_lockup_terminate_no_termination_config() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());
    let lockup = Lockup {
        account_id: users.alice.valid_account_id(),
        schedule: Schedule(vec![
            Checkpoint {
                timestamp: GENESIS_TIMESTAMP_SEC,
                balance: 0,
            },
            Checkpoint {
                timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC,
                balance: amount,
            },
        ]),
        claimed_balance: 0,
        termination_config: None,
    };

    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup).unwrap_json();
    assert_eq!(balance.0, amount);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    let lockup_index = lockups[0].0;
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // TERMINATE
    ft_storage_deposit(&users.eve, TOKEN_ID, &users.eve.account_id);
    let res = e.terminate(&users.eve, lockup_index);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("No termination config"));
}

#[test]
fn test_lockup_terminate_wrong_terminator() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());
    let lockup = Lockup {
        account_id: users.alice.valid_account_id(),
        schedule: Schedule(vec![
            Checkpoint {
                timestamp: GENESIS_TIMESTAMP_SEC,
                balance: 0,
            },
            Checkpoint {
                timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC,
                balance: amount,
            },
        ]),
        claimed_balance: 0,
        termination_config: Some(TerminationConfig {
            terminator_id: users.eve.valid_account_id(),
            vesting_schedule: None,
        }),
    };

    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup).unwrap_json();
    assert_eq!(balance.0, amount);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    let lockup_index = lockups[0].0;
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // TERMINATE
    ft_storage_deposit(&users.dude, TOKEN_ID, &users.dude.account_id);
    let res = e.terminate(&users.dude, lockup_index);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("Unauthorized"));
}

#[test]
fn test_lockup_terminate_with_no_token_storage_deposit() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    let lockup = Lockup {
        account_id: users.alice.valid_account_id(),
        schedule: Schedule(vec![
            Checkpoint {
                timestamp: GENESIS_TIMESTAMP_SEC,
                balance: 0,
            },
            Checkpoint {
                timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC,
                balance: amount,
            },
        ]),
        claimed_balance: 0,
        termination_config: Some(TerminationConfig {
            terminator_id: users.eve.valid_account_id(),
            vesting_schedule: None,
        }),
    };

    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup).unwrap_json();
    assert_eq!(balance.0, amount);
    let lockups = e.get_account_lockups(&users.alice);
    let lockup_index = lockups[0].0;

    // 1/3 unlock, terminate
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC / 3);
    // Claim tokens
    // TERMINATE, without deposit must create unlocked lockup for terminator
    let res: WrappedBalance = e.terminate(&users.eve, lockup_index).unwrap_json();
    assert_eq!(res.0, 0);

    ft_storage_deposit(&users.eve, TOKEN_ID, &users.eve.account_id);
    let terminator_balance = e.ft_balance_of(&users.eve);
    assert_eq!(terminator_balance, 0);

    // claiming balance from newly created lockup for terminator
    {
        let lockups = e.get_account_lockups(&users.eve);
        assert_eq!(lockups.len(), 1);
        assert_eq!(lockups[0].1.claimed_balance, 0);
        assert_eq!(lockups[0].1.unclaimed_balance, amount * 2 / 3);
        let terminator_lockup_index = lockups[0].0;

        // Claim from lockup refund
        let res: WrappedBalance = e.claim(&users.eve).unwrap_json();
        assert_eq!(res.0, amount * 2 / 3);
        let balance = e.ft_balance_of(&users.eve);
        assert_eq!(balance, amount * 2 / 3);

        // Terminator's lockups should be empty, since fully claimed.
        let lockups = e.get_account_lockups(&users.eve);
        assert!(lockups.is_empty());

        // Manually checking the terminator's lockup by index
        let lockup = e.get_lockup(terminator_lockup_index);
        assert_eq!(lockup.claimed_balance, amount * 2 / 3);
        assert_eq!(lockup.unclaimed_balance, 0);
    }

    // claiming vested balance for beneficiary
    {
        let lockups = e.get_account_lockups(&users.alice);
        assert_eq!(lockups.len(), 1);
        assert_eq!(lockups[0].1.claimed_balance, 0);
        assert_eq!(lockups[0].1.unclaimed_balance, amount / 3);

        // Claim by user
        ft_storage_deposit(&users.alice, TOKEN_ID, &users.alice.account_id);
        let balance = e.ft_balance_of(&users.alice);
        assert_eq!(balance, 0);

        let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
        assert_eq!(res.0, amount / 3);
        let balance = e.ft_balance_of(&users.alice);
        assert_eq!(balance, amount / 3);

        // User's lockups should be empty, since fully claimed.
        let lockups = e.get_account_lockups(&users.alice);
        assert!(lockups.is_empty());

        // Manually checking the terminator's lockup by index
        let lockup = e.get_lockup(lockup_index);
        assert_eq!(lockup.claimed_balance, amount / 3);
        assert_eq!(lockup.unclaimed_balance, 0);
    }
}

#[test]
fn test_lockup_terminate_custom_vesting_hash() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    let (lockup_schedule, vesting_schedule) = lockup_vesting_schedule(amount);
    let vesting_hash = e.hash_schedule(&vesting_schedule);
    let lockup = Lockup {
        account_id: users.alice.valid_account_id(),
        schedule: lockup_schedule,
        claimed_balance: 0,
        termination_config: Some(TerminationConfig {
            terminator_id: users.eve.valid_account_id(),
            vesting_schedule: Some(HashOrSchedule::Hash(vesting_hash)),
        }),
    };

    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup).unwrap_json();
    assert_eq!(balance.0, amount);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    let lockup_index = lockups[0].0;

    // 1Y, 1 / 4 vested, 0 unlocked
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // TERMINATE
    ft_storage_deposit(&users.eve, TOKEN_ID, &users.eve.account_id);
    let res: WrappedBalance = e
        .terminate_with_schedule(&users.eve, lockup_index, vesting_schedule)
        .unwrap_json();
    assert_eq!(res.0, amount * 3 / 4);
    let terminator_balance = e.ft_balance_of(&users.eve);
    assert_eq!(terminator_balance, amount * 3 / 4);

    // Checking lockup
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount / 4);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // Rewind to 2Y + Y * 2 / 3, 1/4 of original unlock, full vested unlock
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2 + ONE_YEAR_SEC * 2 / 3);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount / 4);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 4);

    // claiming
    ft_storage_deposit(&users.alice, TOKEN_ID, &users.alice.account_id);
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount / 4);

    // Checking lockups
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    // User lockups are empty
    let lockup = e.get_lockup(lockup_index);
    assert_eq!(lockup.total_balance, amount / 4);
    assert_eq!(lockup.claimed_balance, amount / 4);
    assert_eq!(lockup.unclaimed_balance, 0);
}

#[test]
fn test_lockup_terminate_custom_vesting_invalid_hash() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    let (lockup_schedule, vesting_schedule) = lockup_vesting_schedule(amount);
    let vesting_hash = e.hash_schedule(&vesting_schedule);
    let lockup = Lockup {
        account_id: users.alice.valid_account_id(),
        schedule: lockup_schedule,
        claimed_balance: 0,
        termination_config: Some(TerminationConfig {
            terminator_id: users.eve.valid_account_id(),
            vesting_schedule: Some(HashOrSchedule::Hash(vesting_hash)),
        }),
    };

    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup).unwrap_json();
    assert_eq!(balance.0, amount);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    let lockup_index = lockups[0].0;

    // 1Y, 1 / 4 vested, 0 unlocked
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // TERMINATE
    let fake_schedule = Schedule(vec![
        Checkpoint {
            timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2,
            balance: 0,
        },
        Checkpoint {
            timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 4,
            balance: amount,
        },
    ]);
    ft_storage_deposit(&users.eve, TOKEN_ID, &users.eve.account_id);
    let res = e.terminate_with_schedule(&users.eve, lockup_index, fake_schedule);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("The revealed schedule hash doesn't match"));
}

#[test]
fn test_lockup_terminate_custom_vesting_incompatible_vesting_schedule_by_hash() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    let (lockup_schedule, _vesting_schedule) = lockup_vesting_schedule(amount);
    let incompatible_vesting_schedule = Schedule(vec![
        Checkpoint {
            timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 4,
            balance: 0,
        },
        Checkpoint {
            timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 4 + 1,
            balance: amount,
        },
    ]);
    let incompatible_vesting_hash = e.hash_schedule(&incompatible_vesting_schedule);
    let lockup = Lockup {
        account_id: users.alice.valid_account_id(),
        schedule: lockup_schedule,
        claimed_balance: 0,
        termination_config: Some(TerminationConfig {
            terminator_id: users.eve.valid_account_id(),
            vesting_schedule: Some(HashOrSchedule::Hash(incompatible_vesting_hash)),
        }),
    };

    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup).unwrap_json();
    assert_eq!(balance.0, amount);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    let lockup_index = lockups[0].0;

    // 1Y, 1 / 4 vested, 0 unlocked
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // TERMINATE
    ft_storage_deposit(&users.eve, TOKEN_ID, &users.eve.account_id);
    let res = e.terminate_with_schedule(&users.eve, lockup_index, incompatible_vesting_schedule);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("The lockup schedule is ahead of"));
}

#[test]
fn test_lockup_terminate_custom_vesting_terminate_before_cliff() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    let (lockup_schedule, vesting_schedule) = lockup_vesting_schedule(amount);
    let lockup = Lockup {
        account_id: users.alice.valid_account_id(),
        schedule: lockup_schedule,
        claimed_balance: 0,
        termination_config: Some(TerminationConfig {
            terminator_id: users.eve.valid_account_id(),
            vesting_schedule: Some(HashOrSchedule::Schedule(vesting_schedule)),
        }),
    };

    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup).unwrap_json();
    assert_eq!(balance.0, amount);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    let lockup_index = lockups[0].0;

    // 1Y - 1 before cliff termination
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC - 1);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // TERMINATE
    ft_storage_deposit(&users.eve, TOKEN_ID, &users.eve.account_id);
    let res: WrappedBalance = e.terminate(&users.eve, lockup_index).unwrap_json();
    assert_eq!(res.0, amount);

    let terminator_balance = e.ft_balance_of(&users.eve);
    assert_eq!(terminator_balance, amount);

    // Checking lockup

    // after ALL the schedules have finished

    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    let lockup = e.get_lockup(lockup_index);
    assert_eq!(lockup.total_balance, 0);
    assert_eq!(lockup.claimed_balance, 0);
    assert_eq!(lockup.unclaimed_balance, 0);

    // Trying to claim
    ft_storage_deposit(&users.alice, TOKEN_ID, &users.alice.account_id);
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, 0);

    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, 0);
}

#[test]
fn test_lockup_terminate_custom_vesting_before_release() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    let (lockup_schedule, vesting_schedule) = lockup_vesting_schedule(amount);
    let lockup = Lockup {
        account_id: users.alice.valid_account_id(),
        schedule: lockup_schedule,
        claimed_balance: 0,
        termination_config: Some(TerminationConfig {
            terminator_id: users.eve.valid_account_id(),
            vesting_schedule: Some(HashOrSchedule::Schedule(vesting_schedule)),
        }),
    };

    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup).unwrap_json();
    assert_eq!(balance.0, amount);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    let lockup_index = lockups[0].0;

    // 1Y, 1 / 4 vested, 0 unlocked
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // TERMINATE
    ft_storage_deposit(&users.eve, TOKEN_ID, &users.eve.account_id);
    let res: WrappedBalance = e.terminate(&users.eve, lockup_index).unwrap_json();
    assert_eq!(res.0, amount * 3 / 4);
    let terminator_balance = e.ft_balance_of(&users.eve);
    assert_eq!(terminator_balance, amount * 3 / 4);

    // Checking lockup
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount / 4);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // Trying to claim
    ft_storage_deposit(&users.alice, TOKEN_ID, &users.alice.account_id);
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, 0);

    // Rewind to 2Y + Y/3, 1/8 of original should be unlocked
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2 + ONE_YEAR_SEC / 3);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount / 4);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 8);

    // claiming
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount / 8);

    // Rewind to 2Y + Y * 2 / 3, 1/4 of original unlock, full vested unlock
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2 + ONE_YEAR_SEC * 2 / 3);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount / 4);
    assert_eq!(lockups[0].1.claimed_balance, amount / 8);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 8);

    // claiming
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount / 8);

    // Checking lockups
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    // User lockups are empty
    let lockup = e.get_lockup(lockup_index);
    assert_eq!(lockup.total_balance, amount / 4);
    assert_eq!(lockup.claimed_balance, amount / 4);
    assert_eq!(lockup.unclaimed_balance, 0);
}

#[test]
fn test_lockup_terminate_custom_vesting_during_release() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    let (lockup_schedule, vesting_schedule) = lockup_vesting_schedule(amount);
    let lockup = Lockup {
        account_id: users.alice.valid_account_id(),
        schedule: lockup_schedule,
        claimed_balance: 0,
        termination_config: Some(TerminationConfig {
            terminator_id: users.eve.valid_account_id(),
            vesting_schedule: Some(HashOrSchedule::Schedule(vesting_schedule)),
        }),
    };

    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup).unwrap_json();
    assert_eq!(balance.0, amount);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    let lockup_index = lockups[0].0;

    // 2Y + Y / 3, 1/8 unlocked
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2 + ONE_YEAR_SEC / 3);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 8);

    // Trying to claim
    ft_storage_deposit(&users.alice, TOKEN_ID, &users.alice.account_id);
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount / 8);

    // TERMINATE, 2Y + Y / 2, 5/8 unlocked
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2 + ONE_YEAR_SEC / 2);
    ft_storage_deposit(&users.eve, TOKEN_ID, &users.eve.account_id);
    let res: WrappedBalance = e.terminate(&users.eve, lockup_index).unwrap_json();
    assert_eq!(res.0, amount * 3 / 8);
    let terminator_balance = e.ft_balance_of(&users.eve);
    assert_eq!(terminator_balance, amount * 3 / 8);

    // Checking lockup
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount * 5 / 8);
    assert_eq!(lockups[0].1.claimed_balance, amount / 8);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 16);

    // Rewind to 2Y + Y*2/3, 1/4 of original should be unlocked
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2 + ONE_YEAR_SEC * 2 / 3);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount * 5 / 8);
    assert_eq!(lockups[0].1.claimed_balance, amount / 8);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 8);

    // claiming
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount / 8);

    // Rewind to 3Y + Y * 2 / 3, 5/8 of original unlock, full vested unlock
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 3 + ONE_YEAR_SEC * 2 / 3);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount * 5 / 8);
    assert_eq!(lockups[0].1.claimed_balance, amount * 2 / 8);
    assert_eq!(lockups[0].1.unclaimed_balance, amount * 3 / 8);

    // claiming
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount * 3 / 8);

    // Checking lockups
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    // User lockups are empty
    let lockup = e.get_lockup(lockup_index);
    assert_eq!(lockup.total_balance, amount * 5 / 8);
    assert_eq!(lockup.claimed_balance, amount * 5 / 8);
    assert_eq!(lockup.unclaimed_balance, 0);
}

#[test]
fn test_lockup_terminate_custom_vesting_during_lockup_cliff() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    let (lockup_schedule, vesting_schedule) = lockup_vesting_schedule(amount);
    let lockup = Lockup {
        account_id: users.alice.valid_account_id(),
        schedule: lockup_schedule,
        claimed_balance: 0,
        termination_config: Some(TerminationConfig {
            terminator_id: users.eve.valid_account_id(),
            vesting_schedule: Some(HashOrSchedule::Schedule(vesting_schedule)),
        }),
    };

    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup).unwrap_json();
    assert_eq!(balance.0, amount);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    let lockup_index = lockups[0].0;

    // 2Y + Y * 2 / 3, 1/4 unlocked
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2 + ONE_YEAR_SEC * 2 / 3);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 4);

    // Trying to claim
    ft_storage_deposit(&users.alice, TOKEN_ID, &users.alice.account_id);
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount / 4);

    // TERMINATE, 3Y + Y / 3, 5/6 vested
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 3 + ONE_YEAR_SEC / 3);
    ft_storage_deposit(&users.eve, TOKEN_ID, &users.eve.account_id);
    let res: WrappedBalance = e.terminate(&users.eve, lockup_index).unwrap_json();
    assert_eq!(res.0, amount / 6);
    let terminator_balance = e.ft_balance_of(&users.eve);
    assert_eq!(terminator_balance, amount / 6);

    // Checking lockup
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount * 5 / 6);
    assert_eq!(lockups[0].1.claimed_balance, amount / 4);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 4);

    // claiming
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount / 4);

    // Rewind to 4Y
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 4);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount * 5 / 6);
    assert_eq!(lockups[0].1.claimed_balance, amount * 1 / 2);
    assert_eq!(lockups[0].1.unclaimed_balance, amount * 1 / 4);

    // Rewind to 4Y + 1, full unlock including part of cliff
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 4 + 1);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount * 5 / 6);
    assert_eq!(lockups[0].1.claimed_balance, amount * 1 / 2);
    assert_eq!(lockups[0].1.unclaimed_balance, amount * 1 / 3);

    // claiming
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount * 1 / 3);

    // Checking lockups
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    // User lockups are empty
    let lockup = e.get_lockup(lockup_index);
    assert_eq!(lockup.total_balance, amount * 5 / 6);
    assert_eq!(lockup.claimed_balance, amount * 5 / 6);
    assert_eq!(lockup.unclaimed_balance, 0);
}

#[test]
fn test_lockup_terminate_custom_vesting_after_vesting_finished() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    let (lockup_schedule, vesting_schedule) = lockup_vesting_schedule(amount);
    let lockup = Lockup {
        account_id: users.alice.valid_account_id(),
        schedule: lockup_schedule,
        claimed_balance: 0,
        termination_config: Some(TerminationConfig {
            terminator_id: users.eve.valid_account_id(),
            vesting_schedule: Some(HashOrSchedule::Schedule(vesting_schedule)),
        }),
    };

    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup).unwrap_json();
    assert_eq!(balance.0, amount);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    let lockup_index = lockups[0].0;

    // 2Y + Y * 2 / 3, 1/8 unlocked
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2 + ONE_YEAR_SEC * 2 / 3);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 4);

    // Trying to claim
    ft_storage_deposit(&users.alice, TOKEN_ID, &users.alice.account_id);
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount / 4);

    // TERMINATE, 4Y, fully vested
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 4);
    ft_storage_deposit(&users.eve, TOKEN_ID, &users.eve.account_id);
    let res: WrappedBalance = e.terminate(&users.eve, lockup_index).unwrap_json();
    assert_eq!(res.0, 0);
    let terminator_balance = e.ft_balance_of(&users.eve);
    assert_eq!(terminator_balance, 0);

    // Checking lockup
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, amount / 4);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 2);

    // claiming
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount / 2);

    // Rewind to 4Y + 1, full unlock including part of cliff
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 4 + 1);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, amount * 3 / 4);
    assert_eq!(lockups[0].1.unclaimed_balance, amount * 1 / 4);

    // Claiming
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount * 1 / 4);

    // Checking lockups
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    // Checking by index
    let lockup = e.get_lockup(lockup_index);
    assert_eq!(lockup.total_balance, amount);
    assert_eq!(lockup.claimed_balance, amount);
    assert_eq!(lockup.unclaimed_balance, 0);
}
