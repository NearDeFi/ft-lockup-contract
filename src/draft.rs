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

    pub fn assert_new_valid(&self, payer_id: &ValidAccountId) {
        let amount = self.lockup_create.schedule.total_balance();
        self.lockup_create
            .into_lockup(payer_id)
            .assert_new_valid(amount, &payer_id);
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct DraftGroup {
    pub total_amount: Balance,
    pub payer_id: Option<ValidAccountId>,
    pub draft_indices: HashSet<DraftIndex>,
}

impl DraftGroup {
    pub fn new() -> Self {
        Self {
            total_amount: 0,
            payer_id: None,
            draft_indices: HashSet::new(),
        }
    }

    pub fn assert_can_add_draft(&self) {
        assert!(
            self.payer_id.is_none(),
            "cannot add draft, group already funded"
        );
    }

    pub fn assert_can_convert(&self) {
        assert!(
            self.payer_id.is_some(),
            "cannot convert draft from not funded group"
        );
    }
}
