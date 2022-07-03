mod setup;

use crate::setup::*;

#[test]
fn test_lockup_terminate_with_timestamp_in_future() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    let res = e.add_to_deposit_whitelist(&e.owner, &users.eve.valid_account_id());
    assert!(res.is_ok());
    ft_storage_deposit(&e.owner, TOKEN_ID, &users.eve.account_id);
    e.ft_transfer(&e.owner, amount, &users.eve);

    let (lockup_schedule, vesting_schedule) = lockup_vesting_schedule(amount);
    let lockup_create = LockupCreate {
        account_id: users.alice.valid_account_id(),
        schedule: lockup_schedule,
        vesting_schedule: Some(VestingConditions::Schedule(vesting_schedule)),
    };

    let balance: WrappedBalance = e
        .add_lockup(&users.eve, amount, &lockup_create)
        .unwrap_json();
    assert_eq!(balance.0, amount);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    let lockup_index = lockups[0].0;

    // before_cliff, 0 vested
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC - 1);

    // try TERMINATE with past timestamp
    let res = e.terminate_with_timestamp(
        &e.owner,
        lockup_index,
        GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC - 1 - 1,
    );
    assert!(!res.is_ok(), "expected terminate in past to fail");
    assert!(format!("{:?}", res.status()).contains("expected termination_timestamp >= now"));

    // TERMINATE with future timestamp
    let res: WrappedBalance = e
        .terminate_with_timestamp(
            &e.owner,
            lockup_index,
            GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2,
        )
        .unwrap_json();
    assert_eq!(res.0, amount / 2);

    let terminator_balance = e.ft_balance_of(&users.eve);
    assert_eq!(terminator_balance, amount / 2);

    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount / 2);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // during release of remaining schedule
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2 + ONE_YEAR_SEC * 2 / 3);

    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 4);

    // end of remaining schedule
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 3 + ONE_YEAR_SEC / 3);
    ft_storage_deposit(&users.eve, TOKEN_ID, &users.alice.account_id);
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount / 2);
    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, amount / 2);

    // User's lockups should be empty, since fully claimed.
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    // Manually checking the lockup by index
    let lockup = e.get_lockup(0);
    assert_eq!(lockup.total_balance, amount / 2);
    assert_eq!(lockup.claimed_balance, amount / 2);
    assert_eq!(lockup.unclaimed_balance, 0);
}

#[test]
fn test_lockup_terminate_with_timestamp_in_future_no_storage_deposit() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    // adding another owner
    let res = e.add_to_deposit_whitelist(&e.owner, &users.eve.valid_account_id());
    assert!(res.is_ok());
    ft_storage_deposit(&e.owner, TOKEN_ID, &users.eve.account_id);
    e.ft_transfer(&e.owner, amount, &users.eve);

    let schedule = Schedule(vec![
        Checkpoint {
            timestamp: GENESIS_TIMESTAMP_SEC,
            balance: 0,
        },
        Checkpoint {
            timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC,
            balance: amount,
        },
    ]);

    let lockup_create = LockupCreate {
        account_id: users.alice.valid_account_id(),
        schedule: schedule.clone(),
        vesting_schedule: Some(VestingConditions::Schedule(schedule.clone())),
    };

    // create lockup succeeds
    let res = e.add_lockup(&users.eve, amount, &lockup_create);
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, amount);

    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    let lockup_index = lockups[0].0;

    storage_force_unregister(&users.eve, TOKEN_ID);

    // terminate with no storage deposit creates unlocked lockup
    let termination_call_timestamp = GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 1 / 3;
    let termination_effective_timestamp = GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2 / 3;
    e.set_time_sec(termination_call_timestamp);
    let res: WrappedBalance = e.terminate_with_timestamp(
        &users.eve,
        lockup_index,
        termination_effective_timestamp
    ).unwrap_json();
    assert_eq!(res.0, 0);
    let lockups = e.get_account_lockups(&users.eve);
    assert_eq!(lockups.len(), 1);
    let lockup = &lockups[0].1;
    assert_eq!(lockup.unclaimed_balance, amount / 3);
    assert_eq!(lockup.total_balance, amount / 3);
    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, 0);

    // checking schedule, must be unlocked since the moment of termination
    // starting checkpoint is preserved
    assert_eq!(lockup.schedule.0[0].balance, 0);
    assert_eq!(
        lockup.schedule.0[0].timestamp,
        termination_call_timestamp - 1,
        "expected refund finish first timestamp one second before the termination"
    );
    // finish checkpoint is termination timestamp
    assert_eq!(lockup.schedule.0[1].balance, amount / 3);
    assert_eq!(
        lockup.schedule.0[1].timestamp, // trimmed schedule
        termination_call_timestamp,
        "expected refund finish to be at termination timestamp"
    );
}
