use crate::*;

pub type DraftGroupIndex = u32;
pub type DraftIndex = u32;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), derive(Clone))]
#[serde(crate = "near_sdk::serde")]
pub struct Draft {
    pub draft_group_id: DraftGroupIndex,
    pub lockup: Lockup,
}

impl Draft {
    pub fn assert_new_valid(&self) {
        let amount = self.lockup.schedule.total_balance();
        self.lockup.assert_new_valid(amount);
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
pub struct DraftGroup {
    pub total_amount: Balance,
    pub funded: bool,
    pub draft_indices: HashSet<DraftIndex>,
}

impl DraftGroup {
    pub fn new() -> Self {
        Self {
            total_amount: 0,
            funded: false,
            draft_indices: HashSet::new(),
        }
    }

    pub fn assert_can_add_draft(&self) {
        assert!(!self.funded, "cannot add draft, group already funded");
    }

    pub fn assert_can_convert(&self) {
        assert!(self.funded, "cannot convert draft from not funded group");
    }
}
