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
