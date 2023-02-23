#![cfg(test)]

use crate::tests::mock::*;
// TODO remove test utils
use crate::tests::test_utils::{new_issuer_transfers, new_transfers, TokenDataBuilder};
use crate::traits::PalletToken;
use crate::types::{AmmParams, TransferOutputsOf, TransferWithVestingOutputsOf};
use crate::{member, yearly_rate, YearlyRate};
use derive_fixture::Fixture;
use derive_new::new;
use frame_support::dispatch::DispatchResult;
use sp_runtime::{testing::H256, Permill};

use sp_std::collections::btree_map::BTreeMap;
use sp_std::iter::FromIterator;
use storage::{BagId, DataObjectCreationParameters, StaticBagId};

pub trait Fixture {
    fn execute(&self) -> DispatchResult;

    fn execute_call(&self) -> DispatchResult {
        let state_pre = sp_io::storage::root(sp_storage::StateVersion::V1);
        let result = self.execute();

        // no-op in case of error
        if result.is_err() {
            let state_post = sp_io::storage::root(sp_storage::StateVersion::V1);
            assert_eq!(state_pre, state_post)
        }

        result
    }

    fn run(&self) {
        self.execute_call().unwrap()
    }
}

pub fn default_upload_context() -> UploadContext {
    UploadContext {
        bag_id: BagId::<Test>::Static(StaticBagId::Council),
        uploader_account: FIRST_USER_ACCOUNT_ID,
    }
}

pub fn issuer_allocation(amount: Balance) -> BTreeMap<MemberId, TokenAllocation> {
    BTreeMap::from_iter(vec![(
        DEFAULT_ISSUER_MEMBER_ID,
        TokenAllocation {
            amount,
            vesting_schedule_params: None,
        },
    )])
}

#[allow(dead_code)]
pub fn default_single_data_object_upload_params() -> SingleDataObjectUploadParams {
    SingleDataObjectUploadParams {
        expected_data_size_fee: storage::Module::<Test>::data_object_per_mega_byte_fee(),
        expected_data_object_state_bloat_bond:
            storage::Module::<Test>::data_object_state_bloat_bond_value(),
        object_creation_params: DataObjectCreationParameters {
            ipfs_content_id: Vec::from_iter(0..46),
            size: 1_000_000,
        },
    }
}

#[derive(Fixture, new)]
pub struct IssueTokenFixture {
    #[new(value = "DEFAULT_ISSUER_ACCOUNT_ID")]
    issuer_account: AccountId,

    #[new(value = "yearly_rate!(0)")]
    patronage_rate: YearlyRate,

    #[new(value = "H256::zero()")]
    symbol: H256,

    #[new(value = "TransferPolicyParams::Permissionless")]
    transfer_policy_params: TransferPolicyParams,

    #[new(value = "DEFAULT_SPLIT_RATE")]
    revenue_split_rate: Permill,

    #[new(value = "default_upload_context()")]
    upload_context: UploadContext,

    #[new(value = "issuer_allocation(DEFAULT_INITIAL_ISSUANCE)")]
    initial_allocation: BTreeMap<MemberId, TokenAllocation>,
}

impl IssueTokenFixture {
    pub fn with_empty_allocation(self) -> Self {
        Self {
            initial_allocation: BTreeMap::new(),
            ..self
        }
    }

    pub fn with_initial_supply(self, amount: Balance) -> Self {
        Self {
            initial_allocation: issuer_allocation(amount),
            ..self
        }
    }
}

impl Fixture for IssueTokenFixture {
    fn execute(&self) -> DispatchResult {
        let issuance_params = IssuanceParams {
            patronage_rate: self.patronage_rate,
            symbol: self.symbol,
            transfer_policy: self.transfer_policy_params.clone(),
            revenue_split_rate: self.revenue_split_rate,
            initial_allocation: self.initial_allocation.clone(),
        };
        Token::issue_token(
            self.issuer_account.clone(),
            issuance_params,
            self.upload_context.clone(),
        )
        .map(|_| ())
    }
}

#[derive(Fixture, new)]
pub struct BurnFixture {
    #[new(value = "DEFAULT_ISSUER_ACCOUNT_ID")]
    sender: AccountId,

    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,

    #[new(value = "DEFAULT_ISSUER_MEMBER_ID")]
    member_id: MemberId,

    #[new(value = "DEFAULT_USER_BURN_AMOUNT")]
    amount: Balance,
}

impl BurnFixture {
    pub fn with_user(self, account_id: AccountId, member_id: MemberId) -> Self {
        self.with_sender(account_id).with_member_id(member_id)
    }
}

impl Fixture for BurnFixture {
    fn execute(&self) -> DispatchResult {
        Token::burn(
            Origin::signed(self.sender),
            self.token_id,
            self.member_id,
            self.amount,
        )
    }
}
#[derive(Fixture, new)]
pub struct InitTokenSaleFixture {
    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,

    #[new(value = "DEFAULT_ISSUER_MEMBER_ID")]
    member_id: MemberId,

    #[new(value = "Some(DEFAULT_ISSUER_ACCOUNT_ID)")]
    earnings_destination: Option<AccountId>,

    #[new(value = "true")]
    auto_finalize: bool,

    #[new(default)]
    cap_per_member: Option<Balance>,

    #[new(value = "DEFAULT_SALE_DURATION")]
    duration: BlockNumber,

    #[new(default)]
    start_block: Option<BlockNumber>,

    #[new(value = "DEFAULT_SALE_UNIT_PRICE")]
    unit_price: Balance,

    #[new(default)]
    blocks_before_cliff: BlockNumber,

    #[new(value = "DEFAULT_INITIAL_ISSUANCE")]
    upper_bound_quantity: Balance,

    #[new(value = "Permill::zero()")]
    cliff_amount_percentage: Permill,

    #[new(default)]
    metadata: Option<Vec<u8>>,

    #[new(value = "100u32.into()")]
    linear_vesting_duration: BlockNumber,
}

impl Fixture for InitTokenSaleFixture {
    fn execute(&self) -> DispatchResult {
        let sale_params = TokenSaleParams {
            duration: self.duration,
            metadata: self.metadata.clone(),
            starts_at: self.start_block,
            unit_price: self.unit_price,
            upper_bound_quantity: self.upper_bound_quantity,
            vesting_schedule_params: Some(VestingScheduleParams {
                blocks_before_cliff: self.blocks_before_cliff,
                linear_vesting_duration: self.linear_vesting_duration,
                cliff_amount_percentage: self.cliff_amount_percentage,
            }),
            cap_per_member: self.cap_per_member,
        };
        Token::init_token_sale(
            self.token_id,
            self.member_id,
            self.earnings_destination,
            self.auto_finalize,
            sale_params,
        )
    }
}

#[derive(Fixture, new)]
pub struct UpdateUpcomingSaleFixture {
    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,

    #[new(value = "Some(DEFAULT_SALE_DURATION + 1)")]
    new_duration: Option<BlockNumber>,

    #[new(value = "Some(200)")]
    new_start_block: Option<BlockNumber>,
}

impl Fixture for UpdateUpcomingSaleFixture {
    fn execute(&self) -> DispatchResult {
        Token::update_upcoming_sale(
            self.token_id,
            self.new_start_block.clone(),
            self.new_duration.clone(),
        )
    }
}

#[derive(Fixture, new)]
pub struct PurchaseTokensOnSaleFixture {
    #[new(value = "FIRST_USER_ACCOUNT_ID")]
    sender: AccountId,

    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,

    #[new(value = "FIRST_USER_MEMBER_ID")]
    member_id: MemberId,

    #[new(value = "DEFAULT_SALE_PURCHASE_AMOUNT")]
    amount: Balance,
}

impl Fixture for PurchaseTokensOnSaleFixture {
    fn execute(&self) -> DispatchResult {
        Token::purchase_tokens_on_sale(
            Origin::signed(self.sender),
            self.token_id,
            self.member_id,
            self.amount,
        )
    }
}

#[derive(Fixture, new)]
pub struct FinalizeTokenSaleFixture {
    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,
}

impl Fixture for FinalizeTokenSaleFixture {
    fn execute(&self) -> DispatchResult {
        Token::finalize_token_sale(self.token_id).map(|_| ())
    }
}

#[derive(Fixture, new)]
pub struct IssueRevenueSplitFixture {
    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,

    #[new(default)]
    start: Option<BlockNumber>,

    #[new(value = "DEFAULT_SPLIT_DURATION")]
    duration: BlockNumber,

    #[new(value = "DEFAULT_ISSUER_ACCOUNT_ID")]
    revenue_source_account: AccountId,

    #[new(value = "DEFAULT_SPLIT_REVENUE.into()")]
    revenue_amount: JoyBalance,
}

impl Fixture for IssueRevenueSplitFixture {
    fn execute(&self) -> DispatchResult {
        Token::issue_revenue_split(
            self.token_id,
            self.start,
            self.duration,
            self.revenue_source_account,
            self.revenue_amount,
        )
        .map(|_| ())
    }
}

#[derive(Fixture, new)]
pub struct FinalizeRevenueSplitFixture {
    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,

    #[new(value = "DEFAULT_ISSUER_ACCOUNT_ID")]
    account_id: AccountId,
}

impl Fixture for FinalizeRevenueSplitFixture {
    fn execute(&self) -> DispatchResult {
        Token::finalize_revenue_split(self.token_id, self.account_id)
    }
}

#[derive(Fixture, new)]
pub struct ParticipateInSplitFixture {
    #[new(value = "FIRST_USER_ACCOUNT_ID")]
    sender: AccountId,

    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,

    #[new(value = "FIRST_USER_MEMBER_ID")]
    member_id: MemberId,

    #[new(value = "DEFAULT_SPLIT_PARTICIPATION")]
    amount: Balance,
}

impl Fixture for ParticipateInSplitFixture {
    fn execute(&self) -> DispatchResult {
        Token::participate_in_split(
            Origin::signed(self.sender),
            self.token_id,
            self.member_id,
            self.amount,
        )
    }
}

#[derive(Fixture, new)]
pub struct ChangeToPermissionlessFixture {
    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,
}

impl Fixture for ChangeToPermissionlessFixture {
    fn execute(&self) -> DispatchResult {
        Token::change_to_permissionless(self.token_id)
    }
}

#[derive(Fixture, new)]
pub struct TransferFixture {
    #[new(value = "FIRST_USER_ACCOUNT_ID")]
    sender: AccountId,

    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,

    #[new(value = "FIRST_USER_MEMBER_ID")]
    src_member_id: MemberId,

    #[new(value = "new_transfers(vec![(SECOND_USER_MEMBER_ID, DEFAULT_USER_BALANCE)])")]
    outputs: TransferOutputsOf<Test>,

    #[new(value = "b\"metadata\".to_vec()")]
    metadata: Vec<u8>,
}

impl TransferFixture {
    pub fn with_output(self, member_id: MemberId, amount: Balance) -> Self {
        self.with_outputs(new_transfers(vec![(member_id, amount)]))
    }

    pub fn with_multioutput_and_same_amount(
        self,
        first_dest: MemberId,
        second_dest: MemberId,
        amount: Balance,
    ) -> Self {
        self.with_outputs(new_transfers(vec![
            (first_dest, amount),
            (second_dest, amount),
        ]))
    }
}
impl Fixture for TransferFixture {
    fn execute(&self) -> DispatchResult {
        Token::transfer(
            Origin::signed(self.sender),
            self.src_member_id,
            self.token_id,
            self.outputs.clone(),
            self.metadata.clone(),
        )
    }
}

#[derive(Fixture, new)]
pub struct IssuerTransferFixture {
    #[new(value = "DEFAULT_ISSUER_ACCOUNT_ID")]
    bloat_bond_payer: AccountId,

    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,

    #[new(value = "DEFAULT_ISSUER_MEMBER_ID")]
    src_member_id: MemberId,

    #[new(
        value = "new_issuer_transfers(vec![(FIRST_USER_MEMBER_ID, DEFAULT_USER_BALANCE, None)])"
    )]
    outputs: TransferWithVestingOutputsOf<Test>,

    #[new(value = "b\"metadata\".to_vec()")]
    metadata: Vec<u8>,
}

impl IssuerTransferFixture {
    pub fn with_output(
        self,
        member_id: MemberId,
        amount: Balance,
        vesting: Option<VestingScheduleParams>,
    ) -> Self {
        self.with_outputs(new_issuer_transfers(vec![(member_id, amount, vesting)]))
    }
}

impl Fixture for IssuerTransferFixture {
    fn execute(&self) -> DispatchResult {
        Token::issuer_transfer(
            self.token_id,
            self.src_member_id,
            self.bloat_bond_payer,
            self.outputs.clone(),
            self.metadata.clone(),
        )
    }
}

#[derive(Fixture, new)]
pub struct ExitRevenueSplitFixture {
    #[new(value = "FIRST_USER_ACCOUNT_ID")]
    sender: AccountId,

    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,

    #[new(value = "FIRST_USER_MEMBER_ID")]
    member_id: MemberId,
}

impl Fixture for ExitRevenueSplitFixture {
    fn execute(&self) -> DispatchResult {
        Token::exit_revenue_split(Origin::signed(self.sender), self.token_id, self.member_id)
    }
}

#[derive(Fixture, new)]
pub struct ActivateAmmFixture {
    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,

    #[new(value = "DEFAULT_ISSUER_MEMBER_ID")]
    member_id: MemberId,

    #[new(value = "AMM_CURVE_SLOPE")]
    slope: Permill,

    #[new(value = "AMM_CURVE_INTERCEPT")]
    intercept: Permill,
}

impl Fixture for ActivateAmmFixture {
    fn execute(&self) -> DispatchResult {
        let amm_params = AmmParams {
            slope: self.slope,
            intercept: self.intercept,
        };
        Token::activate_amm(self.token_id, self.member_id, amm_params)
    }
}

#[derive(Fixture, new)]
pub struct AmmBuyFixture {
    #[new(value = "FIRST_USER_ACCOUNT_ID")]
    sender: AccountId,

    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,

    #[new(value = "FIRST_USER_MEMBER_ID")]
    member_id: MemberId,

    #[new(value = "DEFAULT_AMM_BUY_AMOUNT")]
    amount: Balance,

    #[new(default)]
    slippage_tolerance: Option<(Permill, Balance)>,
}

impl Fixture for AmmBuyFixture {
    fn execute(&self) -> DispatchResult {
        Token::buy_on_amm(
            Origin::signed(self.sender),
            self.token_id,
            self.member_id,
            self.amount,
            self.slippage_tolerance.clone(),
        )
    }
}

#[derive(Fixture, new)]
pub struct AmmSellFixture {
    #[new(value = "FIRST_USER_ACCOUNT_ID")]
    sender: AccountId,

    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,

    #[new(value = "FIRST_USER_MEMBER_ID")]
    member_id: MemberId,

    #[new(value = "DEFAULT_AMM_SELL_AMOUNT")]
    amount: Balance,

    #[new(default)]
    slippage_tolerance: Option<(Permill, Balance)>,
}

impl Fixture for AmmSellFixture {
    fn execute(&self) -> DispatchResult {
        Token::sell_on_amm(
            Origin::signed(self.sender),
            self.token_id,
            self.member_id,
            self.amount,
            self.slippage_tolerance,
        )
    }
}

#[derive(Fixture, new)]
pub struct DeactivateAmmFixture {
    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,

    #[new(value = "DEFAULT_ISSUER_MEMBER_ID")]
    member_id: MemberId,
}

impl Fixture for DeactivateAmmFixture {
    fn execute(&self) -> DispatchResult {
        Token::deactivate_amm(self.token_id, self.member_id)
    }
}

#[derive(Fixture, new)]
pub struct ClaimPatronageCreditFixture {
    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,

    #[new(value = "FIRST_USER_MEMBER_ID")]
    member_id: MemberId,
}

impl Fixture for ClaimPatronageCreditFixture {
    fn execute(&self) -> DispatchResult {
        Token::claim_patronage_credit(self.token_id, self.member_id)
    }
}

#[derive(Fixture, new)]
pub struct DustAccountFixture {
    #[new(value = "DEFAULT_ISSUER_ACCOUNT_ID")]
    sender: AccountId,

    #[new(value = "DEFAULT_ISSUER_MEMBER_ID")]
    member_id: MemberId,

    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,
}

impl DustAccountFixture {
    pub fn with_user(self, account_id: AccountId, member_id: MemberId) -> Self {
        self.with_sender(account_id).with_member_id(member_id)
    }
}

impl Fixture for DustAccountFixture {
    fn execute(&self) -> DispatchResult {
        Token::dust_account(Origin::signed(self.sender), self.token_id, self.member_id)
    }
}

#[derive(Fixture, new)]
pub struct DeissueTokenFixture {
    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,
}

impl Fixture for DeissueTokenFixture {
    fn execute(&self) -> DispatchResult {
        Token::deissue_token(self.token_id)
    }
}

#[derive(Fixture, new)]
pub struct ReducePatronageRateToFixture {
    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,

    #[new(value = "DEFAULT_YEARLY_PATRONAGE_RATE.into()")]
    rate: YearlyRate,
}

impl Fixture for ReducePatronageRateToFixture {
    fn execute(&self) -> DispatchResult {
        Token::reduce_patronage_rate_to(self.token_id, self.rate)
    }
}

#[derive(Fixture, new)]
pub struct JoinWhitelistFixture {
    #[new(value = "FIRST_USER_ACCOUNT_ID")]
    sender: AccountId,

    #[new(value = "FIRST_USER_MEMBER_ID")]
    member_id: MemberId,

    #[new(value = "DEFAULT_TOKEN_ID")]
    token_id: TokenId,

    #[new(value = "crate::MerkleProof(vec![])")]
    merkle_proof: MerkleProof,
}

impl Fixture for JoinWhitelistFixture {
    fn execute(&self) -> DispatchResult {
        Token::join_whitelist(
            Origin::signed(self.sender),
            self.member_id,
            self.token_id,
            self.merkle_proof.clone(),
        )
    }
}

#[derive(Fixture, new)]
pub struct TokenContext {
    #[new(value = "false")]
    empty_allocation: bool,

    #[new(default)]
    first_user_balance: Option<Balance>,

    #[new(default)]
    first_user_vesting: Option<VestingScheduleParams>,

    #[new(default)]
    second_user_balance: Option<Balance>,

    #[new(default)]
    second_user_vesting: Option<VestingScheduleParams>,

    #[new(default)]
    permissioned_policy_with_members: Option<Vec<MemberId>>, // Some(vec![user1, user2 ]) -> Permissioned else Permissionless
}

impl TokenContext {
    pub fn build(self) {
        let issue_token_fixture = IssueTokenFixture::new();

        let mut allocation = vec![];
        if !self.empty_allocation {
            allocation = vec![(
                DEFAULT_ISSUER_MEMBER_ID,
                TokenAllocation {
                    amount: DEFAULT_INITIAL_ISSUANCE,
                    vesting_schedule_params: None,
                },
            )];

            if let Some(balance) = self.first_user_balance {
                let first_user_allocation = TokenAllocation {
                    amount: balance,
                    vesting_schedule_params: self.first_user_vesting,
                };
                allocation.push((FIRST_USER_MEMBER_ID, first_user_allocation));
            }

            if let Some(balance) = self.second_user_balance {
                let second_user_allocation = TokenAllocation {
                    amount: balance,
                    vesting_schedule_params: self.second_user_vesting,
                };
                allocation.push((SECOND_USER_MEMBER_ID, second_user_allocation));
            }
        }

        let policy_params = if let Some(members) = self.permissioned_policy_with_members {
            let commitment = generate_merkle_root_helper::<Test, _>(members.as_slice())
                .pop()
                .unwrap();
            TransferPolicyParams::Permissioned(WhitelistParams {
                commitment,
                payload: None,
            })
        } else {
            TransferPolicyParams::Permissionless
        };

        issue_token_fixture
            .with_initial_allocation(allocation.into_iter().collect())
            .with_transfer_policy_params(policy_params)
            .run();
    }

    // some contexts
    pub fn with_issuer_and_first_user() {
        Self::new()
            .with_first_user_balance(Some(DEFAULT_USER_BALANCE))
            .build();
    }

    pub fn with_issuer_and_users() {
        Self::new()
            .with_first_user_balance(Some(DEFAULT_USER_BALANCE))
            .with_second_user_balance(Some(DEFAULT_USER_BALANCE))
            .build();
    }

    pub fn with_issuer_only() {
        Self::new().build();
    }

    pub fn with_issuer_only_permissioned() {
        Self::new()
            .with_permissioned_policy_with_members(Some(vec![
                FIRST_USER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
            ]))
            .build();
    }

    pub fn with_issuer_and_first_user_permissioned() {
        Self::new()
            .with_first_user_balance(Some(DEFAULT_USER_BALANCE))
            .with_permissioned_policy_with_members(Some(vec![
                FIRST_USER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
            ]))
            .build();
    }

    pub fn with_issuer_and_users_permissioned() {
        Self::new()
            .with_first_user_balance(Some(DEFAULT_USER_BALANCE))
            .with_second_user_balance(Some(DEFAULT_USER_BALANCE))
            .with_permissioned_policy_with_members(Some(vec![
                FIRST_USER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
            ]))
            .build();
    }
}
