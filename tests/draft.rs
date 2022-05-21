mod setup;

use crate::setup::*;

#[test]
fn test_create_draft_group() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    // create by not authorized account
    let res = e.create_draft_group(&users.alice);
    assert!(!res.is_ok(), "only deposit whitelist can create group");

    let res = e.create_draft_group(&e.owner);
    assert!(res.is_ok());
    let index: DraftGroupIndex = res.unwrap_json();
    assert_eq!(index, 0);

    let res = e.create_draft_group(&e.owner);
    assert!(res.is_ok());
    let index: DraftGroupIndex = res.unwrap_json();
    assert_eq!(index, 1);
}

#[test]
fn test_view_draft_groups() {
    let e = Env::init(None);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    e.create_draft_group(&e.owner);
    e.create_draft_group(&e.owner);
    e.create_draft_group(&e.owner);

    let result = e.get_draft_group(2);
    assert!(result.is_some());
    assert!(result.unwrap().draft_indices.is_empty());
    let result = e.get_draft_group(3);
    assert!(result.is_none());

    let result = e.get_draft_groups_paged(None, None);
    assert_eq!(result.len(), 3);
    assert_eq!(result[0].0, 0);
    assert_eq!(result[1].0, 1);
    assert_eq!(result[2].0, 2);

    let result = e.get_draft_groups_paged(Some(1), Some(2));
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, 1);
    assert!(result[0].1.draft_indices.is_empty());
}

#[test]
fn test_create_draft() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    let amount = d(60000, TOKEN_DECIMALS);
    let draft_group_id = 0;
    let draft = Draft {
        draft_group_id,
        lockup_create: LockupCreate::new_unlocked(users.alice.account_id.clone(), amount),
    };

    let res = e.create_draft(&e.owner, &draft);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("draft group not found"));

    e.create_draft_group(&e.owner);

    let res = e.create_draft(&users.alice, &draft);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("Not in deposit whitelist"));

    // create draft 0
    let res = e.create_draft(&e.owner, &draft);
    assert!(res.is_ok());
    let res: DraftGroupIndex = res.unwrap_json();
    assert_eq!(res, 0);

    // create draft 1
    let res = e.create_draft(&e.owner, &draft);
    assert!(res.is_ok());
    let res: DraftGroupIndex = res.unwrap_json();
    assert_eq!(res, 1);

    // check draft group
    let res = e.get_draft_group(0).unwrap();
    let mut draft_indices = res.draft_indices;
    draft_indices.sort();
    assert_eq!(draft_indices, vec![0, 1]);
    assert_eq!(res.total_amount, amount * 2);
}

#[test]
fn test_create_drafts_batch() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    let amount = d(60000, TOKEN_DECIMALS);
    let drafts: Vec<Draft> = vec![&users.alice, &users.bob]
        .iter()
        .map(|user| {
            let draft_group_id = 0;
            Draft {
                draft_group_id,
                lockup_create: LockupCreate::new_unlocked(user.account_id.clone(), amount),
            }
        })
        .collect();

    e.create_draft_group(&e.owner);

    let res = e.create_drafts(&e.owner, &drafts);
    assert!(res.is_ok());
    let ids: Vec<DraftIndex> = res.unwrap_json();
    assert_eq!(ids, vec![0, 1]);

    // check draft group
    let res = e.get_draft_group(0).unwrap();
    let mut draft_indices = res.draft_indices;
    draft_indices.sort();
    assert_eq!(draft_indices, vec![0, 1]);
    assert_eq!(res.total_amount, amount * 2);

    let draft = e.get_draft(0).unwrap();
    assert_eq!(draft.lockup.account_id, users.alice.valid_account_id());
    let draft = e.get_draft(1).unwrap();
    assert_eq!(draft.lockup.account_id, users.bob.valid_account_id());
}

#[test]
fn test_fund_draft_group() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    let amount = d(60000, TOKEN_DECIMALS);
    let draft_group_id = 0;
    let draft = Draft {
        draft_group_id,
        lockup_create: LockupCreate::new_unlocked(users.alice.account_id.clone(), amount),
    };

    e.create_draft_group(&e.owner);

    // create draft 0
    let res = e.create_draft(&e.owner, &draft);
    assert!(res.is_ok());
    // create draft 1
    let res = e.create_draft(&e.owner, &draft);
    assert!(res.is_ok());

    ft_storage_deposit(&e.owner, TOKEN_ID, &users.alice.account_id);
    e.ft_transfer(&e.owner, amount * 2, &users.alice);

    // fund with not authorized account
    let res = e.fund_draft_group(&users.alice, amount * 2, 0);
    assert!(res.logs()[0].contains("Refund"));
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, 0);

    // fund with wrong amount
    let res = e.fund_draft_group(&e.owner, amount, 0);
    assert!(res.logs()[0].contains("Refund"));
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, 0);

    // fund draft group
    let res = e.fund_draft_group(&e.owner, amount * 2, 0);
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, amount * 2);

    let res = e.get_draft_group(0).unwrap();
    assert_eq!(res.funded, true, "expected draft group to be funded");

    // fund again, should fail
    let res = e.fund_draft_group(&e.owner, amount * 2, 0);
    assert!(res.logs()[0].contains("Refund"));
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, 0);

    // add draft after funding
    let res = e.create_draft(&e.owner, &draft);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("group already funded"));
}

#[test]
fn test_convert_draft() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    let amount = d(60000, TOKEN_DECIMALS);
    let draft_group_id = 0;
    let draft = Draft {
        draft_group_id,
        lockup_create: LockupCreate::new_unlocked(users.alice.account_id.clone(), amount),
    };

    assert_eq!(e.get_next_draft_group_id(), 0);
    e.create_draft_group(&e.owner);
    assert_eq!(e.get_next_draft_group_id(), 1);
    e.create_draft_group(&e.owner);
    assert_eq!(e.get_next_draft_group_id(), 2);

    // create draft 0
    let res = e.create_draft(&e.owner, &draft);
    assert!(res.is_ok());
    // create draft 1
    let res = e.create_draft(&e.owner, &draft);
    assert!(res.is_ok());

    // try convert before fund
    let res = e.convert_draft(&users.bob, 0);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("not funded group"));

    // fund draft group
    let res = e.fund_draft_group(&e.owner, amount * 2, 0);
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, amount * 2);

    // convert by anonymous
    let res = e.convert_draft(&users.bob, 0);
    assert!(res.is_ok());
    let res: DraftIndex = res.unwrap_json();
    assert_eq!(res, 0);

    let res = e.get_draft(0);
    assert!(res.is_none(), "expected converted draft to be deleted");
    let res = e.get_draft_group(0).unwrap();
    assert_eq!(
        res.draft_indices,
        vec![1],
        "draft indices must be removed after convert"
    );
    assert_eq!(
        res.total_amount, amount,
        "draft amount must be subtracted from group"
    );

    let lockup = e.get_lockup(0);
    assert_eq!(lockup.account_id, users.alice.valid_account_id());
    assert_eq!(lockup.total_balance, amount);

    // try to convert again
    let res = e.convert_draft(&users.bob, 0);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("draft not found"));

    // converting second draft
    let res = e.convert_draft(&users.bob, 1);
    assert!(res.is_ok());

    assert_eq!(
        e.get_next_draft_group_id(), 2,
        "expected next_draft_group_id not changed after group remove",
    );

    // draft group must be deleted
    assert!(
        e.get_draft_group(0).is_none(),
        "draft group must be removed after convert",
    );
}

#[test]
fn test_convert_drafts_batch() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    let amount = d(60000, TOKEN_DECIMALS);
    let drafts: Vec<Draft> = vec![&users.alice, &users.bob]
        .iter()
        .map(|user| {
            let draft_group_id = 0;
            Draft {
                draft_group_id,
                lockup_create: LockupCreate::new_unlocked(user.account_id.clone(), amount),
            }
        })
        .collect();

    e.create_draft_group(&e.owner);

    let res = e.create_drafts(&e.owner, &drafts);
    assert!(res.is_ok());

    // fund draft group
    let res = e.fund_draft_group(&e.owner, amount * 2, 0);
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, amount * 2);

    // convert by anonymous
    let res = e.convert_drafts(&users.bob, &vec![0, 1]);
    println!("{:#?}", res);
    assert!(res.is_ok());
    let res: Vec<LockupIndex> = res.unwrap_json();
    assert_eq!(res, vec![0, 1]);

    let lockup = e.get_lockup(0);
    assert_eq!(lockup.account_id, users.alice.valid_account_id());

    let lockup = e.get_lockup(1);
    assert_eq!(lockup.account_id, users.bob.valid_account_id());
}

#[test]
fn test_view_drafts() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    let amount = d(60000, TOKEN_DECIMALS);
    let draft_group_id = 0;
    let draft = Draft {
        draft_group_id,
        lockup_create: LockupCreate::new_unlocked(users.alice.account_id.clone(), amount),
    };

    e.create_draft_group(&e.owner);
    e.create_draft(&e.owner, &draft);
    e.create_draft(&e.owner, &draft);
    e.create_draft(&e.owner, &draft);

    // fund draft group
    let res = e.fund_draft_group(&e.owner, amount * 3, 0);
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, amount * 3);
    let res = e.convert_draft(&users.bob, 0);
    assert!(res.is_ok());

    let res = e.get_drafts(vec![2, 0]);
    assert_eq!(res.len(), 1);
    assert_eq!(res[0].0, 2);
    let draft = &res[0].1;
    assert_eq!(draft.draft_group_id, 0);
    assert_eq!(draft.lockup.total_balance, amount);
}

#[test]
fn test_create_via_draft_batches_and_claim() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    let amount = d(60000, TOKEN_DECIMALS);
    let draft_group_id = 0;
    let draft = Draft {
        draft_group_id,
        lockup_create: LockupCreate::new_unlocked(users.alice.account_id.clone(), amount),
    };

    e.create_draft_group(&e.owner);
    e.create_drafts(&e.owner, &vec![draft]);

    // fund draft group
    let res = e.fund_draft_group(&e.owner, amount, 0);
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, amount);
    let res = e.convert_drafts(&users.bob, &vec![0]);
    assert!(res.is_ok());

    ft_storage_deposit(&users.alice, TOKEN_ID, &users.alice.account_id);
    let res: WrappedBalance = e.claim(&users.alice).unwrap_json();
    assert_eq!(res.0, amount);
    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, amount);
}

#[test]
fn test_draft_payer_update() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let amount = d(60000, TOKEN_DECIMALS);

    let res = e.add_to_deposit_whitelist(&e.owner, &users.eve.valid_account_id());
    assert!(res.is_ok());
    ft_storage_deposit(&e.owner, TOKEN_ID, &users.eve.account_id);

    let res = e.add_to_deposit_whitelist(&e.owner, &users.dude.valid_account_id());
    assert!(res.is_ok());
    ft_storage_deposit(&e.owner, TOKEN_ID, &users.dude.account_id);
    e.ft_transfer(&e.owner, amount, &users.dude);

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
        vesting_schedule: Some(HashOrSchedule::Schedule(schedule.clone())),
    };

    let res = e.create_draft_group(&e.owner);
    assert!(res.is_ok());
    let draft_group_id = 0;

    let draft = Draft {
        draft_group_id,
        lockup_create,
    };
    e.create_draft(&users.eve, &draft);

    // fund draft group
    let res = e.fund_draft_group(&users.dude, amount, 0);
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, amount);
    let res = e.convert_draft(&users.bob, 0);
    assert!(res.is_ok());
    let lockup_index: LockupIndex = res.unwrap_json();

    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    let lockup = &lockups[0].1;
    assert_eq!(
        lockup
            .termination_config
            .as_ref()
            .expect("expected termination_config")
            .beneficiary_id,
        users.dude.valid_account_id(),
        "expected beneficiary_id from draft group payer",
    );

    let res: WrappedBalance = e.terminate(&e.owner, lockup_index).unwrap_json();
    assert_eq!(res.0, amount);
    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, 0);
    let balance = e.ft_balance_of(&users.eve);
    assert_eq!(balance, 0);
    let balance = e.ft_balance_of(&users.dude);
    assert_eq!(balance, amount);
}
