use crate::Module;
use frame_support::decl_error;

decl_error! {
    pub enum Error for Module<T: crate::Trait> {
        /// Free balance is insufficient for freezing specified amount
        InsufficientFreeBalanceForReserving,

        /// Reserved balance is insufficient for unfreezing specified amount
        InsufficientReservedBalance,

        /// Free balance is insufficient for slashing specified amount
        InsufficientFreeBalanceForDecreasing,

        /// Free balance is insufficient for transferring specfied amount
        InsufficientFreeBalanceForTransfer,

        /// Current total issuance cannot be decrease by specified amount
        InsufficientIssuanceToDecreaseByAmount,

        /// Requested token does not exist
        TokenDoesNotExist,

        /// Requested account data does not exist
        AccountInformationDoesNotExist,

        /// Existential deposit >= initial issuance
        ExistentialDepositExceedsInitialIssuance,

        /// Merkle proof verification failed
        MerkleProofVerificationFailure,

        /// Merkle proof not provided
        MerkleProofNotProvided,

        /// Source and Destination Location coincide
        SameSourceAndDestinationLocations,

        /// Patronage reduction exceeeding patronage rate
        ReductionExceedingPatronageRate,

        /// Symbol already in use
        TokenSymbolAlreadyInUse,

        /// Account Already exists
        AccountAlreadyExists,

        /// Insufficient Balance for Bloat bond
        InsufficientBalanceForBloatBond,

        /// Attempt to removed non owned account under permissioned mode
        AttemptToRemoveNonOwnedAccountUnderPermissionedMode,

        /// Attempt to removed non empty non owned
        AttemptToRemoveNonOwnedAndNonEmptyAccount,

        /// Cannot join whitelist in permissionless mode
        CannotJoinWhitelistInPermissionlessMode,

        /// Cannot Deissue Token with outstanding accounts
        CannotDeissueTokenWithOutstandingAccounts,

    }
}
