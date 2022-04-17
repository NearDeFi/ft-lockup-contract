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
    let lockup = Lockup::new_unlocked(users.alice.account_id.clone(), amount);
    let draft_group_id = 0;
    let draft = Draft {
        draft_group_id,
        lockup,
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
}

#[test]
fn test_view_drafts() {
    let e = Env::init(None);
    let users = Users::init(&e);
    e.set_time_sec(GENESIS_TIMESTAMP_SEC);

    let amount = d(60000, TOKEN_DECIMALS);
    let lockup = Lockup::new_unlocked(users.alice.account_id.clone(), amount);
    let draft_group_id = 0;
    let draft = Draft {
        draft_group_id,
        lockup,
    };

    e.create_draft_group(&e.owner);
    e.create_draft(&e.owner, &draft);
    e.create_draft(&e.owner, &draft);
    e.create_draft(&e.owner, &draft);

    let res = e.get_drafts(vec![2, 0]);
    assert_eq!(res.len(), 2);

    assert_eq!(res[0].0, 2);
    let draft = &res[0].1;
    assert_eq!(draft.draft_group_id, 0);
    assert_eq!(draft.lockup.total_balance, amount);

    assert_eq!(res[1].0, 0);
    let draft = &res[1].1;
    assert_eq!(draft.draft_group_id, 0);
    assert_eq!(draft.lockup.total_balance, amount);
}
