#[cfg(test)]
use frame_support::{assert_noop, assert_ok};
use sp_arithmetic::traits::{Saturating, Zero};

use crate::tests::mock::*;
use crate::tests::test_utils::TokenDataBuilder;
use crate::traits::PalletToken;
use crate::{account, balance, last_event_eq, token, Error, RawEvent};

// helper macros
#[macro_export]
macro_rules! block {
    ($b:expr) => {
        BlockNumber::from($b as u32)
    };
}

#[test]
fn issue_split_fails_with_invalid_token_id() {
    let token_id = token!(1);
    let config = GenesisConfigBuilder::new_empty().build();
    let (start, duration) = (block!(1), block!(100));
    let (src, allocation) = (account!(1), balance!(100));

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
    let (src, allocation) = (account!(1), balance!(100));

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
    let config = GenesisConfigBuilder::new_empty()
        .with_token(token_id, token_data)
        .build();

    let (start, duration) = (block!(1), MinRevenueSplitDuration::get() - block!(1));
    let (src, allocation) = (account!(1), balance!(100));

    build_test_externalities(config).execute_with(|| {
        let result = <Token as PalletToken<AccountId, Policy, IssuanceParams>>::issue_revenue_split(
            token_id, start, duration, src, allocation,
        );

        assert_noop!(result, Error::<Test>::RevenueSplitDurationTooShort);
    })
}
