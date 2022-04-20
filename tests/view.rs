mod setup;

use crate::setup::*;

#[test]
fn test_hash_schedule() {
    let e = Env::init(None);
    let amount = d(60000, TOKEN_DECIMALS);
    let (lockup_schedule, vesting_schedule) = lockup_vesting_schedule_2(amount);
    assert_eq!(
        e.hash_schedule(&vesting_schedule),
        e.hash_schedule(&vesting_schedule)
    );
    assert_ne!(
        e.hash_schedule(&vesting_schedule),
        e.hash_schedule(&lockup_schedule),
    );
}

#[test]
fn test_validate_schedule() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    let (lockup_schedule, vesting_schedule) = lockup_vesting_schedule_2(amount);

    let res = e.validate_schedule(&lockup_schedule, amount.into(), Some(&vesting_schedule));
    assert!(res.is_ok());

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
    let res = e.validate_schedule(
        &lockup_schedule,
        amount.into(),
        Some(&incompatible_vesting_schedule),
    );
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.unwrap_err()).contains("The lockup schedule is ahead of"));
}

#[test]
fn test_get_lockups() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(1, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    // create some lockups
    for user in vec![&users.alice, &users.bob, &users.charlie] {
        let balance: WrappedBalance = e
            .add_lockup(
                &e.owner,
                amount,
                &Lockup::new_unlocked(user.account_id().clone(), amount),
            )
            .unwrap_json();
        assert_eq!(balance.0, amount);
    }

    // get_num_lockups
    let num_lockups = e.get_num_lockups();
    assert_eq!(num_lockups, 3);

    // get_lockups by indices
    let res = e.get_lockups(&vec![2, 0]);
    assert_eq!(res.len(), 2);
    assert_eq!(res[0].1.account_id, users.charlie.valid_account_id());
    assert_eq!(res[1].1.account_id, users.alice.valid_account_id());

    // get_lockups_paged from to
    let res = e.get_lockups_paged(Some(1), Some(2));
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].1.account_id, users.bob.valid_account_id());

    // get_lockups_paged from
    let res = e.get_lockups_paged(Some(1), None);
    assert_eq!(res.len(), 2);
    assert_eq!(res[0].1.account_id, users.bob.valid_account_id());
    assert_eq!(res[1].1.account_id, users.charlie.valid_account_id());

    // get_lockups_paged to
    let res = e.get_lockups_paged(None, Some(2));
    assert_eq!(res.len(), 2);
    assert_eq!(res[0].1.account_id, users.alice.valid_account_id());
    assert_eq!(res[1].1.account_id, users.bob.valid_account_id());

    // get_lockups_paged all
    let res = e.get_lockups_paged(None, None);
    assert_eq!(res.len(), 3);
    assert_eq!(res[0].1.account_id, users.alice.valid_account_id());
    assert_eq!(res[1].1.account_id, users.bob.valid_account_id());
    assert_eq!(res[2].1.account_id, users.charlie.valid_account_id());
}

#[test]
fn test_get_token_account_id() {
    let e = Env::init(None);

    let result = e.get_token_account_id();
    assert_eq!(result, e.token.valid_account_id());
}
