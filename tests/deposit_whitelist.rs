mod setup;

use crate::setup::*;

#[test]
fn test_deposit_whitelist_get() {
    let e = Env::init(None);
    let users = Users::init(&e);
    let amount = d(1, TOKEN_DECIMALS);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let lockups = e.get_account_lockups(&users.alice);
    assert!(lockups.is_empty());

    // deposit whitelist has owner by default
    let deposit_whitelist = e.get_deposit_whitelist();
    assert_eq!(deposit_whitelist, vec![e.owner.account_id.clone()]);

    // user from whitelist can create lockups
    let lockup = Lockup {
        account_id: users.alice.valid_account_id(),
        schedule: Schedule(vec![
            Checkpoint {
                timestamp: 0,
                balance: 0,
            },
            Checkpoint {
                timestamp: 1,
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

    // user from whitelist can add other users
    let res = e.add_to_deposit_whitelist(&e.owner, &users.eve.valid_account_id());
    assert!(res.is_ok());

    let deposit_whitelist = e.get_deposit_whitelist();
    assert_eq!(
        deposit_whitelist,
        vec![e.owner.account_id.clone(), users.eve.account_id.clone()]
    );

    // user from whiltelist can remove other users
    let res = e.remove_from_deposit_whitelist(&users.eve, &e.owner.valid_account_id());
    assert!(res.is_ok());

    let deposit_whitelist = e.get_deposit_whitelist();
    assert_eq!(deposit_whitelist, vec![users.eve.account_id.clone()]);

    // user not from whitelist cannot add users
    let res = e.add_to_deposit_whitelist(&e.owner, &users.dude.valid_account_id());
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("Not in deposit whitelist"));

    // user not from whitelist cannot remove users
    let res = e.remove_from_deposit_whitelist(&e.owner, &users.eve.valid_account_id());
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("Not in deposit whitelist"));

    // user not in whitelist cannot create lockups
    let res = e.add_lockup(&e.owner, amount, &lockup);
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, 0);
    assert!(res.logs()[0].contains("Refund"));
    let lockups = e.get_account_lockups(&users.alice);
    // not increased
    assert_eq!(lockups.len(), 1);

    // user from whiltelist can remove itself from the list, even if it's the last user
    let res = e.remove_from_deposit_whitelist(&users.eve, &users.eve.valid_account_id());
    assert!(res.is_ok());
    let deposit_whitelist = e.get_deposit_whitelist();
    assert!(deposit_whitelist.is_empty());
}
