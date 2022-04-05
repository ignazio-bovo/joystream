#[cfg(test)]
use frame_support::{assert_noop, assert_ok, traits::Currency};
use sp_runtime::{traits::AccountIdConversion, Percent};

use crate::tests::mock::*;
use crate::tests::test_utils::{increase_account_balance, TokenDataBuilder};
use crate::traits::PalletToken;
use crate::{account, balance, last_event_eq, token, Error, RawEvent};

// helper macros
#[macro_export]
macro_rules! time_params {
    ($s:expr,$d:expr) => {
        TimelineParams::new($s, $d)
    };
}

#[macro_export]
macro_rules! timeline {
    ($s:expr,$d:expr) => {
        SplitTimelineOf::new($s, $d)
    };
}

#[macro_export]
macro_rules! block {
    ($b:expr) => {
        BlockNumber::from($b as u32)
    };
}

#[macro_export]
macro_rules! percent {
    ($p:expr) => {
        Percent::from_percent($p)
    };
}

#[macro_export]
macro_rules! joys {
    ($b:expr) => {
        ReserveBalance::from($b as u32)
    };
}

#[macro_export]
macro_rules! treasury {
    ($t:expr) => {
        TokenModuleId::get().into_sub_account::<AccountId>($t)
    };
}

#[test]
fn issue_split_fails_with_invalid_token_id() {
    let token_id = token!(1);
    let config = GenesisConfigBuilder::new_empty().build();
    let timeline_p = time_params!(block!(1), block!(10));
    let (src, allocation) = (account!(1), joys!(100));
    let percentage = Percent::from_percent(10);

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(src, allocation);
        let result = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::issue_revenue_split(
            token_id, timeline_p, src, allocation, percentage,
        );

        assert_noop!(result, Error::<Test>::TokenDoesNotExist);
    })
}

#[test]
fn issue_split_fails_with_invalid_starting_block() {
    let token_id = token!(1);
    let token_data = TokenDataBuilder::new_empty().build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();
    let timeline_p = time_params!(block!(1), block!(10));
    let (src, allocation) = (account!(1), joys!(100));
    let percentage = Percent::from_percent(10);

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(src, allocation);
        increase_block_number_by(block!(2));

        let result = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::issue_revenue_split(
            token_id, timeline_p, src, allocation, percentage,
        );

        assert_noop!(result, Error::<Test>::StartingBlockLowerThanCurrentBlock);
    })
}

#[test]
fn issue_split_fails_with_duration_too_short() {
    let token_id = token!(1);
    let token_data = TokenDataBuilder::new_empty().build();
    let timeline_p = time_params!(block!(1), MinRevenueSplitDuration::get() - block!(1));
    let (src, allocation) = (account!(1), joys!(100));
    let percentage = Percent::from_percent(10);

    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(src, allocation);

        let result = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::issue_revenue_split(
            token_id, timeline_p, src, allocation, percentage,
        );

        assert_noop!(result, Error::<Test>::RevenueSplitDurationTooShort);
    })
}

#[test]
fn issue_split_fails_with_source_having_insufficient_balance() {
    let token_id = token!(1);
    let token_data = TokenDataBuilder::new_empty().build();
    let timeline_p = time_params!(block!(1), block!(10));
    let (src, allocation) = (account!(1), joys!(100));
    let percentage = Percent::from_percent(10);

    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(src, joys!(0));

        let result = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::issue_revenue_split(
            token_id, timeline_p, src, allocation, percentage,
        );

        assert_noop!(
            result,
            Error::<Test>::InsufficientBalanceForSpecifiedAllocation
        );
    })
}

#[test]
fn issue_split_fails_with_split_already_active() {
    let token_id = token!(1);
    let timeline_p = time_params!(block!(1), block!(10));
    let timeline = timeline!(block!(1), block!(10));
    let (src, allocation) = (account!(1), joys!(50));
    let percentage = Percent::from_percent(10);

    let token_data = TokenDataBuilder::new_empty()
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(src, joys!(100));

        let result = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::issue_revenue_split(
            token_id, timeline_p, src, allocation, percentage,
        );

        assert_noop!(result, Error::<Test>::RevenueSplitAlreadyActiveForToken);
    })
}

#[test]
fn issue_split_fails_with_non_existing_source_account() {
    let token_id = token!(1);
    let token_data = TokenDataBuilder::new_empty().build();
    let timeline_p = time_params!(block!(1), block!(10));
    let percentage = Percent::from_percent(10);
    let (src, allocation) = (account!(1), joys!(100));

    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        let result = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::issue_revenue_split(
            token_id, timeline_p, src, allocation, percentage,
        );

        assert_noop!(
            result,
            Error::<Test>::InsufficientBalanceForSpecifiedAllocation
        );
    })
}

#[test]
fn issue_split_ok() {
    let token_id = token!(1);
    let token_data = TokenDataBuilder::new_empty().build();
    let timeline_p = time_params!(block!(1), block!(10));
    let (src, allocation) = (account!(1), joys!(100));
    let percentage = Percent::from_percent(10);

    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(src, allocation);

        let result = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::issue_revenue_split(
            token_id, timeline_p, src, allocation, percentage,
        );

        assert_ok!(result);
    })
}

#[test]
fn issue_split_ok_with_event_deposit() {
    let token_id = token!(1);
    let token_data = TokenDataBuilder::new_empty().build();
    let timeline_p = time_params!(block!(1), block!(10));
    let (src, allocation) = (account!(1), joys!(100));
    let percentage = Percent::from_percent(10);

    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(src, allocation);

        let _ = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::issue_revenue_split(
            token_id,
            timeline_p.clone(),
            src,
            allocation,
            percentage,
        );

        last_event_eq!(RawEvent::RevenueSplitIssued(
            token_id,
            timeline_p.start,
            timeline_p.duration,
            allocation,
            percentage,
        ));
    })
}

#[test]
fn issue_split_ok_with_correct_activation() {
    let token_id = token!(1);
    let token_data = TokenDataBuilder::new_empty().build();
    let percentage = Percent::from_percent(10);
    let timeline_p = time_params!(block!(1), block!(10));
    let (src, allocation) = (account!(1), joys!(100));

    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(src, allocation);

        let _ = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::issue_revenue_split(
            token_id,
            timeline_p.clone(),
            src,
            allocation,
            percentage,
        );

        assert_eq!(
            Token::token_info_by_id(token_id).revenue_split,
            SplitStateOf::Active(
                SplitTimelineOf {
                    start: timeline_p.start,
                    duration: timeline_p.duration
                },
                percentage
            ),
        );
    })
}

#[test]
fn issue_split_ok_with_allocation_transfer() {
    let token_id = token!(1);
    let token_data = TokenDataBuilder::new_empty().build();
    let timeline_p = time_params!(block!(1), block!(10));
    let (src, allocation) = (account!(1), joys!(100));
    let treasury = TokenModuleId::get().into_sub_account(token_id);
    let percentage = Percent::from_percent(10);

    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(src, allocation);

        let _ = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::issue_revenue_split(
            token_id, timeline_p, src, allocation, percentage,
        );

        assert_eq!(Balances::total_balance(&treasury), allocation);
    })
}

#[test]
fn finalize_split_fails_with_invalid_token_id() {
    let token_id = token!(1);
    let timeline = timeline!(block!(1), block!(10));
    let (src, allocation, percentage) = (account!(1), joys!(50), percent!(10));
    let treasury = TokenModuleId::get().into_sub_account(token_id);

    let token_data = TokenDataBuilder::new_empty()
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);

        let result =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::finalize_revenue_split(
                token_id + 1,
                src,
            );

        assert_noop!(result, Error::<Test>::TokenDoesNotExist);
    })
}

#[test]
fn finalize_split_fails_with_inactive_split_status() {
    let token_id = token!(1);
    let _timeline = (block!(1), block!(10));
    let (src, allocation, _percentage) = (account!(1), joys!(50), percent!(10));
    let treasury = TokenModuleId::get().into_sub_account(token_id);

    let token_data = TokenDataBuilder::new_empty().build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);

        let result =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::finalize_revenue_split(
                token_id, src,
            );

        assert_noop!(result, Error::<Test>::RevenueSplitNotActiveForToken);
    })
}

#[test]
fn finalize_split_fails_with_split_end_block_not_reached() {
    let token_id = token!(1);
    let timeline = timeline!(block!(1), block!(10));
    let (src, allocation, percentage) = (account!(1), joys!(50), percent!(10));
    let treasury = TokenModuleId::get().into_sub_account(token_id);

    let token_data = TokenDataBuilder::new_empty()
        .with_revenue_split(timeline.clone(), percentage)
        .build();

    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);

        let result =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::finalize_revenue_split(
                token_id, src,
            );

        assert_noop!(result, Error::<Test>::RevenueSplitDidNotEnd);
    })
}

#[test]
fn finalize_split_ok() {
    let token_id = token!(1);
    let timeline = timeline!(block!(1), block!(10));
    let (src, leftovers, percentage) = (account!(1), joys!(50), percent!(10));
    let treasury = TokenModuleId::get().into_sub_account(token_id);

    let token_data = TokenDataBuilder::new_empty()
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, leftovers);
        increase_block_number_by(timeline.end() + 1);

        let result =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::finalize_revenue_split(
                token_id, src,
            );

        assert_ok!(result);
    })
}

#[test]
fn finalize_split_ok_with_event_deposit() {
    let token_id = token!(1);
    let timeline = timeline!(block!(1), block!(10));
    let (src, leftovers, percentage) = (account!(1), joys!(50), percent!(10));
    let treasury = TokenModuleId::get().into_sub_account(token_id);

    let token_data = TokenDataBuilder::new_empty()
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, leftovers);
        increase_block_number_by(timeline.end() + 1);

        let _ = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::finalize_revenue_split(
            token_id, src,
        );

        last_event_eq!(RawEvent::RevenueSplitFinalized(token_id, src, leftovers));
    })
}

#[test]
fn finalize_split_ok_with_treasury_account_having_zero_balance() {
    let token_id = token!(1);
    let timeline = timeline!(block!(1), block!(10));
    let (src, leftovers, percentage) = (account!(1), joys!(50), percent!(10));
    let treasury = TokenModuleId::get().into_sub_account(token_id);

    let token_data = TokenDataBuilder::new_empty()
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, leftovers);
        increase_block_number_by(timeline.end() + 1);

        let _ = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::finalize_revenue_split(
            token_id, src,
        );

        assert_eq!(Balances::free_balance(treasury), joys!(0));
    })
}

#[test]
fn finalize_split_ok_with_recovery_account_receiving_leftovers() {
    let token_id = token!(1);
    let timeline = timeline!(block!(1), block!(10));
    let (src, leftovers, percentage) = (account!(1), joys!(50), percent!(10));
    let treasury = TokenModuleId::get().into_sub_account(token_id);

    let token_data = TokenDataBuilder::new_empty()
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, leftovers);
        increase_block_number_by(timeline.end() + 1);

        let _ = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::finalize_revenue_split(
            token_id, src,
        );

        assert_eq!(leftovers, Balances::free_balance(src));
    })
}

#[test]
fn finalize_split_ok_with_split_status_made_inactive() {
    let token_id = token!(1);
    let timeline = timeline!(block!(1), block!(10));
    let (src, leftovers, percentage) = (account!(1), joys!(50), percent!(10));
    let treasury = TokenModuleId::get().into_sub_account(token_id);

    let token_data = TokenDataBuilder::new_empty()
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, leftovers);
        increase_block_number_by(timeline.end() + 1);

        let _ = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::finalize_revenue_split(
            token_id, src,
        );

        assert!(Token::token_info_by_id(token_id)
            .revenue_split
            .is_inactive());
    })
}

#[test]
fn participate_to_split_fails_with_invalid_token_id() {
    let token_id = token!(1);
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(50), percent!(10));
    let (participant_id, to_stake) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty()
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, to_stake, 0)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);

        let result =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::participate_to_split(
                token_id + 1,
                participant_id,
                to_stake,
            );

        assert_noop!(result, Error::<Test>::TokenDoesNotExist);
    })
}

#[test]
fn participate_to_split_fails_with_invalid_account_id() {
    let token_id = token!(1);
    let timeline = timeline!(block!(1), block!(10));
    let (allocation, percentage) = (joys!(50), percent!(10));
    let (participant_id, to_stake) = (account!(2), balance!(100));
    let treasury = TokenModuleId::get().into_sub_account(token_id);

    let token_data = TokenDataBuilder::new_empty()
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);

        let result =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::participate_to_split(
                token_id,
                participant_id,
                to_stake,
            );

        assert_noop!(result, Error::<Test>::AccountInformationDoesNotExist);
    })
}

#[test]
fn participate_to_split_fails_with_token_not_in_active_split_state() {
    let token_id = token!(1);
    let _timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, _percentage) = (treasury!(token_id), joys!(50), percent!(10));
    let (participant_id, to_stake) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty().build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, to_stake, 0)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);

        let result =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::participate_to_split(
                token_id,
                participant_id,
                to_stake,
            );

        assert_noop!(result, Error::<Test>::RevenueSplitNotActiveForToken);
    })
}

#[test]
fn participate_to_split_fails_with_insufficient_token_to_stake() {
    let token_id = token!(1);
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(50), percent!(10));
    let (participant_id, to_stake) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty()
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, balance!(0), 0) // 0 free balance
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);

        let result =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::participate_to_split(
                token_id,
                participant_id,
                to_stake,
            );

        assert_noop!(result, Error::<Test>::InsufficientFreeBalanceForReserving);
    })
}

#[test]
fn participate_to_split_fails_with_active_state_but_ended_timeline() {
    let token_id = token!(1);
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(50), percent!(10));
    let (participant_id, to_stake) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty()
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, balance!(0), 0) // 0 free balance
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);
        increase_block_number_by(timeline.end());

        let result =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::participate_to_split(
                token_id,
                participant_id,
                to_stake,
            );

        assert_noop!(result, Error::<Test>::RevenueSplitHasEnded);
    })
}

#[test]
fn participate_to_split_fails_with_previous_reserved_amount_outstanding() {
    let token_id = token!(1);
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(50), percent!(10));
    let (participant_id, to_stake, staked) = (account!(2), balance!(100), balance!(10));

    let token_data = TokenDataBuilder::new_empty()
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, to_stake, staked)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);

        let result =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::participate_to_split(
                token_id,
                participant_id,
                to_stake,
            );

        assert_noop!(result, Error::<Test>::PreviousReservedAmountOutstanding);
    })
}

#[test]
fn participate_to_split_ok() {
    let token_id = token!(1);
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(50), percent!(10));
    let (participant_id, to_stake) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty()
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, to_stake, 0)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);

        let result =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::participate_to_split(
                token_id,
                participant_id,
                to_stake,
            );

        assert_ok!(result);
    })
}

#[test]
fn participate_to_split_ok_with_event_deposit() {
    let token_id = token!(1);
    let timeline = timeline!(block!(1), block!(10));
    let (allocation, percentage) = (joys!(50), percent!(10));
    let (participant_id, to_stake) = (account!(2), balance!(100));
    let treasury = TokenModuleId::get().into_sub_account(token_id);

    let token_data = TokenDataBuilder::new_empty()
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, to_stake, 0)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);

        let _ = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::participate_to_split(
            token_id,
            participant_id,
            to_stake,
        );

        last_event_eq!(RawEvent::UserParticipatedToSplit(
            token_id,
            participant_id,
            to_stake,
            block!(1),
        ))
    })
}

#[test]
fn participate_to_split_ok_with_user_account_not_delete_due_to_existential_deposit() {
    let (token_id, ex_deposit) = (token!(1), balance!(10));
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(50), percent!(10));
    let (participant_id, to_stake) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty()
        .with_revenue_split(timeline.clone(), percentage)
        .with_existential_deposit(ex_deposit)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, to_stake + ex_deposit - 1, 0)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);

        let _ = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::participate_to_split(
            token_id,
            participant_id,
            to_stake,
        );

        assert_eq!(
            Token::account_info_by_token_and_account(token_id, participant_id).liquidity,
            ex_deposit - 1,
        );
    })
}

#[test]
fn participate_to_split_ok_with_user_account_free_balance_amount_decreased() {
    let token_id = token!(1);
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(50), percent!(10));
    let (participant_id, to_stake) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty()
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, to_stake, 0)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);

        let _ = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::participate_to_split(
            token_id,
            participant_id,
            to_stake,
        );

        assert_eq!(
            Token::account_info_by_token_and_account(token_id, participant_id).reserved_balance,
            to_stake,
        );
    })
}

#[test]
fn participate_to_split_ok_with_user_account_reserved_amount_increased() {
    let token_id = token!(1);
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(50), percent!(10));
    let (participant_id, to_stake) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty()
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, to_stake, 0)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);

        let _ = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::participate_to_split(
            token_id,
            participant_id,
            to_stake,
        );

        assert_eq!(
            Token::account_info_by_token_and_account(token_id, participant_id).liquidity,
            balance!(0),
        );
    })
}

#[test]
fn claim_split_revenue_fails_with_invalid_token_id() {
    let (token_id, issuance) = (token!(1), balance!(1_000));
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(50), percent!(10));
    let (participant_id, staked) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty()
        .with_issuance(issuance)
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, 0, staked)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);
        increase_block_number_by(timeline.end());

        let result =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::claim_revenue_split_amount(
                token_id + 1,
                participant_id,
            );

        assert_noop!(result, Error::<Test>::TokenDoesNotExist);
    })
}

#[test]
fn claim_split_revenue_fails_with_invalid_account_id() {
    let (token_id, issuance) = (token!(1), balance!(1_000));
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(50), percent!(10));
    let (participant_id, staked) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty()
        .with_issuance(issuance)
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, 0, staked)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);
        increase_block_number_by(timeline.end());

        let result =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::claim_revenue_split_amount(
                token_id,
                participant_id + 1,
            );

        assert_noop!(result, Error::<Test>::AccountInformationDoesNotExist);
    })
}

#[test]
fn claim_split_revenue_fails_with_inactive_revenue_split_state() {
    let (token_id, issuance) = (token!(1), balance!(1_000));
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(50), percent!(10));
    let (participant_id, staked) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty()
        .with_issuance(issuance)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, 0, staked)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);
        increase_block_number_by(timeline.end());

        let result =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::claim_revenue_split_amount(
                token_id,
                participant_id + 1,
            );

        assert_noop!(result, Error::<Test>::RevenueSplitNotActiveForToken);
    })
}

#[test]
fn claim_split_revenue_fails_with_active_state_and_timeline_not_ended() {
    let (token_id, issuance) = (token!(1), balance!(1_000));
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(50), percent!(10));
    let (participant_id, staked) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty()
        .with_issuance(issuance)
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, 0, staked)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);
        increase_block_number_by(timeline.end() - 1);

        let result =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::claim_revenue_split_amount(
                token_id,
                participant_id,
            );

        assert_noop!(result, Error::<Test>::RevenueSplitDidNotEnd);
    })
}

#[test]
fn claim_split_revenue_ok() {
    let (token_id, issuance) = (token!(1), balance!(1_000));
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(50), percent!(10));
    let (participant_id, staked, _revenue) = (account!(2), balance!(100), joys!(10));

    let token_data = TokenDataBuilder::new_empty()
        .with_issuance(issuance)
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, 0, staked)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);
        increase_block_number_by(timeline.end());

        let result =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::claim_revenue_split_amount(
                token_id,
                participant_id,
            );

        assert_ok!(result);
    })
}

#[test]
fn claim_split_revenue_ok_with_event_deposit() {
    let (token_id, pre_issuance) = (token!(1), balance!(900));
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(1000), percent!(10));
    let (participant_id, staked, revenue) = (account!(2), balance!(100), joys!(10));

    let token_data = TokenDataBuilder::new_empty()
        .with_issuance(pre_issuance)
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, 0, staked) // total issuance = pre_issuance + staked
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);
        increase_block_number_by(timeline.end());

        let _ =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::claim_revenue_split_amount(
                token_id,
                participant_id,
            );

        last_event_eq!(RawEvent::UserClaimedRevenueSplit(
            token_id,
            participant_id,
            revenue,
            timeline.end() + 1 // end + starting block
        ));
    })
}

#[test]
fn claim_split_revenue_ok_with_treasury_funds_decreased() {
    let (token_id, pre_issuance) = (token!(1), balance!(900));
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(1000), percent!(10));
    let (participant_id, staked, revenue) = (account!(2), balance!(100), joys!(10));

    let token_data = TokenDataBuilder::new_empty()
        .with_issuance(pre_issuance)
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, 0, staked) // total issuance = pre_issuance + staked
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);
        increase_block_number_by(timeline.end());

        let _ =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::claim_revenue_split_amount(
                token_id,
                participant_id,
            );
        assert_eq!(Balances::free_balance(treasury), allocation - revenue);
    })
}

#[test]
fn claim_split_revenue_ok_with_user_funds_increased() {
    let (token_id, pre_issuance) = (token!(1), balance!(900));
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(1000), percent!(10));
    let (participant_id, staked, revenue) = (account!(2), balance!(100), joys!(10));

    let token_data = TokenDataBuilder::new_empty()
        .with_issuance(pre_issuance)
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, 0, staked) // total_issuance = pre_issuance + staked
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);
        increase_block_number_by(timeline.end());

        let _ =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::claim_revenue_split_amount(
                token_id,
                participant_id,
            );
        assert_eq!(Balances::free_balance(participant_id), revenue);
    })
}

#[test]
fn claim_split_revenue_ok_with_user_reserved_amount_reset() {
    let (token_id, pre_issuance) = (token!(1), balance!(1_000));
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(1000), percent!(10));
    let (participant_id, staked, _revenue) = (account!(2), balance!(100), joys!(10));

    let token_data = TokenDataBuilder::new_empty()
        .with_issuance(pre_issuance)
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, 0, staked)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);
        increase_block_number_by(timeline.end());

        let _ =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::claim_revenue_split_amount(
                token_id,
                participant_id,
            );
        assert_eq!(
            Token::account_info_by_token_and_account(token_id, participant_id).reserved_balance,
            balance!(0)
        );
    })
}

#[test]
fn claim_split_revenue_ok_noop_with_user_having_no_stacked_funds() {
    let (token_id, pre_issuance) = (token!(1), balance!(900));
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(1000), percent!(10));
    let (participant_id, _staked, _revenue) = (account!(2), balance!(100), joys!(10));

    let token_data = TokenDataBuilder::new_empty()
        .with_issuance(pre_issuance)
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, 0, 0)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);
        increase_block_number_by(timeline.end());

        let result =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::claim_revenue_split_amount(
                token_id,
                participant_id,
            );
        assert_ok!(result);
    })
}

#[test]
fn claim_split_revenue_ok_with_user_free_balance_increased() {
    let (token_id, issuance) = (token!(1), balance!(900));
    let timeline = timeline!(block!(1), block!(10));
    let (treasury, allocation, percentage) = (treasury!(token_id), joys!(1000), percent!(10));
    let (participant_id, staked, _revenue) = (account!(2), balance!(100), joys!(10));

    let token_data = TokenDataBuilder::new_empty()
        .with_issuance(issuance)
        .with_revenue_split(timeline.clone(), percentage)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, 0, staked)
        .build();

    build_test_externalities(config).execute_with(|| {
        increase_account_balance(treasury, allocation);
        increase_block_number_by(timeline.end());

        let _ =
            <Token as PalletToken<AccountId, Policy, IssuanceParams>>::claim_revenue_split_amount(
                token_id,
                participant_id,
            );
        assert_eq!(
            Token::account_info_by_token_and_account(token_id, participant_id).liquidity,
            staked,
        );
    })
}

#[test]
fn unreserved_fails_with_invalid_token_id() {
    let (token_id, pre_issuance) = (token!(1), balance!(900));
    let timeline = timeline!(block!(1), block!(10));
    let (participant_id, staked) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty()
        .with_issuance(pre_issuance)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, 0, staked)
        .build();

    build_test_externalities(config).execute_with(|| {
        let result = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::unreserve(
            token_id + 1,
            participant_id,
            staked,
        );

        assert_noop!(result, Error::<Test>::TokenDoesNotExist);
    })
}

#[test]
fn unreserved_fails_with_insufficient_reserved_balance() {
    let (token_id, pre_issuance) = (token!(1), balance!(900));
    let (participant_id, staked) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty()
        .with_issuance(pre_issuance)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, 0, staked - 1)
        .build();

    build_test_externalities(config).execute_with(|| {
        let result = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::unreserve(
            token_id,
            participant_id,
            staked,
        );

        assert_noop!(result, Error::<Test>::InsufficientReservedBalance);
    })
}

#[test]
fn unreserved_fails_with_invalid_account_id() {
    let (token_id, pre_issuance) = (token!(1), balance!(900));
    let (participant_id, staked) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty()
        .with_issuance(pre_issuance)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        let result = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::unreserve(
            token_id,
            participant_id,
            staked,
        );

        assert_noop!(result, Error::<Test>::AccountInformationDoesNotExist);
    })
}

#[test]
fn unreserved_ok() {
    let (token_id, pre_issuance) = (token!(1), balance!(900));
    let (participant_id, staked) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty()
        .with_issuance(pre_issuance)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(participant_id, 0, staked)
        .build();

    build_test_externalities(config).execute_with(|| {
        let result = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::unreserve(
            token_id,
            participant_id,
            staked,
        );

        assert_ok!(result);
    })
}

#[test]
fn unreserved_ok_with_event_deposit() {
    let (token_id, pre_issuance) = (token!(1), balance!(900));
    let (account_id, staked) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty()
        .with_issuance(pre_issuance)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(account_id, 0, staked)
        .build();

    build_test_externalities(config).execute_with(|| {
        let _ = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::unreserve(
            token_id, account_id, staked,
        );

        last_event_eq!(RawEvent::TokenAmountUnreservedFrom(
            token_id, account_id, staked
        ));
    })
}

#[test]
fn unreserved_ok_with_reserved_amount_zero() {
    let (token_id, pre_issuance) = (token!(1), balance!(900));
    let (account_id, staked) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty()
        .with_issuance(pre_issuance)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(account_id, 0, staked)
        .build();

    build_test_externalities(config).execute_with(|| {
        let _ = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::unreserve(
            token_id, account_id, staked,
        );

        assert_eq!(
            Token::account_info_by_token_and_account(token_id, account_id).stacked_balance(),
            balance!(0)
        );
    })
}

#[test]
fn unreserved_ok_with_free_balance_increased() {
    let (token_id, pre_issuance) = (token!(1), balance!(900));
    let (account_id, staked) = (account!(2), balance!(100));

    let token_data = TokenDataBuilder::new_empty()
        .with_issuance(pre_issuance)
        .build();
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .with_account(account_id, 0, staked)
        .build();

    build_test_externalities(config).execute_with(|| {
        let _ = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::unreserve(
            token_id, account_id, staked,
        );

        assert_eq!(
            Token::account_info_by_token_and_account(token_id, account_id).liquidity,
            staked
        );
    })
}
