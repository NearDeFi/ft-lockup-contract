mod setup;

use crate::setup::*;

#[test]
fn test_lockup_claim_logic() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(10000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());
    let lockup_create = LockupCreate {
        account_id: users.alice.valid_account_id(),
        schedule: Schedule(vec![
            Checkpoint {
                timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC - 1,
                balance: 0,
            },
            Checkpoint {
                timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC,
                balance: amount,
            },
        ]),
        vesting_schedule: None,
    };
    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup_create).unwrap_json();
    assert_eq!(balance.0, amount);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // Claim attempt before unlock.
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, 0);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.claimed_balance, 0);

    // Set time to the first checkpoint.
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC - 1);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // Set time to the second checkpoint.
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, amount);

    // Attempt to claim. No storage deposit for Alice.
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, 0);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, amount);

    ft_storage_deposit(&users.alice, TOKEN_ID, &users.alice.account_id);

    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, 0);

    // Claim tokens.
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount);
    // User's lockups should be empty, since fully claimed.
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    // Manually checking the lockup by index
    let lockup = e.get_lockup(0);
    assert_eq!(lockup.claimed_balance, amount);
    assert_eq!(lockup.unclaimed_balance, 0);

    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, amount);
}

#[test]
fn test_lockup_linear() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());
    let lockup_create = LockupCreate {
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
        vesting_schedule: None,
    };
    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup_create).unwrap_json();
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

    // Remove storage from token to verify claim refund.
    // Note, this burns `amount / 3` tokens.
    storage_force_unregister(&users.alice, TOKEN_ID);
    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, 0);

    // Trying to claim, should fail and refund the amount back to the lockup
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, 0);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, amount / 3);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 6);

    // Claim again but with storage deposit
    ft_storage_deposit(&users.alice, TOKEN_ID, &users.alice.account_id);
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount / 6);
    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, amount / 6);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, amount / 2);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // 2/3 unlock
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2 / 3);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.claimed_balance, amount / 2);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 6);

    // Claim tokens
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount / 6);
    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, amount / 3);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.claimed_balance, amount * 2 / 3);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // Claim again with no unclaimed_balance
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, 0);
    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, amount / 3);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.claimed_balance, amount * 2 / 3);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // full unlock
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.claimed_balance, amount * 2 / 3);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 3);

    // Final claim
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount / 3);
    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, amount * 2 / 3);

    // User's lockups should be empty, since fully claimed.
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    // Manually checking the lockup by index
    let lockup = e.get_lockup(0);
    assert_eq!(lockup.claimed_balance, amount);
    assert_eq!(lockup.unclaimed_balance, 0);
}

#[test]
fn test_lockup_cliff_amazon() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());
    let lockup_create = LockupCreate {
        account_id: users.alice.valid_account_id(),
        schedule: Schedule(vec![
            Checkpoint {
                timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC - 1,
                balance: 0,
            },
            Checkpoint {
                timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC,
                balance: amount / 10,
            },
            Checkpoint {
                timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2,
                balance: 3 * amount / 10,
            },
            Checkpoint {
                timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 3,
                balance: 6 * amount / 10,
            },
            Checkpoint {
                timestamp: GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 4,
                balance: amount,
            },
        ]),
        vesting_schedule: None,
    };
    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup_create).unwrap_json();
    assert_eq!(balance.0, amount);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // 1/12 time. pre-cliff unlock
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC / 3);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);

    // 1/4 time. cliff unlock
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.total_balance, amount);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 10);

    // 3/8 time. cliff unlock + 1/2 of 2nd year.
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC + ONE_YEAR_SEC / 2);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.unclaimed_balance, 2 * amount / 10);

    // 1/2 time.
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.unclaimed_balance, 3 * amount / 10);

    // 1/2 + 1/12 time.
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2 + ONE_YEAR_SEC / 3);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.unclaimed_balance, 4 * amount / 10);

    // 1/2 + 2/12 time.
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 2 + ONE_YEAR_SEC * 2 / 3);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.unclaimed_balance, 5 * amount / 10);

    // 3/4 time.
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 3);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.unclaimed_balance, 6 * amount / 10);

    // 7/8 time.
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 3 + ONE_YEAR_SEC / 2);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.unclaimed_balance, 8 * amount / 10);

    // full unlock.
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 4);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.unclaimed_balance, amount);

    // after unlock.
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC * 5);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.unclaimed_balance, amount);

    // attempt to claim without storage.
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, 0);
    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, 0);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups[0].1.unclaimed_balance, amount);

    // Claim tokens
    ft_storage_deposit(&users.alice, TOKEN_ID, &users.alice.account_id);
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount);
    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, amount);

    // Check lockup after claim
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());
    let lockup = e.get_lockup(0);
    assert_eq!(lockup.claimed_balance, amount);
    assert_eq!(lockup.unclaimed_balance, 0);
}

#[test]
fn test_claim_specific_lockups_with_specific_amounts_success() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    let lockup_create = LockupCreate {
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
        vesting_schedule: None,
    };

    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup_create).unwrap_json();
    assert_eq!(balance.0, amount);
    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup_create).unwrap_json();
    assert_eq!(balance.0, amount);
    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup_create).unwrap_json();
    assert_eq!(balance.0, amount);

    // Set time to half unlock
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC / 2);
    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 3);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 2);
    assert_eq!(lockups[1].1.claimed_balance, 0);
    assert_eq!(lockups[1].1.unclaimed_balance, amount / 2);
    assert_eq!(lockups[2].1.claimed_balance, 0);
    assert_eq!(lockups[2].1.unclaimed_balance, amount / 2);

    ft_storage_deposit(&users.alice, TOKEN_ID, &users.alice.account_id);

    // CLAIM
    let res: WrappedBalance = e
        .claim_specific_lockups(
            &users.alice,
            &vec![(2, None), (1, Some((amount / 3).into()))],
        )
        .unwrap_json();
    assert_eq!(res.0, amount / 3 + amount / 2);

    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 3);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.unclaimed_balance, amount / 2);
    assert_eq!(lockups[1].1.claimed_balance, amount / 3);
    assert_eq!(lockups[1].1.unclaimed_balance, amount / 6);
    assert_eq!(lockups[2].1.claimed_balance, amount / 2);
    assert_eq!(lockups[2].1.unclaimed_balance, 0);

    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, amount / 3 + amount / 2);
}

#[test]
fn test_claim_specific_lockups_with_specific_amounts_fail() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    let lockup_create = LockupCreate {
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
        vesting_schedule: None,
    };

    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup_create).unwrap_json();
    assert_eq!(balance.0, amount);
    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup_create).unwrap_json();
    assert_eq!(balance.0, amount);

    ft_storage_deposit(&users.alice, TOKEN_ID, &users.alice.account_id);

    // Set time to half unlock
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC / 2);

    // CLAIM not existing lockup
    let res = e.claim_specific_lockups(&users.bob, &vec![(9, Some((amount / 3).into()))]);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("lockup not found for account"));

    // CLAIM by wrong user
    let res = e.claim_specific_lockups(
        &users.bob,
        &vec![
            (1, Some((amount / 3).into())),
            (0, Some((amount / 4).into())),
        ],
    );
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("lockup not found for account"));

    // CLAIM by wrong user without amount
    let res = e.claim_specific_lockups(&users.bob, &vec![(1, None)]);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("lockup not found for account"));

    // CLAIM with too big amount
    let res = e.claim_specific_lockups(
        &users.alice,
        &vec![
            (1, Some((amount * 2 / 3).into())),
            (0, Some((amount / 4).into())),
        ],
    );
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("too big claim_amount for lockup"));
}

#[test]
fn test_claim_specific_lockups_overflow() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(60000, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    let lockup_create = LockupCreate {
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
        vesting_schedule: None,
    };

    let balance: WrappedBalance = e.add_lockup(&e.owner, amount, &lockup_create).unwrap_json();
    assert_eq!(balance.0, amount);

    // Set time to half unlock
    e.set_time_sec(GENESIS_TIMESTAMP_SEC + ONE_YEAR_SEC / 2);

    ft_storage_deposit(&users.alice, TOKEN_ID, &users.alice.account_id);

    // claim part
    let res = e.claim_specific_lockups(&users.alice, &vec![(0, Some((amount / 4).into()))]);
    assert!(res.is_ok());
    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, amount / 4);

    // claim with overflow
    let res = e.claim_specific_lockups(&users.alice, &vec![(0, Some(u128::MAX.into()))]);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("attempt to add with overflow"));
}
