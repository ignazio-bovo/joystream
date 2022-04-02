#[cfg(test)]
use frame_support::{assert_noop, assert_ok, traits::Currency};
use sp_runtime::{traits::AccountIdConversion, Percent};

use crate::tests::mock::*;
use crate::tests::test_utils::{increase_account_balance, TokenDataBuilder};
use crate::traits::PalletToken;
use crate::{account, last_event_eq, token, Error, RawEvent};

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

        assert_noop!(result, Error::<Test>::RevenueSplitNotActiveForToken);
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
