use codec::Encode;
use frame_support::traits::Currency;
use sp_arithmetic::traits::{One, Saturating, Zero};
use sp_runtime::traits::{Convert, Hash};
use sp_runtime::Percent;

use crate::tests::mock::*;
use crate::types::{
    IssuanceState, LinearVestingSchedule, MerkleSide, PatronageData, SimpleLocation, SplitState,
    SplitTimeline, SplitTimelineParameters, TransferPolicy, VerifiableLocation, VestingSchedule,
};
use crate::GenesisConfig;

pub struct TokenDataBuilder<Balance, Hash, BlockNumber> {
    pub(crate) current_total_issuance: Balance,
    pub(crate) existential_deposit: Balance,
    pub(crate) issuance_state: IssuanceState,
    pub(crate) transfer_policy: TransferPolicy<Hash>,
    pub(crate) patronage_info: PatronageData<Balance>,
    pub(crate) revenue_split: SplitState<BlockNumber>,
}

impl<Balance: Zero + Copy + PartialOrd + Saturating, Hash, BlockNumber>
    TokenDataBuilder<Balance, Hash, BlockNumber>
{
    pub fn build(self) -> crate::types::TokenData<Balance, Hash, BlockNumber> {
        crate::types::TokenData::<Balance, Hash, BlockNumber> {
            current_total_issuance: self.current_total_issuance,
            existential_deposit: self.existential_deposit,
            issuance_state: self.issuance_state,
            transfer_policy: self.transfer_policy,
            patronage_info: self.patronage_info,
            revenue_split: self.revenue_split,
        }
    }

    pub fn with_issuance(self, current_total_issuance: Balance) -> Self {
        Self {
            current_total_issuance,
            ..self
        }
    }

    pub fn with_existential_deposit(self, existential_deposit: Balance) -> Self {
        Self {
            existential_deposit,
            ..self
        }
    }

    pub fn with_revenue_split(
        self,
        timeline: SplitTimeline<BlockNumber>,
        percentage: Percent,
    ) -> Self {
        Self {
            revenue_split: SplitState::<BlockNumber>::Active(timeline, percentage),
            ..self
        }
    }

    pub fn with_transfer_policy(self, transfer_policy: TransferPolicy<Hash>) -> Self {
        Self {
            transfer_policy,
            ..self
        }
    }

    pub fn with_patronage_rate(self, rate: Percent) -> Self {
        Self {
            patronage_info: PatronageData::<Balance> {
                rate,
                ..self.patronage_info
            },
            ..self
        }
    }

    pub fn with_patronage_credit(self, outstanding_credit: Balance) -> Self {
        Self {
            patronage_info: PatronageData::<Balance> {
                outstanding_credit,
                ..self.patronage_info
            },
            current_total_issuance: self
                .current_total_issuance
                .saturating_add(outstanding_credit),
            ..self
        }
    }

    pub fn new_empty() -> Self {
        Self {
            current_total_issuance: Balance::zero(),
            issuance_state: IssuanceState::Idle,
            existential_deposit: Balance::zero(),
            transfer_policy: TransferPolicy::<Hash>::Permissionless,
            patronage_info: PatronageData::<Balance> {
                rate: Percent::zero(),
                outstanding_credit: Balance::zero(),
            },
            revenue_split: SplitState::Inactive,
        }
    }
}

impl GenesisConfigBuilder {
    pub fn new_empty() -> Self {
        Self {
            token_info_by_id: vec![],
            account_info_by_token_and_account: vec![],
            next_token_id: TokenId::one(),
            symbols_used: vec![],
        }
    }

    // add token with given params & zero issuance
    pub fn with_token(mut self, token_id: TokenId, token_info: TokenData) -> Self {
        self.token_info_by_id.push((token_id, token_info));
        self.next_token_id = self.next_token_id.saturating_add(TokenId::one());
        self.symbols_used.push((Hashing::hash_of(&token_id), ()));
        self
    }

    // add account & updates token issuance
    pub fn with_account(
        mut self,
        account_id: AccountId,
        liquidity: Balance,
        staked_balance: Balance,
    ) -> Self {
        let id = self.next_token_id.saturating_sub(TokenId::one());
        let new_account_info = AccountData {
            liquidity,
            staked_balance,
            vesting_schedule: VestingSchedule::<BlockNumber, Balance>::default(),
        };

        let new_issuance = self
            .token_info_by_id
            .last()
            .unwrap()
            .1
            .current_total_issuance
            .saturating_add(Balance::from(liquidity.saturating_add(staked_balance)));

        self.account_info_by_token_and_account
            .push((id, account_id, new_account_info));

        self.token_info_by_id
            .last_mut()
            .unwrap()
            .1
            .current_total_issuance = new_issuance;
        self
    }

    pub fn with_vesting<Converter: Convert<BlockNumber, Balance>>(
        mut self,
        account_id: AccountId,
        vesting: VestingSchedule<BlockNumber, Balance>,
    ) -> Self {
        let token_id = self.next_token_id.saturating_sub(TokenId::one());
        if token_id.is_zero() {
            return self;
        }
        if let Some((_, _, account_info)) = self
            .account_info_by_token_and_account
            .iter_mut()
            .find(|&&mut (_, id, _)| id == account_id)
        {
            let amount = vesting.total_amount::<Converter>();
            account_info.vesting_schedule = vesting;
            let (tk_id, mut token_info) = self.token_info_by_id.pop().unwrap();
            token_info.current_total_issuance =
                token_info.current_total_issuance.saturating_add(amount);
            self.token_info_by_id.push((tk_id, token_info))
        }
        self
    }

    pub fn build(self) -> GenesisConfig<Test> {
        GenesisConfig::<Test> {
            account_info_by_token_and_account: self.account_info_by_token_and_account,
            token_info_by_id: self.token_info_by_id,
            next_token_id: self.next_token_id,
            symbols_used: self.symbols_used,
        }
    }
}

impl<BlockNumber: Copy + Saturating + PartialOrd> SplitTimelineParameters<BlockNumber> {
    pub fn new(start: BlockNumber, duration: BlockNumber) -> Self {
        Self { start, duration }
    }
}

impl<AccountId> SimpleLocation<AccountId> {
    pub fn new(account: AccountId) -> Self {
        Self { account }
    }
}

impl<BlockNumber: Clone, Balance: Zero + Saturating + Clone> VestingSchedule<BlockNumber, Balance> {
    pub fn new(
        cliff: BlockNumber,
        vesting_rate: Balance,
        starting_block: BlockNumber,
        duration: BlockNumber,
    ) -> Self {
        let vesting = LinearVestingSchedule::<BlockNumber, Balance> {
            cliff,
            vesting_rate,
            starting_block,
            duration,
        };
        Self(Some(vesting))
    }

    pub fn total_amount<BlockNumberToBalance: Convert<BlockNumber, Balance>>(&self) -> Balance {
        self.0.as_ref().map_or(Balance::zero(), |vesting| {
            let duration_b = BlockNumberToBalance::convert(vesting.duration.to_owned());
            duration_b.saturating_mul(vesting.vesting_rate.to_owned())
        })
    }
}

impl<AccountId: Encode, Hasher: Hash> VerifiableLocation<AccountId, Hasher> {
    pub fn new(merkle_proof: Vec<(Hasher::Output, MerkleSide)>, account: AccountId) -> Self {
        Self {
            merkle_proof,
            account,
        }
    }
}

pub fn increase_account_balance(account_id: AccountId, balance: ReserveBalance) {
    let _ = Balances::deposit_creating(&account_id, balance);
}

#[cfg(test)]
#[ignore]
#[test]
fn with_token_assigns_correct_token_id() {
    let token_id: TokenId = 1;
    let token_params = TokenDataBuilder::new_empty().build();

    let builder = GenesisConfigBuilder::new_empty().with_token(token_id, token_params);

    let id = builder.token_info_by_id.last().unwrap().0;
    assert_eq!(id, token_id);
}

#[ignore]
#[test]
fn with_issuance_adds_issuance_to_token() {
    let token_params = TokenDataBuilder::new_empty().with_issuance(5).build();

    let builder = GenesisConfigBuilder::new_empty().with_token(1, token_params);

    let issuance = builder
        .token_info_by_id
        .last()
        .unwrap()
        .1
        .current_total_issuance;
    assert_eq!(issuance, 5);
}

#[ignore]
#[test]
fn adding_account_with_liquidity_also_adds_issuance() {
    let token_params = TokenDataBuilder::new_empty().with_issuance(5).build();
    let mut builder = GenesisConfigBuilder::new_empty().with_token(1, token_params);
    builder = builder.with_account(1, 5, 5);

    let issuance = builder
        .token_info_by_id
        .last()
        .unwrap()
        .1
        .current_total_issuance;
    assert_eq!(issuance, 15);
}
