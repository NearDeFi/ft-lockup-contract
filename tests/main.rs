mod setup;

use crate::setup::*;

#[test]
fn test_init_env() {
    let e = Env::init(None);
    let _users = Users::init(&e);
}
