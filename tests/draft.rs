mod setup;

use crate::setup::*;

#[test]
fn test_create_draft_group() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    // create by not authorized account
    let res = e.create_draft_group(&users.alice);
    assert!(!res.is_ok(), "only draft_operator can create group");

    // owner can create draft group
    let res = e.create_draft_group(&e.owner);
    assert!(res.is_ok());
    let index: DraftGroupIndex = res.unwrap_json();
    assert_eq!(index, 0);

    // non-owner draft_operator can create draft group
    let res = e.create_draft_group(&e.draft_operator);
    assert!(res.is_ok());
    let index: DraftGroupIndex = res.unwrap_json();
    assert_eq!(index, 1);
}

#[test]
fn test_view_draft_groups() {
    let e = Env::init(None);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    e.create_draft_group(&e.draft_operator);
    e.create_draft_group(&e.draft_operator);
    e.create_draft_group(&e.draft_operator);

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

    let result = e.get_draft_groups_paged(Some(2), Some(5));
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].0, 2);

    assert!(e.get_draft_groups_paged(Some(1), Some(1)).is_empty());
    assert!(e.get_draft_groups_paged(Some(3), Some(1)).is_empty());
    assert!(e.get_draft_groups_paged(Some(4), Some(5)).is_empty());
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
        lockup_create: LockupCreate::new_unlocked(users.alice.valid_account_id(), amount),
    };

    let res = e.create_draft(&e.draft_operator, &draft);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("draft group not found"));

    e.create_draft_group(&e.owner);

    let res = e.create_draft(&users.alice, &draft);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("Not in draft operators whitelist"));

    // create draft 0 by draft_operator
    let res = e.create_draft(&e.draft_operator, &draft);
    assert!(res.is_ok());
    let res: DraftGroupIndex = res.unwrap_json();
    assert_eq!(res, 0);

    // create draft 1 by owner
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
fn test_create_draft_with_zero_amount_fails() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    let amount = d(0, TOKEN_DECIMALS);
    let draft_group_id = 0;
    let draft = Draft {
        draft_group_id,
        lockup_create: LockupCreate::new_unlocked(users.alice.valid_account_id(), amount),
    };

    assert!(e.create_draft_group(&e.owner).is_ok());

    // create draft with zero amount must fail
    let res = e.create_draft(&e.owner, &draft);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("expected total balance to be positive"));
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
                lockup_create: LockupCreate::new_unlocked(user.valid_account_id(), amount),
            }
        })
        .collect();

    e.create_draft_group(&e.draft_operator);

    let res = e.create_drafts(&e.draft_operator, &drafts);
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
    assert_eq!(
        draft.lockup_create.account_id,
        users.alice.valid_account_id()
    );
    let draft = e.get_draft(1).unwrap();
    assert_eq!(draft.lockup_create.account_id, users.bob.valid_account_id());
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
        lockup_create: LockupCreate::new_unlocked(users.alice.valid_account_id(), amount),
    };

    e.create_draft_group(&e.draft_operator);

    // create draft 0
    let res = e.create_draft(&e.draft_operator, &draft);
    assert!(res.is_ok());
    // create draft 1
    let res = e.create_draft(&e.draft_operator, &draft);
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

    // fund draft group by owner should succeed
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
    let res = e.create_draft(&e.draft_operator, &draft);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("group already funded"));
}

#[test]
fn test_fund_draft_group_with_convert() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    let amount = d(60000, TOKEN_DECIMALS);
    let draft_group_id = 0;
    let draft = Draft {
        draft_group_id,
        lockup_create: LockupCreate::new_unlocked(users.alice.valid_account_id(), amount),
    };

    e.create_draft_group(&e.owner);

    // create draft 0
    let res = e.create_draft(&e.owner, &draft);
    assert!(res.is_ok());

    // fund draft group
    let res = e.fund_draft_group_with_convert(&e.owner, amount, 0);
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, amount);

    let res = e.get_draft_group(0);
    assert!(res.is_none(), "expected draft group to be removed");

    // draft should have been converted to lockup
    let res = e.get_lockups_paged(None, None);
    assert_eq!(res.len(), 1);
    // draft should have been converted to lockup
    let res = e.get_draft(0);
    assert!(res.is_none(), "expected draft to be converted");
    let res = e.get_draft_groups_paged(None, None);
    assert_eq!(res.len(), 0, "expected draft group to be removed");
}

#[test]
fn test_fund_draft_group_with_convert_too_big_group() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    let amount = d(600, TOKEN_DECIMALS);
    let draft_group_id = 0;
    let draft = Draft {
        draft_group_id,
        lockup_create: LockupCreate::new_unlocked(users.alice.valid_account_id(), amount),
    };

    e.create_draft_group(&e.owner);

    let n_drafts = 100;
    // intentionally create too big draft group to convert with restricted gas
    let drafts: Vec<Draft> = iter::repeat(draft).take(n_drafts).collect();

    // create draft 0
    let res = e.create_drafts(&e.owner, &drafts);
    assert!(res.is_ok());

    // fund draft group
    let res = e.fund_draft_group_with_convert(&e.owner, amount * (n_drafts as Balance), 0);
    let balance: WrappedBalance = res.unwrap_json();
    // draft group has been converted since ft_transfer_call succeeds
    assert_eq!(balance.0, amount * (n_drafts as Balance));

    // but the draft group tried to be converted and failed
    let res = e.get_draft_group(0);
    assert!(res.is_some(), "expected draft group to not be removed");
    let res: DraftGroupView = res.unwrap();
    assert!(res.funded, "expected draft group to be funded");

    // lockups should not have been created
    let res = e.get_lockups_paged(None, None);
    assert_eq!(res.len(), 0);
    // draft should not have been converted to lockup
    let res = e.get_draft(0);
    assert!(res.is_some(), "expected draft not to be converted");
    let res = e.get_draft_groups_paged(None, None);
    assert_eq!(res.len(), 1, "expected draft group not to be removed");
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
        lockup_create: LockupCreate::new_unlocked(users.alice.valid_account_id(), amount),
    };

    assert_eq!(e.get_next_draft_group_id(), 0);
    assert_eq!(e.get_num_draft_groups(), 0);
    e.create_draft_group(&e.draft_operator);
    assert_eq!(e.get_next_draft_group_id(), 1);
    assert_eq!(e.get_num_draft_groups(), 1);
    e.create_draft_group(&e.draft_operator);
    assert_eq!(e.get_next_draft_group_id(), 2);
    assert_eq!(e.get_num_draft_groups(), 2);

    assert_eq!(e.get_next_draft_id(), 0);
    // create draft 0
    let res = e.create_draft(&e.draft_operator, &draft);
    assert_eq!(e.get_next_draft_id(), 1);
    assert!(res.is_ok());
    // create draft 1
    let res = e.create_draft(&e.draft_operator, &draft);
    assert_eq!(e.get_next_draft_id(), 2);
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
    assert_eq!(
        e.get_next_draft_id(),
        2,
        "expected next_draft_id not changed after draft convert",
    );
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
    assert_eq!(
        e.get_next_draft_id(),
        2,
        "expected next_draft_id not changed after draft convert",
    );
    assert!(res.is_ok());

    assert_eq!(
        e.get_next_draft_group_id(),
        2,
        "expected next_draft_group_id not changed after group remove",
    );
    assert_eq!(
        e.get_num_draft_groups(),
        1,
        "expected num_draft_groups to decrease after group remove",
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

    let build_draft = |draft_group_id, user: &UserAccount| Draft {
        draft_group_id,
        lockup_create: LockupCreate::new_unlocked(user.valid_account_id(), amount),
    };

    let group_0: DraftGroupIndex = e.create_draft_group(&e.draft_operator).unwrap_json();
    let group_1: DraftGroupIndex = e.create_draft_group(&e.draft_operator).unwrap_json();

    let res = e.create_drafts(
        &e.draft_operator,
        &vec![
            build_draft(group_0, &users.alice),
            build_draft(group_0, &users.bob),
            build_draft(group_1, &users.charlie),
            build_draft(group_1, &users.dude),
        ],
    );
    assert!(res.is_ok());

    // fund draft group
    let res = e.fund_draft_group(&e.owner, amount * 2, group_0);
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, amount * 2);
    let res = e.fund_draft_group(&e.owner, amount * 2, group_1);
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, amount * 2);

    // convert by anonymous
    let res = e.convert_drafts(&users.eve, &vec![3, 0, 2, 1]);
    assert!(res.is_ok());
    let mut res: Vec<LockupIndex> = res.unwrap_json();
    res.sort();
    assert_eq!(res, vec![0, 1, 2, 3]);

    let lockups = e.get_lockups_paged(None, None);
    let mut account_ids: Vec<ValidAccountId> =
        lockups.into_iter().map(|x| x.1.account_id).collect();
    account_ids.sort();
    let expected: Vec<ValidAccountId> = vec![users.alice, users.bob, users.charlie, users.dude]
        .iter()
        .map(|x| x.valid_account_id())
        .collect();
    assert_eq!(account_ids, expected, "wrong set of receivers");
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
        lockup_create: LockupCreate::new_unlocked(users.alice.valid_account_id(), amount),
    };

    assert!(e.create_draft_group(&e.draft_operator).is_ok());
    assert!(e.create_draft(&e.draft_operator, &draft).is_ok());
    assert!(e.create_draft(&e.draft_operator, &draft).is_ok());
    assert!(e.create_draft(&e.draft_operator, &draft).is_ok());

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
    assert_eq!(draft.lockup_create.total_balance, amount);
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
        lockup_create: LockupCreate::new_unlocked(users.alice.valid_account_id(), amount),
    };

    e.create_draft_group(&e.draft_operator);
    e.create_drafts(&e.draft_operator, &vec![draft]);

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

    let res = e.create_draft_group(&e.draft_operator);
    assert!(res.is_ok());
    let draft_group_id = 0;

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
        "expected beneficiary_id from draft group payer_id",
    );

    // terminating as owner, unvested balance returns to the payer
    let res: WrappedBalance = e.terminate(&e.owner, lockup_index).unwrap_json();
    assert_eq!(res.0, amount);
    let balance = e.ft_balance_of(&users.alice);
    assert_eq!(balance, 0);
    let balance = e.ft_balance_of(&users.eve);
    assert_eq!(balance, 0);
    let balance = e.ft_balance_of(&users.dude);
    assert_eq!(balance, amount);
}

#[test]
fn test_delete_draft_group_before_add_drafts() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    let res = e.create_draft_group(&e.draft_operator);
    assert!(res.is_ok());
    let draft_group_id: DraftGroupIndex = res.unwrap_json();
    assert_eq!(draft_group_id, 0);

    // anonymous cannot discard draft group
    let res = e.discard_draft_group(&users.eve, draft_group_id);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("Not in draft operators whitelist"));

    // admin can discard empty draft group
    let res = e.discard_draft_group(&e.draft_operator, draft_group_id);
    assert!(res.is_ok());
    let res = e.get_draft_group(draft_group_id);
    assert!(
        res.is_none(),
        "expected discarded draft group to be removed"
    );
}

#[test]
fn test_delete_draft_group_before_fund() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    let res = e.create_draft_group(&e.draft_operator);
    assert!(res.is_ok());
    let draft_group_id: DraftGroupIndex = res.unwrap_json();
    assert_eq!(draft_group_id, 0);

    let res = e.get_draft_group(draft_group_id);
    assert!(res.is_some());
    let res = res.unwrap();
    assert!(!res.discarded, "expected draft group not to be discarded");

    let amount = d(60000, TOKEN_DECIMALS);
    let draft = Draft {
        draft_group_id,
        lockup_create: LockupCreate::new_unlocked(users.alice.valid_account_id(), amount),
    };

    // create draft 0
    let res = e.create_draft(&e.draft_operator, &draft);
    assert!(res.is_ok());
    let draft_id_0: DraftIndex = res.unwrap_json();
    assert_eq!(draft_id_0, 0);

    // create draft 1
    let res = e.create_draft(&e.draft_operator, &draft);
    assert!(res.is_ok());
    let draft_id_1: DraftIndex = res.unwrap_json();
    assert_eq!(draft_id_1, 1);

    let res = e.get_draft_group(draft_group_id).unwrap();
    assert_eq!(res.total_amount, amount * 2);

    // admin can discard non-empty draft group
    let res = e.discard_draft_group(&e.draft_operator, draft_group_id);
    assert!(res.is_ok());

    // draft group is not removed immediately
    let res = e.get_draft_group(draft_group_id);
    assert!(res.is_some());
    let res = res.unwrap();
    assert!(res.discarded, "expected draft group to be discarded");
    assert_eq!(res.total_amount, amount * 2);

    // admin cannot add drafts to the group
    let res = e.create_draft(&e.draft_operator, &draft);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("draft group is discarded"));

    // admin cannot fund the group
    let res = e.fund_draft_group(&e.owner, amount, draft_group_id);
    assert!(res.logs()[0].contains("Refund"));
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, 0);

    // cannot convert draft after discard
    let res = e.convert_draft(&users.bob, 0);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("draft group is discarded"));

    // anyone can delete draft after the group is discarded
    let res = e.delete_drafts(&users.eve, vec![draft_id_0]);
    assert!(res.is_ok());
    // first draft is removed
    let res = e.get_draft(draft_id_0);
    assert!(res.is_none(), "expected draft to be removed");
    let res = e.get_draft_group(draft_group_id).unwrap();
    assert_eq!(
        res.total_amount, amount,
        "expected total amount to decrease after draft delete"
    );

    // deleting last draft
    let res = e.delete_drafts(&users.eve, vec![draft_id_1]);
    assert!(res.is_ok());
    // last draft is removed
    let res = e.get_draft(draft_id_1);
    assert!(res.is_none(), "expected draft to be removed");
    // draft group is removed with last draft
    let res = e.get_draft_group(draft_group_id);
    assert!(
        res.is_none(),
        "expected discarded draft group to be removed"
    );
}

#[test]
fn test_delete_draft_group_after_fund() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    let res = e.create_draft_group(&e.draft_operator);
    assert!(res.is_ok());
    let draft_group_id: DraftGroupIndex = res.unwrap_json();
    assert_eq!(draft_group_id, 0);

    let amount = d(60000, TOKEN_DECIMALS);
    let draft = Draft {
        draft_group_id,
        lockup_create: LockupCreate::new_unlocked(users.alice.valid_account_id(), amount),
    };

    // create draft 0
    let res = e.create_draft(&e.draft_operator, &draft);
    assert!(res.is_ok());
    let res: DraftIndex = res.unwrap_json();
    assert_eq!(res, 0);

    // fund the group
    let res = e.fund_draft_group(&e.owner, amount, draft_group_id);
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, amount);

    // admin cannot discard non-empty draft group after it's converted
    let res = e.discard_draft_group(&e.draft_operator, draft_group_id);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("draft group already funded"));
}

#[test]
fn test_draft_operator_lockup_permissions() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    let amount = d(60000, TOKEN_DECIMALS);
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

    ft_storage_deposit(&e.owner, TOKEN_ID, &e.draft_operator.account_id);
    e.ft_transfer(&e.owner, amount, &e.draft_operator);

    // draft_operator cannot create drafts
    let res = e.add_lockup(&e.draft_operator, amount, &lockup_create);
    assert!(res.logs()[0].contains("Refund"));
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, 0);

    // create draft by owner
    let res = e.add_lockup(&e.owner, amount, &lockup_create);
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, amount);

    let lockups = e.get_account_lockups(&users.alice);
    assert_eq!(lockups.len(), 1);
    let lockup_index = lockups[0].0;

    // draft_operator cannot terminate drafts
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);
    let res = e.terminate(&e.draft_operator, lockup_index);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("Not in deposit whitelist"));

    let draft_group_id = 0;
    let draft = Draft {
        draft_group_id,
        lockup_create: LockupCreate::new_unlocked(users.alice.valid_account_id(), amount),
    };
    assert!(e.create_draft_group(&e.draft_operator).is_ok());
    let res = e.create_draft(&e.draft_operator, &draft);
    assert!(res.is_ok());

    // fund draft group by draft operator should fail
    let res = e.fund_draft_group(&e.draft_operator, amount, draft_group_id);
    assert!(res.logs()[0].contains("Refund"));
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, 0);
}

#[test]
fn test_draft_operator_permission_updates() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    // draft operator cannot control permissions for deposit
    let res = e.add_to_deposit_whitelist(&e.draft_operator, &e.owner.valid_account_id());
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("Not in deposit whitelist"));

    let res = e.remove_from_deposit_whitelist(&e.draft_operator, &e.owner.valid_account_id());
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("Not in deposit whitelist"));

    // draft operator cannot control permissions for draft operators
    let res = e.add_to_draft_operators_whitelist(&e.draft_operator, &users.eve.valid_account_id());
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("Not in deposit whitelist"));

    let res =
        e.remove_from_draft_operators_whitelist(&e.draft_operator, &users.eve.valid_account_id());
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("Not in deposit whitelist"));

    // deposit_whitelist can add draft_operators
    let res = e.add_to_draft_operators_whitelist(&e.owner, &users.eve.valid_account_id());
    assert!(res.is_ok());

    // checking deposit list
    let res: Vec<AccountId> = e.get_deposit_whitelist();
    assert_eq!(res, vec![e.owner.account_id()]);
    // checking draft operators list
    let mut res: Vec<AccountId> = e.get_draft_operators_whitelist();
    res.sort();
    assert_eq!(
        res,
        vec![e.draft_operator.account_id(), users.eve.account_id()]
    );

    // deposit can remove draft operators
    let res =
        e.remove_from_draft_operators_whitelist(&e.owner, &e.draft_operator.valid_account_id());
    assert!(res.is_ok());
    let res: Vec<AccountId> = e.get_draft_operators_whitelist();
    assert_eq!(res, vec![users.eve.account_id()]);

    // new draft operator can create draft groups
    let res = e.create_draft_group(&users.eve);
    assert!(res.is_ok());
    // old draft operator cannot create draft groups
    let res = e.create_draft_group(&e.draft_operator);
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("Not in draft operators whitelist"));

    // new draft operator is NOT deposit, cannot manage users
    let res = e.add_to_draft_operators_whitelist(&users.eve, &users.dude.valid_account_id());
    assert!(!res.is_ok());
    assert!(format!("{:?}", res.status()).contains("Not in deposit whitelist"));

    // role presence in both lists
    let res = e.add_to_draft_operators_whitelist(&e.owner, &e.owner.valid_account_id());
    assert!(res.is_ok());
    let mut res: Vec<AccountId> = e.get_draft_operators_whitelist();
    res.sort();
    assert_eq!(res, vec![users.eve.account_id(), e.owner.account_id()]);

    // user is not removed from the draft operators list
    let mut res: Vec<AccountId> = e.get_deposit_whitelist();
    res.sort();
    assert_eq!(res, vec![e.owner.account_id()]);

    // user still has deposit abilities
    let amount = d(60000, TOKEN_DECIMALS);
    let lockup_create = LockupCreate::new_unlocked(users.alice.valid_account_id(), amount);
    let res = e.add_lockup(&e.owner, amount, &lockup_create);
    assert!(res.is_ok());
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, amount);

    // adding new draft operator, it's not allowed to remove every deposit_whitelist
    let res = e.add_to_deposit_whitelist(&e.owner, &users.charlie.valid_account_id());
    assert!(res.is_ok());
    // removing deposit role, draft operator role must be retained
    let res = e.remove_from_deposit_whitelist(&e.owner, &e.owner.valid_account_id());
    assert!(res.is_ok());
    // deposit role is removed
    let res: Vec<AccountId> = e.get_deposit_whitelist();
    assert_eq!(res, vec![users.charlie.account_id()] as Vec<AccountId>);
    // draft operator role is still present
    let res: Vec<AccountId> = e.get_draft_operators_whitelist();
    assert_eq!(res, vec![users.eve.account_id(), e.owner.account_id()]);

    // deposit role must be removed
    let res = e.add_lockup(&e.owner, amount, &lockup_create);
    assert!(res.is_ok());
    let balance: WrappedBalance = res.unwrap_json();
    assert_eq!(balance.0, 0);

    // draft operator role is retained
    let res = e.create_draft_group(&e.owner);
    assert!(res.is_ok());
}
