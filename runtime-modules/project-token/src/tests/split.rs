#[cfg(test)]
use frame_support::{assert_noop, assert_ok};

use crate::tests::mock::*;
use crate::tests::test_utils::{increase_account_balance, TokenDataBuilder};
use crate::traits::PalletToken;
use crate::{account, balance, last_event_eq, token, Error, RawEvent};

// helper macros
#[macro_export]
macro_rules! block {
    ($b:expr) => {
        BlockNumber::from($b as u32)
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
    let (start, duration) = (block!(1), block!(100));
    let (src, allocation) = (account!(1), joys!(100));
    increase_account_balance(src, allocation);

    build_test_externalities(config).execute_with(|| {
        let result = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::issue_revenue_split(
            token_id, start, duration, src, allocation,
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
    let (start, duration) = (block!(1), block!(100));
    let (src, allocation) = (account!(1), joys!(100));
    increase_account_balance(src, allocation);

    build_test_externalities(config).execute_with(|| {
        increase_block_number_by(block!(2));

        let result = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::issue_revenue_split(
            token_id, start, duration, src, allocation,
        );

        assert_noop!(result, Error::<Test>::StartingBlockLowerThanCurrentBlock);
    })
}

#[test]
fn issue_split_fails_with_duration_too_short() {
    let token_id = token!(1);
    let token_data = TokenDataBuilder::new_empty().build();
    let (start, duration) = (block!(1), MinRevenueSplitDuration::get() - block!(1));
    let (src, allocation) = (account!(1), joys!(100));
    increase_account_balance(src, allocation);

    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        let result = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::issue_revenue_split(
            token_id, start, duration, src, allocation,
        );

        assert_noop!(result, Error::<Test>::RevenueSplitDurationTooShort);
    })
}

#[test]
fn issue_split_fails_with_source_having_insufficient_balance() {
    let token_id = token!(1);
    let token_data = TokenDataBuilder::new_empty().build();
    let (start, duration) = (block!(1), MinRevenueSplitDuration::get() - block!(1));
    let (src, allocation) = (account!(1), joys!(100));
    increase_account_balance(src, joys!(0));

    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        let result = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::issue_revenue_split(
            token_id, start, duration, src, allocation,
        );

        assert_noop!(
            result,
            Error::<Test>::InsufficientBalanceForSpecifiedAllocation
        );
    })
}

#[test]
fn issue_split_fails_with_non_existing_source_account() {
    let token_id = token!(1);
    let token_data = TokenDataBuilder::new_empty().build();
    let (start, duration) = (block!(1), block!(10));
    let (src, allocation) = (account!(1), joys!(100));

    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        let result = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::issue_revenue_split(
            token_id, start, duration, src, allocation,
        );

        assert_noop!(result, Error::<Test>::AllocationSourceAccountDoesNotExist);
    })
}

#[test]
fn issue_split_ok() {
    let token_id = token!(1);
    let token_data = TokenDataBuilder::new_empty().build();
    let (start, duration) = (block!(1), block!(10));
    let (src, allocation) = (account!(1), joys!(100));
    increase_account_balance(src, allocation);

    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        let result = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::issue_revenue_split(
            token_id, start, duration, src, allocation,
        );

        assert_ok!(result);
    })
}

#[test]
fn issue_split_ok_with_event_deposit() {
    let token_id = token!(1);
    let token_data = TokenDataBuilder::new_empty().build();
    let (start, duration) = (block!(1), block!(10));
    let (src, allocation) = (account!(1), joys!(100));
    increase_account_balance(src, allocation);

    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    build_test_externalities(config).execute_with(|| {
        let _ = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::issue_revenue_split(
            token_id, start, duration, src, allocation,
        );

        last_event_eq!(RawEvent::RevenueSplitIssued(
            token_id, start, duration, allocation
        ));
    })
}
