use codec::{Decode, Encode};
use frame_support::{
    dispatch::{DispatchError, DispatchResult},
    ensure,
};
use sp_arithmetic::traits::{Saturating, Zero};
use sp_runtime::{traits::Hash, Percent};

// crate imports
use crate::traits::TransferLocationTrait;

pub(crate) enum DecreaseOp<Balance> {
    /// reduce amount by
    Reduce(Balance),

    /// Remove Account (original amonut, dust below ex deposit)
    Remove(Balance, Balance),
}
impl<Balance: Clone + Saturating> DecreaseOp<Balance> {
    pub(crate) fn amount(&self) -> Balance {
        match self {
            Self::Reduce(amount) => amount.to_owned(),
            Self::Remove(amount, _) => amount.to_owned(),
        }
    }
    pub(crate) fn total_amount(&self) -> Balance {
        match self {
            Self::Reduce(amount) => amount.to_owned(),
            Self::Remove(amount, dust) => amount.to_owned().saturating_add(dust.to_owned()),
        }
    }
}

/// Info for the account
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct AccountData<Balance> {
    /// Non-reserved part of the balance. There may still be restrictions
    /// on this, but it is the total pool what may in principle be
    /// transferred, reserved and used for tipping.
    pub(crate) free_balance: Balance,

    /// This balance is a 'reserve' balance that other subsystems use
    /// in order to set aside tokens that are still 'owned' by the
    /// account holder, but which are not usable in any case.
    pub(crate) reserved_balance: Balance,
}

// TODO: add extra type for Reserve = JOY balance different from CRT balance?
/// Info for the token
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, Debug)]
pub struct TokenData<Balance, Hash, BlockNumber> {
    /// Current token issuance
    pub(crate) current_total_issuance: Balance,

    /// Existential deposit allowed for the token
    pub(crate) existential_deposit: Balance,

    /// Initial issuance state
    pub(crate) issuance_state: IssuanceState,

    /// Transfer policy
    pub(crate) transfer_policy: TransferPolicy<Hash>,

    /// Patronage Information
    pub(crate) patronage_info: PatronageData<Balance>,

    /// Revenue Split state info
    pub(crate) revenue_split: SplitState<BlockNumber>,
}

/// Revenue Split State Information
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub enum SplitState<BlockNumber> {
    /// Inactive state: no split ongoing
    Inactive,

    /// Active state: split ongoing with info
    Active(SplitTimeline<BlockNumber>, Percent),
}

/// Revenue Split State Information
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct SplitTimeline<BlockNumber> {
    /// Inactive state: no split ongoing
    pub(crate) start: BlockNumber,

    /// Active state: split ongoing with info
    pub(crate) duration: BlockNumber,
}

/// Patronage information
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, Debug)]
pub struct PatronageData<Balance> {
    /// Outstanding patronage credit
    pub(crate) outstanding_credit: Balance,

    /// Patronage rate
    pub(crate) rate: Percent,
}

/// The two possible transfer policies
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub enum TransferPolicy<Hash> {
    /// Permissionless
    Permissionless,

    /// Permissioned transfer with whitelist commitment
    Permissioned(Hash),
}

impl<Hash> Default for TransferPolicy<Hash> {
    fn default() -> Self {
        TransferPolicy::<Hash>::Permissionless
    }
}

/// The possible issuance variants: This is a stub
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub(crate) enum IssuanceState {
    /// Initial idle state
    Idle,

    /// Initial state sale (this has to be defined)
    Sale,

    /// state for IBCO, it might get decorated with the JOY reserve
    /// amount for the token
    BondingCurve,
}

/// Builder for the token data struct
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default)]
pub struct TokenIssuanceParameters<Balance, Hash> {
    /// Initial issuance
    pub(crate) initial_issuance: Balance,

    /// Initial State builder: stub
    pub(crate) initial_state: IssuanceState,

    /// Initial existential deposit
    pub(crate) existential_deposit: Balance,

    /// Token Symbol
    pub(crate) symbol: Hash,

    /// Initial transfer policy:
    pub(crate) transfer_policy: TransferPolicy<Hash>,

    /// Initial Patronage rate
    pub(crate) patronage_rate: Percent,
}

/// Transfer location without merkle proof
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, Debug)]
pub struct SimpleLocation<AccountId> {
    pub(crate) account: AccountId,
}

/// Transfer location with merkle proof
#[derive(Encode, Decode, Clone, PartialEq, Eq, Default, Debug)]
pub struct VerifiableLocation<AccountId, Hasher: Hash> {
    merkle_proof: Vec<(Hasher::Output, MerkleSide)>,
    pub account: AccountId,
}

/// Utility enum used in merkle proof verification
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, Copy)]
pub enum MerkleSide {
    /// This element appended to the right of the subtree hash
    Right,

    /// This element appended to the left of the subtree hash
    Left,
}

/// Default trait for Merkle Side
impl Default for MerkleSide {
    fn default() -> Self {
        MerkleSide::Right
    }
}

// implementation

/// Default trait for Issuance state
impl Default for IssuanceState {
    fn default() -> Self {
        IssuanceState::Idle
    }
}

/// Default trait for AccountData
impl<Balance: Zero> Default for AccountData<Balance> {
    fn default() -> Self {
        Self {
            free_balance: Balance::zero(),
            reserved_balance: Balance::zero(),
        }
    }
}

/// Encapsules parameters validation + TokenData construction
impl<Balance: Zero + Copy + PartialOrd + Saturating> AccountData<Balance> {
    /// Verify if amount can be decrease taking account existential deposit
    /// Returns the amount that should be removed
    pub(crate) fn decrease_with_ex_deposit<T: crate::Trait>(
        &self,
        amount: Balance,
        existential_deposit: Balance,
    ) -> Result<DecreaseOp<Balance>, DispatchError> {
        ensure!(
            self.free_balance >= amount,
            crate::Error::<T>::InsufficientFreeBalanceForDecreasing,
        );

        let new_total = self
            .free_balance
            .saturating_sub(amount)
            .saturating_add(self.reserved_balance);

        if new_total.is_zero() || new_total < existential_deposit {
            Ok(DecreaseOp::<Balance>::Remove(amount, new_total))
        } else {
            Ok(DecreaseOp::<Balance>::Reduce(amount))
        }
    }

    pub(crate) fn _total_balance(&self) -> Balance {
        self.free_balance.saturating_add(self.reserved_balance)
    }
}
/// Token Data implementation
impl<Balance, Hash, BlockNumber> TokenData<Balance, Hash, BlockNumber> {
    // validate transfer destination location according to self.policy
    pub(crate) fn ensure_valid_location_for_policy<T, AccountId, Location>(
        &self,
        location: &Location,
    ) -> DispatchResult
    where
        T: crate::Trait,
        Location: TransferLocationTrait<AccountId, TransferPolicy<Hash>>,
    {
        ensure!(
            location.is_valid_location_for_policy(&self.transfer_policy),
            crate::Error::<T>::LocationIncompatibleWithCurrentPolicy
        );
        Ok(())
    }
}

// Simple location
impl<AccountId: Clone, Hash> TransferLocationTrait<AccountId, TransferPolicy<Hash>>
    for SimpleLocation<AccountId>
{
    fn is_valid_location_for_policy(&self, policy: &TransferPolicy<Hash>) -> bool {
        matches!(policy, TransferPolicy::<Hash>::Permissionless)
    }

    fn location_account(&self) -> AccountId {
        self.account.to_owned()
    }
}

impl<AccountId> SimpleLocation<AccountId> {
    pub(crate) fn _new(account: AccountId) -> Self {
        Self { account }
    }
}

// Verifiable Location implementation
impl<AccountId: Clone + Encode, Hasher: Hash>
    TransferLocationTrait<AccountId, TransferPolicy<Hasher::Output>>
    for VerifiableLocation<AccountId, Hasher>
{
    fn is_valid_location_for_policy(&self, policy: &TransferPolicy<Hasher::Output>) -> bool {
        // visitee dispatch
        match policy {
            TransferPolicy::<Hasher::Output>::Permissioned(whitelist_commit) => {
                self.is_merkle_proof_valid(whitelist_commit.to_owned())
            }
            // ignore verification in the permissionless case
            TransferPolicy::<Hasher::Output>::Permissionless => true,
        }
    }

    fn location_account(&self) -> AccountId {
        self.account.to_owned()
    }
}

impl<AccountId: Encode, Hasher: Hash> VerifiableLocation<AccountId, Hasher> {
    pub(crate) fn is_merkle_proof_valid(&self, commit: Hasher::Output) -> bool {
        let init = Hasher::hash_of(&self.account);
        let proof_result = self
            .merkle_proof
            .iter()
            .fold(init, |acc, (hash, side)| match side {
                MerkleSide::Left => Hasher::hash_of(&(hash, acc)),
                MerkleSide::Right => Hasher::hash_of(&(acc, hash)),
            });

        proof_result == commit
    }

    pub fn _new(merkle_proof: Vec<(Hasher::Output, MerkleSide)>, account: AccountId) -> Self {
        Self {
            merkle_proof,
            account,
        }
    }
}

impl<BlockNumber> Default for SplitState<BlockNumber> {
    fn default() -> Self {
        SplitState::Inactive
    }
}

impl<BlockNumber: Clone> SplitState<BlockNumber> {
    pub(crate) fn is_active(&self) -> bool {
        matches!(self, SplitState::Active(..))
    }

    pub(crate) fn is_inactive(&self) -> bool {
        matches!(self, SplitState::Inactive)
    }

    pub(crate) fn activate(&mut self, timeline: SplitTimeline<BlockNumber>, percentage: Percent) {
        *self = SplitState::<BlockNumber>::Active(timeline, percentage);
    }
}

/// Auxiliary type: timeline parameters
#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug)]
pub struct SplitTimelineParameters<BlockNumber> {
    pub(crate) start: BlockNumber,
    pub(crate) duration: BlockNumber,
}

impl<BlockNumber: Copy + Saturating + PartialOrd> SplitTimelineParameters<BlockNumber> {
    pub fn new(start: BlockNumber, duration: BlockNumber) -> Self {
        Self { start, duration }
    }

    pub(crate) fn try_build<T: crate::Trait>(
        self,
        now: BlockNumber,
        min_duration: BlockNumber,
    ) -> Result<SplitTimeline<BlockNumber>, DispatchError> {
        ensure!(
            self.start >= now,
            crate::Error::<T>::StartingBlockLowerThanCurrentBlock
        );

        ensure!(
            self.duration >= min_duration,
            crate::Error::<T>::RevenueSplitDurationTooShort,
        );

        Ok(SplitTimeline::<_>::new(self.start, self.duration))
    }
}

impl<BlockNumber: Copy + Saturating + PartialOrd> SplitTimeline<BlockNumber> {
    pub(crate) fn new(start: BlockNumber, duration: BlockNumber) -> Self {
        Self { start, duration }
    }

    pub(crate) fn is_ongoing(&self, now: BlockNumber) -> bool {
        self.end() >= now
    }

    pub(crate) fn end(&self) -> BlockNumber {
        self.start.saturating_add(self.duration)
    }
}

// Aliases
/// Alias for Account Data
pub(crate) type AccountDataOf<T> = AccountData<<T as crate::Trait>::Balance>;

/// Alias for Timeline parameters
pub(crate) type TimelineParamsOf<T> =
    SplitTimelineParameters<<T as frame_system::Trait>::BlockNumber>;

/// Alias for Token Data
pub(crate) type TokenDataOf<T> = TokenData<
    <T as crate::Trait>::Balance,
    <T as frame_system::Trait>::Hash,
    <T as frame_system::Trait>::BlockNumber,
>;

/// Alias for Token Issuance Parameters
pub(crate) type TokenIssuanceParametersOf<T> =
    TokenIssuanceParameters<<T as crate::Trait>::Balance, <T as frame_system::Trait>::Hash>;

/// Alias for TransferPolicy
pub(crate) type TransferPolicyOf<T> = TransferPolicy<<T as frame_system::Trait>::Hash>;

/// Alias for decrease operation
pub(crate) type DecOp<T> = DecreaseOp<<T as crate::Trait>::Balance>;
