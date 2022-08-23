mod setup;

use crate::setup::*;

use ft_lockup::lockup::{BatchedUsers};
use near_sdk::json_types::WrappedBalance;
use near_sdk::json_types::U128;

const ONE_YEAR_SEC: TimestampSec = 365 * 30 * 24 * 60 * 60;

const TGE_TIMESTAMP: TimestampSec = 1663070400; // 2022-09-13T12:00:00 UTC
const FULL_UNLOCK_TIMESTAMP: TimestampSec = 1726228800; // 2024-09-13T12:00:00 UTC

#[test]
fn test_tge_user() {
    let env = Env::init(None);
    let users = Users::init(&env);

    let a = U128(d(60000, TOKEN_DECIMALS));
    let a_at_tge = U128(d(6000, TOKEN_DECIMALS));
    let b = U128(d(3000, TOKEN_DECIMALS));
    let b_at_tge = U128(d(300, TOKEN_DECIMALS));
    let c = U128(d(20, TOKEN_DECIMALS));
    let c_at_tge = U128(d(2, TOKEN_DECIMALS));


    // BEFORE TGE
    env.set_time_sec(TGE_TIMESTAMP - 1);

    let lockups = env.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    let arr = vec![
        (users.alice.valid_account_id(), a),
        (users.bob.valid_account_id(), b),
        (users.charlie.valid_account_id(), c)
    ];
    let batch = BatchedUsers { batch: arr };
    let balance: WrappedBalance = env
        .add_batched_lockup(&env.owner, a.0 + b.0 + c.0, &batch)
        .unwrap_json();
    assert_eq!(balance.0, a.0 + b.0 + c.0);

    let lockups = env.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    assert_eq!(lockups[0].1.total_balance, a.0);

    let lockups = env.get_account_lockups(&users.bob);
    assert_eq!(lockups.len(), 1);
    assert_eq!(lockups[0].1.total_balance, b.0);

    let lockups = env.get_account_lockups(&users.charlie);
    assert_eq!(lockups.len(), 1);
    assert_eq!(lockups[0].1.total_balance, c.0);

    // TGE
    env.set_time_sec(TGE_TIMESTAMP);
    ft_storage_deposit(&users.alice, TOKEN_ID, &users.alice.account_id);
    ft_storage_deposit(&users.bob, TOKEN_ID, &users.bob.account_id);
    ft_storage_deposit(&users.charlie, TOKEN_ID, &users.charlie.account_id);

    // alice claims at tge
    let lockups = env.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    println!("{:?}", lockups[0].1);
    assert_eq!(lockups[0].1.unclaimed_balance, a_at_tge.0);
    assert_eq!(lockups[0].1.total_balance, a.0);
    let res: WrappedBalance = env.claim(&users.alice).unwrap_json();
    let lockups = env.get_account_lockups(&users.alice);
    assert_eq!(res.0, a_at_tge.0);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);
    assert_eq!(lockups[0].1.claimed_balance, a_at_tge.0);
    assert_eq!(lockups[0].1.total_balance, a.0);

    // bob claims at tge
    let lockups = env.get_account_lockups(&users.bob);
    assert_eq!(lockups.len(), 1);
    assert_eq!(lockups[0].1.unclaimed_balance, b_at_tge.0);
    assert_eq!(lockups[0].1.total_balance, b.0);
    let res: WrappedBalance = env.claim(&users.bob).unwrap_json();
    let lockups = env.get_account_lockups(&users.bob);
    assert_eq!(res.0, b_at_tge.0);
    assert_eq!(lockups[0].1.unclaimed_balance, 0);
    assert_eq!(lockups[0].1.claimed_balance, b_at_tge.0);
    assert_eq!(lockups[0].1.total_balance, b.0);


    // charlie doesn't claim at tge
    let lockups = env.get_account_lockups(&users.charlie);
    assert_eq!(lockups.len(), 1);
    assert_eq!(lockups[0].1.unclaimed_balance, c_at_tge.0);
    assert_eq!(lockups[0].1.total_balance, c.0);

    // AFTER LOCKUP PERIOD PASSED
    env.set_time_sec(FULL_UNLOCK_TIMESTAMP);

    // Alice claims the remaining
    let lockups = env.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    assert_eq!(lockups[0].1.unclaimed_balance, a.0 - a_at_tge.0);
    assert_eq!(lockups[0].1.claimed_balance, a_at_tge.0);
    assert_eq!(lockups[0].1.total_balance, a.0);
    let res: WrappedBalance = env.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, a.0 - a_at_tge.0);
    assert!(env.get_account_lockups(&users.alice).is_empty());
    assert_eq!(env.ft_balance_of(&users.alice), a.0);

    // Bob claims the remaining
    let lockups = env.get_account_lockups(&users.bob);
    assert_eq!(lockups.len(), 1);
    assert_eq!(lockups[0].1.unclaimed_balance, b.0 - b_at_tge.0);
    assert_eq!(lockups[0].1.claimed_balance, b_at_tge.0);
    assert_eq!(lockups[0].1.total_balance, b.0);
    let res: WrappedBalance = env.claim(&users.bob).unwrap_json();
    assert_eq!(res.0, b.0 - b_at_tge.0);
    assert!(env.get_account_lockups(&users.bob).is_empty());
    assert_eq!(env.ft_balance_of(&users.bob), b.0);

    // Charlie doesn't claim
    let lockups = env.get_account_lockups(&users.charlie);
    assert_eq!(lockups.len(), 1);
    assert_eq!(lockups[0].1.unclaimed_balance, c.0);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.total_balance, c.0);
    assert_eq!(env.ft_balance_of(&users.charlie), 0);

    // Long after lockup period:
    env.set_time_sec(FULL_UNLOCK_TIMESTAMP + ONE_YEAR_SEC);

    // Alice and Bob have empty lockups, since all is claimed
    assert!(env.get_account_lockups(&users.alice).is_empty());
    assert_eq!(env.ft_balance_of(&users.alice), a.0);
    assert!(env.get_account_lockups(&users.bob).is_empty());
    assert_eq!(env.ft_balance_of(&users.bob), b.0);

    // Charlie claims the remaining tokens
    let lockups = env.get_account_lockups(&users.charlie);
    assert_eq!(lockups.len(), 1);
    assert_eq!(lockups[0].1.unclaimed_balance, c.0);
    assert_eq!(lockups[0].1.claimed_balance, 0);
    assert_eq!(lockups[0].1.total_balance, c.0);
    let res: WrappedBalance = env.claim(&users.charlie).unwrap_json();
    assert_eq!(res.0, c.0);
    assert!(env.get_account_lockups(&users.charlie).is_empty());
    assert_eq!(env.ft_balance_of(&users.charlie), c.0);
}
