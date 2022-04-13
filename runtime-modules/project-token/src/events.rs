use crate::types::{Output, TransferPolicyOf};
use frame_support::decl_event;

decl_event! {
    pub enum Event<T>
    where
        Balance = <T as crate::Trait>::Balance,
        TokenId = <T as crate::Trait>::TokenId,
        AccountId = <T as frame_system::Trait>::AccountId,
        BlockNumber = <T as frame_system::Trait>::BlockNumber,
        Output = Output<<T as frame_system::Trait>::AccountId, <T as crate::Trait>::Balance>,
        TransferPolicy = TransferPolicyOf<T>,

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
        /// - outputs: list of pairs (destination account, amount)
        TokenAmountTransferred(TokenId, AccountId, Vec<Output>),

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
        PatronageRateDecreasedTo(TokenId, Balance),

        /// Patronage credit claimed by creator
        /// Params:
        /// - token identifier
        /// - credit amount
        /// - account
        PatronageCreditClaimedAtBlock(TokenId, Balance, AccountId, BlockNumber),

        /// Member joined whitelist
        /// Params:
        /// - token identifier
        /// - account that has just joined
        /// - ongoing transfer policy
        MemberJoinedWhitelist(TokenId, AccountId, TransferPolicy),

        /// Account Dusted
        /// Params:
        /// - token identifier
        /// - account dusted
        /// - account that called the extrinsic
        /// - ongoing policy
        AccountDustedBy(TokenId, AccountId, AccountId, TransferPolicy),

        /// Token Deissued
        /// Params:
        /// - token id
        TokenDeissued(TokenId),
    }
}
