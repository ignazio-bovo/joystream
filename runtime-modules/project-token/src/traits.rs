use frame_support::dispatch::{DispatchError, DispatchResult};
use sp_runtime::Percent;

/// Account wrapper that encapsulates the validation for the transfer location
/// by means of the visitor pattern
pub trait TransferLocationTrait<AccountId, Policy> {
    /// encapsulates eventual merkle tree validation given policy
    fn is_valid_location_for_policy(&self, policy: &Policy) -> bool;

    /// the wrapped account
    fn location_account(&self) -> AccountId;
}

pub trait PalletToken<AccountId, Policy, IssuanceParams> {
    /// Balance type used
    type Balance;

    /// Token Identifier type used
    type TokenId;

    /// Block number type
    type BlockNumber;

    /// Reserve Balance used (JOY balance)
    type ReserveBalance;

    /// Parameters used for Split timeline consruction
    type SplitTimelineParameters;

    /// Mint `amount` into account `who` (possibly creating it)
    fn deposit_creating(
        token_id: Self::TokenId,
        who: AccountId,
        amount: Self::Balance,
    ) -> DispatchResult;

    /// Issue token with specified characteristics
    fn issue_token(issuance_parameters: IssuanceParams) -> DispatchResult;

    /// Remove token data from storage
    fn deissue_token(token_id: Self::TokenId) -> DispatchResult;

    /// Change to permissionless
    fn change_to_permissionless(token_id: Self::TokenId) -> DispatchResult;

    /// Transfer `amount` from `src` account to `dst` according to provided policy
    fn transfer<Destination>(
        token_id: Self::TokenId,
        src: AccountId,
        dst: Destination,
        amount: Self::Balance,
    ) -> DispatchResult
    where
        Destination: TransferLocationTrait<AccountId, Policy> + Clone;

    /// Transfer `amount` from `src` account to `dst` according to provided policy
    fn multi_output_transfer<Destination>(
        token_id: Self::TokenId,
        src: AccountId,
        outputs: &[(Destination, Self::Balance)],
    ) -> DispatchResult
    where
        Destination: TransferLocationTrait<AccountId, Policy>;

    /// Reduce patronage rate by amount
    fn reduce_patronage_rate_by(token_id: Self::TokenId, decrement: Percent) -> DispatchResult;

    /// Query for patronage credit for token
    fn get_patronage_credit(token_id: Self::TokenId) -> Result<Self::Balance, DispatchError>;

    /// Allow creator to receive credit into his accounts
    fn claim_patronage_credit(token_id: Self::TokenId, to_account: AccountId) -> DispatchResult;

    /// Issue a revenue split for the token
    fn issue_revenue_split(
        token_id: Self::TokenId,
        timeline_params: Self::SplitTimelineParameters,
        reserve_source: AccountId,
        allocation: Self::ReserveBalance,
        percentage: Percent,
    ) -> DispatchResult;

    /// Participate to the token revenue split if ongoing
    fn participate_to_split(
        token_id: Self::TokenId,
        who: AccountId,
        amount: Self::Balance,
    ) -> DispatchResult;

    /// Members can claim their split revenue
    fn claim_revenue_split_amount(token_id: Self::TokenId, who: AccountId) -> DispatchResult;

    /// Finalize split by sending back eventual JOYs leftover
    fn finalize_revenue_split(token_id: Self::TokenId, account_id: AccountId) -> DispatchResult;
}
