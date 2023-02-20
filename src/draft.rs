use crate::*;

pub type DraftGroupIndex = u32;
pub type DraftIndex = u32;

#[derive(BorshDeserialize, BorshSerialize, Deserialize, Serialize)]
#[serde(crate = "near_sdk::serde")]
#[cfg_attr(not(target_arch = "wasm32"), derive(Debug, PartialEq, Clone))]
pub struct Draft {
    pub draft_group_id: DraftGroupIndex,
    pub lockup_create: LockupCreate,
}

impl Draft {
    pub fn total_balance(&self) -> Balance {
        self.lockup_create.schedule.total_balance()
    }

    pub fn assert_new_valid(&self) {
        let amount = self.lockup_create.schedule.total_balance();
        // any valid near account id will work fine here as a parameter
        self.lockup_create
            .into_lockup(&env::predecessor_account_id().try_into().unwrap())
            .assert_new_valid(amount);
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct DraftGroup {
    pub total_amount: Balance,
    pub payer_id: Option<ValidAccountId>,
    pub draft_indices: HashSet<DraftIndex>,
    pub discarded: bool,
}

impl DraftGroup {
    pub fn new() -> Self {
        Self {
            total_amount: 0,
            payer_id: None,
            draft_indices: HashSet::new(),
            discarded: false,
        }
    }

    pub fn assert_can_add_draft(&self) {
        assert!(
            !self.discarded,
            "cannot add draft, draft group is discarded"
        );
        assert!(
            self.payer_id.is_none(),
            "cannot add draft, group already funded"
        );
    }

    pub fn assert_can_convert_draft(&self) {
        assert!(
            !self.discarded,
            "cannot convert draft, draft group is discarded"
        );
        assert!(
            self.payer_id.is_some(),
            "cannot convert draft from not funded group"
        );
    }

    pub fn assert_can_fund(&self) {
        assert!(
            !self.discarded,
            "cannot fund draft, draft group is discarded"
        );
        assert!(self.payer_id.is_none(), "draft group already funded");
    }

    pub fn fund(&mut self, payer_id: &ValidAccountId) {
        self.assert_can_fund();
        self.payer_id = Some(payer_id.clone());
    }

    pub fn assert_can_discard(&mut self) {
        assert!(
            !self.discarded,
            "cannot discard, draft group already discarded"
        );
        assert!(
            self.payer_id.is_none(),
            "cannot discard, draft group already funded"
        );
    }

    pub fn discard(&mut self) {
        self.assert_can_discard();
        self.discarded = true;
    }

    pub fn assert_can_delete_draft(&mut self) {
        assert!(
            self.discarded,
            "cannot delete draft, draft group is not discarded"
        );
        assert!(
            self.payer_id.is_none(),
            "cannot delete draft, draft group already funded"
        );
    }
}
