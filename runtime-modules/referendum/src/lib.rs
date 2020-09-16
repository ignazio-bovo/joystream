// TODO: module documentation
// TODO: adjust all extrinsic weights

// NOTE: This module is instantiable pallet as described here https://substrate.dev/recipes/3-entrees/instantiable.html
// No default instance is provided.

/////////////////// Configuration //////////////////////////////////////////////
#![cfg_attr(not(feature = "std"), no_std)]

// used dependencies
use codec::{Codec, Decode, Encode};
use frame_support::traits::{
    Currency, EnsureOrigin, Get, LockIdentifier, LockableCurrency, WithdrawReason,
};
use frame_support::{
    decl_error, decl_event, decl_module, decl_storage, error::BadOrigin, Parameter, StorageValue,
};
use sp_arithmetic::traits::BaseArithmetic;
use sp_runtime::traits::{MaybeSerialize, Member};
use std::marker::PhantomData;
use system::ensure_signed;

// declared modules
mod mock;
mod tests;

/////////////////// Data Structures ////////////////////////////////////////////

/// Possible referendum states.
#[derive(Encode, Decode, PartialEq, Eq, Debug)]
pub enum ReferendumStage<BlockNumber, VotePower> {
    /// The referendum is dormant and waiting to be started by external source.
    Inactive,
    /// In the voting stage, users can cast their sealed votes.
    Voting(ReferendumStageVoting<BlockNumber>),
    /// In the revealing stage, users can reveal votes they cast in the voting stage.
    Revealing(ReferendumStageRevealing<BlockNumber, VotePower>),
}

impl<BlockNumber, VotePower: Encode + Decode> Default for ReferendumStage<BlockNumber, VotePower> {
    fn default() -> ReferendumStage<BlockNumber, VotePower> {
        ReferendumStage::Inactive
    }
}

/// Representation for voting stage state.
#[derive(Encode, Decode, PartialEq, Eq, Debug, Default)]
pub struct ReferendumStageVoting<BlockNumber> {
    started: BlockNumber,      // block in which referendum started
    winning_target_count: u64, // target number of winners
}

/// Representation for revealing stage state.
#[derive(Encode, Decode, PartialEq, Eq, Debug, Default)]
pub struct ReferendumStageRevealing<BlockNumber, VotePower> {
    pub started: BlockNumber,      // block in which referendum started
    pub winning_target_count: u64, // target number of winners
    pub intermediate_winners: Vec<OptionResult<VotePower>>, // intermediate winning options
}

#[derive(Encode, Decode, PartialEq, Eq, Debug, Default, Clone)]
pub struct OptionResult<VotePower> {
    pub option_id: u64,
    pub vote_power: VotePower,
}

/// Vote cast in referendum. Vote target is concealed until user reveals commitment's proof.
#[derive(Encode, Decode, PartialEq, Eq, Debug, Default)]
pub struct CastVote<Hash, Currency> {
    commitment: Hash, // a commitment that a user submits in the voting stage before revealing what this vote is actually for
    cycle_id: u64,    // current referendum cycle number
    stake: Currency,  // stake locked for vote
    vote_for: Option<u64>, // target option this vote favors; is `None` before the vote is revealed
}

/////////////////// Type aliases ///////////////////////////////////////////////

// `Ez` prefix in some of the following type aliases means *easy* and is meant to create unique short names
// aliasing existing structs and enums

// types simplifying access to common structs and enums
pub type Balance<T, I> =
    <<T as Trait<I>>::Currency as Currency<<T as system::Trait>::AccountId>>::Balance;
pub type EzCastVote<T, I> = CastVote<<T as system::Trait>::Hash, Balance<T, I>>;
pub type EzReferendumStageVoting<T> = ReferendumStageVoting<<T as system::Trait>::BlockNumber>;
pub type EzReferendumStageRevealing<T, I> =
    ReferendumStageRevealing<<T as system::Trait>::BlockNumber, <T as Trait<I>>::VotePower>;

// types aliases for check functions return values
pub type CanRevealResult<T, I> = (
    EzReferendumStageRevealing<T, I>,
    <T as system::Trait>::AccountId,
    EzCastVote<T, I>,
);

/////////////////// Trait, Storage, Errors, and Events /////////////////////////

// TODO: get rid of dependency on Error<T, I> - create some nongeneric error
/// Trait enabling referendum start and vote commitment calculation.

pub trait ReferendumManager<T: Trait<I>, I: Instance> {
    /// Start a new referendum.
    fn start_referendum(
        origin: T::Origin,
        extra_winning_target_count: u64,
    ) -> Result<(), Error<T, I>>;

    /// Calculate commitment for a vote.
    fn calculate_commitment(
        account_id: &<T as system::Trait>::AccountId,
        salt: &[u8],
        cycle_id: &u64,
        vote_option_id: &u64,
    ) -> T::Hash;
}
/*
pub trait ReferendumManager<Origin, AccountId, Hash, Error> {
    /// Start a new referendum.
    fn start_referendum(
        origin: Origin,
        extra_winning_target_count: u64,
    ) -> Result<(), Error>;

    /// Calculate commitment for a vote.
    fn calculate_commitment(
        account_id: &AccountId,
        salt: &[u8],
        cycle_id: &u64,
        vote_option_id: &u64,
    ) -> Hash;
}
*/
//pub trait Trait<I: Instance>: system::Trait /* + ReferendumManager<Self, I>*/ {
pub trait Trait<I: Instance>: system::Trait {
    /// The overarching event type.
    type Event: From<Event<Self, I>> + Into<<Self as system::Trait>::Event>;

    /// Maximum length of vote commitment salt. Use length that ensures uniqueness for hashing e.g. std::u64::MAX.
    type MaxSaltLength: Get<u64>;

    /// Currency for referendum staking.
    type Currency: LockableCurrency<Self::AccountId, Moment = Self::BlockNumber>;

    /// Identifier for currency locks used for staking.
    type LockId: Get<LockIdentifier>;

    /// Origin from which the referendum can be started.
    type ManagerOrigin: EnsureOrigin<Self::Origin>;

    /// Power of vote(s) used to determine the referendum winner(s).
    type VotePower: Parameter
        + Member
        + BaseArithmetic
        + Codec
        + Default
        + Copy
        + MaybeSerialize
        + PartialEq;

    /// Duration of voting stage (in blocks)
    type VoteStageDuration: Get<Self::BlockNumber>;
    /// Duration of revealing stage (in blocks)
    type RevealStageDuration: Get<Self::BlockNumber>;

    /// Minimum stake needed for voting
    type MinimumStake: Get<Balance<Self, I>>;

    /// Calculate the vote's power for user and his stake.
    fn caclulate_vote_power(
        account_id: &<Self as system::Trait>::AccountId,
        stake: &Balance<Self, I>,
    ) -> <Self as Trait<I>>::VotePower;

    /// Checks if user can unlock his stake from the given vote.
    /// Gives runtime an ability to penalize user for not revealing stake, etc.
    fn can_unstake(vote: &CastVote<Self::Hash, Balance<Self, I>>) -> bool;

    /// Gives runtime an ability to react on referendum result.
    fn process_results(winners: &[OptionResult<Self::VotePower>]);

    /// Check if an option a user is voting for actually exists.
    fn is_valid_option_id(option_id: &u64) -> bool;

    // If the id is a valid alternative, the current total voting mass backing it is returned, otherwise nothing.
    fn get_option_power(option_id: &u64) -> Self::VotePower;

    // Increases voting mass behind given alternative by given amount, if present and return true, otherwise return false.
    fn increase_option_power(option_id: &u64, amount: &Self::VotePower);
}

decl_storage! {
    trait Store for Module<T: Trait<I>, I: Instance> as Referendum {
        /// Current referendum stage.
        pub Stage get(fn stage) config(): ReferendumStage<T::BlockNumber, T::VotePower>;

        /// Votes cast in the referendum. A new record is added to this map when a user casts a sealed vote.
        /// It is modified when a user reveals the vote's commitment proof.
        /// A record is finally removed when the user unstakes, which can happen during a voting stage or after the current cycle ends.
        /// A stake for a vote can be reused in future referendum cycles.
        pub Votes get(fn votes) config(): map hasher(blake2_128_concat) T::AccountId => EzCastVote<T, I>;

        /// Index of the current referendum cycle. It is incremented everytime referendum ends.
        pub CurrentCycleId get(fn current_cycle_id) config(): u64;
    }

    /* This might be needed in some cases
    // add_extra_genesis has to be present in Instantiable Modules - see https://github.com/paritytech/substrate/blob/master/frame/support/procedural/src/lib.rs#L217
    add_extra_genesis {
        config(phantom): PhantomData<I>;
    }
    */
}

decl_event! {
    pub enum Event<T, I>
    where
        Balance = Balance<T, I>,
        <T as system::Trait>::Hash,
        <T as system::Trait>::AccountId,
        <T as Trait<I>>::VotePower,
    {
        /// Referendum started
        ReferendumStarted(u64),

        /// Revealing phase has begun
        RevealingStageStarted(),

        /// Referendum ended and winning option was selected
        ReferendumFinished(Vec<OptionResult<VotePower>>),

        /// User cast a vote in referendum
        VoteCast(AccountId, Hash, Balance),

        /// User revealed his vote
        VoteRevealed(AccountId, u64),

        /// User released his stake
        StakeReleased(AccountId),
    }
}

decl_error! {
    /// Referendum errors
    pub enum Error for Module<T: Trait<I>, I: Instance> {
        /// Origin is invalid
        BadOrigin,

        /// Referendum cannot run twice at the same time
        ReferendumAlreadyRunning,

        /// Referendum is not running when expected to
        ReferendumNotRunning,

        /// Revealing stage is not in progress right now
        RevealingNotInProgress,

        /// Account can't stake enough currency (now)
        InsufficientBalanceToStakeCurrency,

        /// Insufficient stake provided to cast a vote
        InsufficientStake,

        /// Salt and referendum option provided don't correspond to the commitment
        InvalidReveal,

        /// Vote for not existing option was revealed
        InvalidVote,

        /// Trying to reveal vote that was not cast
        VoteNotExisting,

        /// Invalid time to release the locked stake
        InvalidTimeToRelease,

        /// Salt is too long
        SaltTooLong,

        /// Unstaking has been forbidden for the user (at least for now)
        UnstakingForbidden,
    }
}

impl<T: Trait<I>, I: Instance> PartialEq for Error<T, I> {
    fn eq(&self, other: &Self) -> bool {
        self.as_u8() == other.as_u8()
    }
}

impl<T: Trait<I>, I: Instance> From<BadOrigin> for Error<T, I> {
    fn from(_error: BadOrigin) -> Self {
        Error::<T, I>::BadOrigin
    }
}

/////////////////// Module definition and implementation ///////////////////////

decl_module! {
    pub struct Module<T: Trait<I>, I: Instance> for enum Call where origin: T::Origin {
        /// Predefined errors
        type Error = Error<T, I>;

        /// Setup events
        fn deposit_event() = default;

        /////////////////// Lifetime ///////////////////////////////////////////

        // No origin so this is a priviledged call
        fn on_finalize(now: T::BlockNumber) {
            Self::try_progress_stage(now);
        }

        /////////////////// User actions ///////////////////////////////////////

        /// Cast a sealed vote in the referendum.
        #[weight = 10_000_000]
        pub fn vote(origin, commitment: T::Hash, stake: Balance<T, I>) -> Result<(), Error<T, I>> {
            // ensure action can be started
            let account_id = EnsureChecks::<T, I>::can_vote(origin, &stake)?;

            //
            // == MUTATION SAFE ==
            //

            // start revealing phase - it can return error when stake fails to lock
            Mutations::<T, I>::vote(&account_id, &commitment, &stake)?;

            // emit event
            Self::deposit_event(RawEvent::VoteCast(account_id, commitment, stake));

            Ok(())
        }

        /// Reveal a sealed vote in the referendum.
        #[weight = 10_000_000]
        pub fn reveal_vote(origin, salt: Vec<u8>, vote_option_id: u64) -> Result<(), Error<T, I>> {
            let (stage_data, account_id, cast_vote) = EnsureChecks::<T, I>::can_reveal_vote::<Self>(origin, &salt, &vote_option_id)?;

            //
            // == MUTATION SAFE ==
            //

            // reveal the vote - it can return error when stake fails to unlock
            Mutations::<T, I>::reveal_vote(stage_data, &account_id, &vote_option_id, cast_vote)?;

            // emit event
            Self::deposit_event(RawEvent::VoteRevealed(account_id, vote_option_id));

            Ok(())
        }


        /// Release a locked stake.
        #[weight = 10_000_000]
        pub fn release_stake(origin) -> Result<(), Error<T, I>> {
            let account_id = EnsureChecks::<T, I>::can_release_stake(origin)?;

            //
            // == MUTATION SAFE ==
            //

            // reveal the vote - it can return error when stake fails to unlock
            Mutations::<T, I>::release_stake(&account_id);

            // emit event
            Self::deposit_event(RawEvent::StakeReleased(account_id));

            Ok(())
        }
    }
}

/////////////////// Inner logic ////////////////////////////////////////////////

impl<T: Trait<I>, I: Instance> Module<T, I> {
    /// Checkout expire of referendum stage.
    fn try_progress_stage(now: T::BlockNumber) {
        match Stage::<T, I>::get() {
            ReferendumStage::Inactive => (),
            ReferendumStage::Voting(stage_data) => {
                if now == stage_data.started + T::VoteStageDuration::get() {
                    Self::end_voting_period(stage_data);
                }
            }
            ReferendumStage::Revealing(stage_data) => {
                if now == stage_data.started + T::RevealStageDuration::get() {
                    Self::end_reveal_period(stage_data);
                }
            }
        }
    }

    /// Finish voting and start ravealing.
    fn end_voting_period(stage_data: EzReferendumStageVoting<T>) {
        // start revealing phase
        Mutations::<T, I>::start_revealing_period(stage_data);

        // emit event
        Self::deposit_event(RawEvent::RevealingStageStarted());
    }

    /// Conclude the referendum.
    fn end_reveal_period(stage_data: EzReferendumStageRevealing<T, I>) {
        // conclude referendum
        let winners = Mutations::<T, I>::conclude_referendum(stage_data);

        // let runtime know about referendum results
        T::process_results(&winners);

        // emit event
        Self::deposit_event(RawEvent::ReferendumFinished(winners));
    }
}

/////////////////// ReferendumManager //////////////////////////////////////////

impl<T: Trait<I>, I: Instance> ReferendumManager<T, I> for Module<T, I> {
    //impl<T: Trait<I>, I: Instance, Origin, AccountId, Hash, Error> ReferendumManager<Origin, AccountId, Hash, Error> for Module<T, I> {
    //impl<T: Trait<I>, I: Instance> ReferendumManager<T::Origin, T::AccountId, T::Hash, Error<T, I>> for Module<T, I> {
    /*
        /// Start new referendum run.
        fn start_referendum(
            origin: T::Origin,
            extra_winning_target_count: u64,
        ) -> Result<(), Error<T, I>> {
    /*
            fn can_start_referendum<Origin, ManagerOrigin: EnsureOrigin<Origin>, >(origin: Origin) -> Result<(), Error> {
                ManagerOrigin::ensure_origin(origin)?;

                // ensure referendum is not already running
                match Stage::<T, I>::get() {
                    ReferendumStage::Inactive => Ok(()),
                    _ => Err(Error::ReferendumAlreadyRunning),
                }?;

                Ok(())
            }
    */

            let winning_target_count = extra_winning_target_count + 1;

            // ensure action can be started
            EnsureChecks::<T, I>::can_start_referendum(origin)?;

            //
            // == MUTATION SAFE ==
            //

            // update state
            Mutations::<T, I>::start_voting_period(&winning_target_count);

            // emit event
            Self::deposit_event(RawEvent::ReferendumStarted(winning_target_count));

            Ok(())
        }

        /// Calculate commitment for a vote.
        fn calculate_commitment(
            account_id: &<T as system::Trait>::AccountId,
            salt: &[u8],
            cycle_id: &u64,
            vote_option_id: &u64,
        ) -> T::Hash {
            let mut payload = account_id.encode();
            let mut mut_option_id = vote_option_id.encode();
            let mut mut_salt = salt.encode(); //.to_vec();
            let mut mut_cycle_id = cycle_id.encode(); //.to_vec();

            payload.append(&mut mut_option_id);
            payload.append(&mut mut_salt);
            payload.append(&mut mut_cycle_id);

            <T::Hashing as sp_runtime::traits::Hash>::hash(&payload)
        }
    */
    /// Start new referendum run.
    fn start_referendum(
        origin: T::Origin,
        extra_winning_target_count: u64,
    ) -> Result<(), Error<T, I>> {
        let winning_target_count = extra_winning_target_count + 1;

        // ensure action can be started
        EnsureChecks::<T, I>::can_start_referendum(origin)?;

        //
        // == MUTATION SAFE ==
        //

        // update state
        Mutations::<T, I>::start_voting_period(&winning_target_count);

        // emit event
        Self::deposit_event(RawEvent::ReferendumStarted(winning_target_count));

        Ok(())
    }

    /// Calculate commitment for a vote.
    fn calculate_commitment(
        account_id: &<T as system::Trait>::AccountId,
        salt: &[u8],
        cycle_id: &u64,
        vote_option_id: &u64,
    ) -> T::Hash {
        let mut payload = account_id.encode();
        let mut mut_option_id = vote_option_id.encode();
        let mut mut_salt = salt.encode(); //.to_vec();
        let mut mut_cycle_id = cycle_id.encode(); //.to_vec();

        payload.append(&mut mut_option_id);
        payload.append(&mut mut_salt);
        payload.append(&mut mut_cycle_id);

        <T::Hashing as sp_runtime::traits::Hash>::hash(&payload)
    }
}

/////////////////// Mutations //////////////////////////////////////////////////

struct Mutations<T: Trait<I>, I: Instance> {
    _dummy: PhantomData<(T, I)>, // 0-sized data meant only to bound generic parameters
}

impl<T: Trait<I>, I: Instance> Mutations<T, I> {
    /// Change the referendum stage from inactive to voting stage.
    fn start_voting_period(winning_target_count: &u64) {
        // change referendum state
        Stage::<T, I>::put(ReferendumStage::Voting(ReferendumStageVoting::<
            T::BlockNumber,
        > {
            started: <system::Module<T>>::block_number(),
            winning_target_count: *winning_target_count,
        }));
    }

    /// Change the referendum stage from inactive to the voting stage.
    fn start_revealing_period(old_stage: EzReferendumStageVoting<T>) {
        // change referendum state
        Stage::<T, I>::put(ReferendumStage::Revealing(EzReferendumStageRevealing::<
            T,
            I,
        > {
            started: <system::Module<T>>::block_number(),
            winning_target_count: old_stage.winning_target_count,
            intermediate_winners: vec![],
        }));
    }

    /// Conclude referendum, count votes, and select the winners.
    //fn conclude_referendum(revealing_stage: EzReferendumStageRevealing<T, I>) -> Vec<OptionResult<T::VotePower>> {
    fn conclude_referendum(
        revealing_stage: EzReferendumStageRevealing<T, I>,
    ) -> Vec<OptionResult<<T as Trait<I>>::VotePower>> {
        // reset referendum state
        Self::reset_referendum();

        // return winning option
        revealing_stage.intermediate_winners
    }

    /// Change the referendum stage from revealing to the inactive stage.
    fn reset_referendum() {
        Stage::<T, I>::put(ReferendumStage::Inactive);
        CurrentCycleId::<I>::put(CurrentCycleId::<I>::get() + 1);
    }

    /// Cast a user's sealed vote for the current referendum cycle.
    fn vote(
        account_id: &<T as system::Trait>::AccountId,
        commitment: &T::Hash,
        stake: &Balance<T, I>,
    ) -> Result<(), Error<T, I>> {
        // lock stake amount
        T::Currency::set_lock(
            T::LockId::get(),
            account_id,
            *stake,
            WithdrawReason::Transfer.into(),
        );

        // store vote
        Votes::<T, I>::insert(
            account_id,
            CastVote {
                commitment: *commitment,
                stake: *stake,
                cycle_id: CurrentCycleId::<I>::get(),
                vote_for: None,
            },
        );

        Ok(())
    }

    /// Reveal user's vote target and check the commitment proof.
    fn reveal_vote(
        stage_data: EzReferendumStageRevealing<T, I>,
        account_id: &<T as system::Trait>::AccountId,
        option_id: &u64,
        cast_vote: EzCastVote<T, I>,
    ) -> Result<(), Error<T, I>> {
        /// Moves winner to new position in winners list. Expects `target_index` to be always smaller or equal to `current_index`.
        fn move_winner<T: Trait<I>, I: Instance>(
            current_winners: &[OptionResult<T::VotePower>],
            current_index: usize,
            target_index: usize,
            new_vote_power: T::VotePower,
        ) -> Vec<OptionResult<T::VotePower>> {
            let tmp = [OptionResult {
                vote_power: new_vote_power,
                ..current_winners[current_index].clone()
            }];
            let list_size = current_winners.len();
            let final_target = if target_index >= list_size {
                list_size - 1
            } else {
                target_index
            };

            // item is in right spot? just update value
            if final_target == current_index {
                return [
                    &current_winners[0..final_target],
                    &tmp[..],
                    &current_winners[final_target + 1..],
                ]
                .concat();
            }

            [
                &current_winners[0..final_target],
                &tmp[..],
                &current_winners[final_target..current_index],
                &current_winners[current_index + 1..current_winners.len()],
            ]
            .concat()
        }

        /// Tries to insert option to the winners list
        fn try_winner_insert<T: Trait<I>, I: Instance>(
            option_result: &OptionResult<T::VotePower>,
            current_winners: &[OptionResult<T::VotePower>],
            winning_target_count: u64,
        ) -> Option<Vec<OptionResult<T::VotePower>>> {
            let current_winners_count = current_winners.len();

            // find where should the vote be inserted into winners list
            let mut insert_index: usize = current_winners_count; // set initial index higher than last winner's index
            for (index, value) in current_winners.iter().enumerate() {
                if option_result.vote_power > value.vote_power {
                    insert_index = index;
                    break;
                }
            }

            // no need to insert?
            if insert_index >= winning_target_count as usize {
                return None;
            }

            // check if option is already somewhere in list
            let mut already_existing_index: Option<usize> = None;
            for (index, value) in current_winners.iter().enumerate() {
                if option_result.option_id == value.option_id {
                    already_existing_index = Some(index);
                    break;
                }
            }

            // option is already in the list and only needs to change it's position?
            if let Some(current_index) = already_existing_index {
                return Some(move_winner::<T, I>(
                    current_winners,
                    current_index,
                    insert_index,
                    option_result.vote_power,
                ));
            }

            let tmp = [option_result.clone()];

            // are we appending at the end of list?
            if insert_index >= current_winners_count {
                return Some([&current_winners, &tmp[..]].concat());
            }

            // insert into middle of list
            Some(
                [
                    &current_winners[0..insert_index],
                    &tmp[..],
                    &current_winners[insert_index..current_winners_count],
                ]
                .concat()[0..winning_target_count as usize]
                    .to_vec(),
            )
        }

        // prepare new values
        let vote_power = T::caclulate_vote_power(&account_id, &cast_vote.stake);
        let old_option_total = T::get_option_power(option_id);
        let new_option_total = old_option_total + vote_power;
        let option_result = OptionResult {
            option_id: *option_id,
            vote_power: new_option_total,
        };
        let new_winners = match try_winner_insert::<T, I>(
            &option_result,
            &stage_data.intermediate_winners,
            stage_data.winning_target_count,
        ) {
            Some(tmp_winners) => tmp_winners,
            None => stage_data.intermediate_winners.clone(),
        };
        let new_stage_data = ReferendumStageRevealing {
            intermediate_winners: new_winners,
            ..stage_data
        };

        T::increase_option_power(option_id, &vote_power);

        // store revealed vote
        Stage::<T, I>::mutate(|stage| *stage = ReferendumStage::Revealing(new_stage_data));

        // remove user commitment to prevent repeated revealing
        Votes::<T, I>::mutate(account_id, |vote| (*vote).vote_for = Some(*option_id));

        Ok(())
    }

    /// Release stake associated to the user's last vote.
    fn release_stake(account_id: &<T as system::Trait>::AccountId) {
        // lock stake amount
        T::Currency::remove_lock(T::LockId::get(), account_id);

        // remove vote record
        Votes::<T, I>::remove(account_id);
    }
}

/////////////////// Ensure checks //////////////////////////////////////////////

struct EnsureChecks<T: Trait<I>, I: Instance> {
    _dummy: PhantomData<(T, I)>, // 0-sized data meant only to bound generic parameters
}

impl<T: Trait<I>, I: Instance> EnsureChecks<T, I> {
    /////////////////// Common checks //////////////////////////////////////////

    fn ensure_regular_user(origin: T::Origin) -> Result<T::AccountId, Error<T, I>> {
        let account_id = ensure_signed(origin)?;

        Ok(account_id)
    }

    /////////////////// Action checks //////////////////////////////////////////

    fn can_start_referendum(origin: T::Origin) -> Result<(), Error<T, I>> {
        T::ManagerOrigin::ensure_origin(origin)?;

        // ensure referendum is not already running
        match Stage::<T, I>::get() {
            ReferendumStage::Inactive => Ok(()),
            _ => Err(Error::ReferendumAlreadyRunning),
        }?;

        Ok(())
    }

    fn can_vote(origin: T::Origin, stake: &Balance<T, I>) -> Result<T::AccountId, Error<T, I>> {
        // ensure superuser requested action
        let account_id = Self::ensure_regular_user(origin)?;

        let stage = Stage::<T, I>::get();

        // ensure referendum is running
        match stage {
            ReferendumStage::Voting(_) => (),
            _ => return Err(Error::ReferendumNotRunning),
        };

        // ensure stake is enough for voting
        if stake < &T::MinimumStake::get() {
            return Err(Error::InsufficientStake);
        }

        // ensure account can lock the stake
        if T::Currency::total_balance(&account_id) < *stake {
            return Err(Error::InsufficientBalanceToStakeCurrency);
        }

        Ok(account_id)
    }

    fn can_reveal_vote<R: ReferendumManager<T, I>>(
        origin: T::Origin,
        salt: &[u8],
        vote_option_id: &u64,
    ) -> Result<CanRevealResult<T, I>, Error<T, I>> {
        let cycle_id = CurrentCycleId::<I>::get();

        // ensure superuser requested action
        let account_id = Self::ensure_regular_user(origin)?;

        let stage = Stage::<T, I>::get();

        // ensure referendum is running
        let stage_data = match stage {
            ReferendumStage::Revealing(tmp_stage_data) => tmp_stage_data,
            _ => return Err(Error::RevealingNotInProgress),
        };

        let cast_vote = Self::ensure_vote_exists(&account_id)?;

        if !T::is_valid_option_id(vote_option_id) {
            return Err(Error::InvalidVote);
        }

        // ensure vote was cast for the running referendum
        if cycle_id != cast_vote.cycle_id {
            return Err(Error::InvalidVote);
        }

        // ensure salt is not too long
        if salt.len() as u64 > T::MaxSaltLength::get() {
            return Err(Error::SaltTooLong);
        }

        // ensure commitment corresponds to salt and vote option
        let commitment = R::calculate_commitment(&account_id, salt, &cycle_id, vote_option_id);
        if commitment != cast_vote.commitment {
            return Err(Error::InvalidReveal);
        }

        Ok((stage_data, account_id, cast_vote))
    }

    fn can_release_stake(origin: T::Origin) -> Result<T::AccountId, Error<T, I>> {
        let cycle_id = CurrentCycleId::<I>::get();

        // ensure superuser requested action
        let account_id = Self::ensure_regular_user(origin)?;

        let cast_vote = Self::ensure_vote_exists(&account_id)?;

        if !T::can_unstake(&cast_vote) {
            return Err(Error::UnstakingForbidden);
        }

        // enable stake release in current cycle only during voting stage
        if cycle_id == cast_vote.cycle_id {
            match Stage::<T, I>::get() {
                ReferendumStage::Voting(_) => Ok(()),
                _ => Err(Error::InvalidTimeToRelease),
            }?;
        }

        // eliminate possibility of unexpected cycle_id
        if cycle_id < cast_vote.cycle_id {
            return Err(Error::InvalidTimeToRelease);
        }

        Ok(account_id)
    }

    fn ensure_vote_exists(account_id: &T::AccountId) -> Result<EzCastVote<T, I>, Error<T, I>> {
        // ensure there is some vote with locked stake
        if !Votes::<T, I>::contains_key(account_id) {
            return Err(Error::VoteNotExisting);
        }

        let cast_vote = Votes::<T, I>::get(account_id);

        Ok(cast_vote)
    }
}
