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

    let balance: WrappedBalance = e.add_lockup(&users.eve, amount, &lockup_create).unwrap_json();
    assert_eq!(balance.0, amount);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    let lockup_index = lockups[0].0;

    // before_cliff, 0 vested
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC - 1);

    // try TERMINATE with past timestamp
    let res = e.terminate_with_timestamp(
        &users.eve,
        lockup_index,
        GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC - 1 - 1,
    );
    assert!(!res.is_ok(), "expected terminate in past to fail");
    assert!(format!("{:?}", res.status()).contains("expected termination_timestamp >= now"));

    // TERMINATE with future timestamp
    let res: WrappedBalance = e
        .terminate_with_timestamp(
            &users.eve,
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
