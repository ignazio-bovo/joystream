use frame_support::{decl_event, traits::Currency};
use sp_runtime::Percent;

decl_event! {
    pub enum Event<T>
    where
        Balance = <T as crate::Trait>::Balance,
        TokenId = <T as crate::Trait>::TokenId,
        AccountId = <T as frame_system::Trait>::AccountId,
        BlockNumber = <T as frame_system::Trait>::BlockNumber,
        ReserveBalance = <<T as crate::Trait>::ReserveCurrency as Currency<<T as frame_system::Trait>::AccountId>>::Balance,
    {
        /// Token amount is deposited
        /// Params:
        /// - token identifier
        /// - recipient account
        /// - amount deposited
        TokenAmountDepositedInto(TokenId, AccountId, Balance),

        /// Token amount is slashed
        /// Params:
        /// - token identifier
        /// - slashed account
        /// - amount slashed
        TokenAmountSlashedFrom(TokenId, AccountId, Balance),

        /// Token amount is transferred from src to dst
        /// Params:
        /// - token identifier
        /// - source account
        /// - destination account
        /// - amount transferred
        TokenAmountTransferred(TokenId, AccountId, AccountId, Balance),

        /// Token amount is transferred from src to dst
        /// Params:
        /// - token identifier
        /// - source account
        /// - outputs: list of pairs (destination account, amount)
        TokenAmountMultiTransferred(TokenId, AccountId, Vec<(AccountId, Balance)>),

        /// Token amount is reserved
        /// Params:
        /// - token identifier
        /// - account tokens are reserved from
        /// - amount reserved
        TokenAmountReservedFrom(TokenId, AccountId, Balance),

        /// Token amount is unreserved
        /// Params:
        /// - token identifier
        /// - account tokens are unreserved from
        /// - amount reserved
        TokenAmountUnreservedFrom(TokenId, AccountId, Balance),

        /// Patronage rate decreased
        /// Params:
        /// - token identifier
        /// - new patronage rate
        PatronageRateDecreasedTo(TokenId, Percent),

        /// Patronage credit claimed by creator
        /// Params:
        /// - token identifier
        /// - credit amount
        /// - account
        PatronageCreditClaimed(TokenId, Balance, AccountId),

        /// Revenue Split issued
        /// Params:
        /// - token identifier
        /// - start of the split
        /// - duration of the split
        /// - JOY allocated for the split
        /// - % of JOY allocated for the split used for accounting
        RevenueSplitIssued(TokenId, BlockNumber, BlockNumber, ReserveBalance, Percent),

        /// Revenue Split issued
        /// Params:
        /// - token identifier
        /// - recovery account for the leftover funds
        /// - leftover funds
        RevenueSplitFinalized(TokenId, AccountId, ReserveBalance),

        /// Revenue Split issued
        /// Params:
        /// - token identifier
        /// - user account
        /// - user allocated reserved balance
        /// - block height
        UserParticipatedToSplit(TokenId, AccountId, Balance, BlockNumber),

        /// User claimed revenue split
        /// Params:
        /// - token identifier
        /// - user account
        /// - Revenue Amount in JOY
        /// - block height
        UserClaimedRevenueSplit(TokenId, AccountId, ReserveBalance, BlockNumber),
    }
}
