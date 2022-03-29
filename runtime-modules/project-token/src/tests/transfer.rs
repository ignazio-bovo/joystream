#![cfg(test)]

use frame_support::{assert_noop, assert_ok, StorageDoubleMap};
use sp_arithmetic::traits::{One, Zero};

use crate::tests::mock::*;
use crate::traits::{ControlledTransfer, MultiCurrencyBase};
use crate::Error;

// base transfer tests
#[test]
fn base_transfer_fails_with_non_existing_token() {
    let config = GenesisConfigBuilder::new()
        .add_token_and_account_info()
        .add_account_info()
        .build();
    let src = AccountId::from(DEFAULT_ACCOUNT_ID);
    let dst = AccountId::from(DEFAULT_ACCOUNT_ID + 1);
    let amount = Balance::one();

    build_test_externalities(config).execute_with(|| {
        let token_id = Token::next_token_id();

        assert_noop!(
            <Token as MultiCurrencyBase<AccountId, IssuanceParams>>::transfer(
                token_id, src, dst, amount
            ),
            Error::<Test>::TokenDoesNotExist,
        );
    })
}

#[test]
fn base_transfer_fails_with_non_existing_src() {
    let config = GenesisConfigBuilder::new()
        .add_token_and_account_info()
        .add_account_info()
        .build();

    let token_id = One::one();
    let dst = AccountId::from(DEFAULT_ACCOUNT_ID + 1);
    let src = dst.saturating_add(One::one());
    let amount = Balance::one();

    build_test_externalities(config).execute_with(|| {
        assert_noop!(
            <Token as MultiCurrencyBase<AccountId, IssuanceParams>>::transfer(
                token_id, src, dst, amount
            ),
            Error::<Test>::AccountInformationDoesNotExist,
        );
    })
}

#[test]
fn base_transfer_fails_with_non_existing_dst() {
    let config = GenesisConfigBuilder::new()
        .add_token_and_account_info()
        .build();

    let token_id = One::one();
    let src = AccountId::from(DEFAULT_ACCOUNT_ID);
    let dst = AccountId::from(DEFAULT_ACCOUNT_ID + 1);
    let amount = Balance::one();

    build_test_externalities(config).execute_with(|| {
        assert_noop!(
            <Token as MultiCurrencyBase<AccountId, IssuanceParams>>::transfer(
                token_id, src, dst, amount
            ),
            Error::<Test>::AccountInformationDoesNotExist,
        );
    })
}

#[test]
fn base_transfer_fails_with_src_having_insufficient_free_balance() {
    let config = GenesisConfigBuilder::new()
        .add_token_and_account_info()
        .add_account_info()
        .build();

    let token_id = One::one();
    let src = AccountId::from(DEFAULT_ACCOUNT_ID);
    let dst = AccountId::from(DEFAULT_ACCOUNT_ID + 1);
    let amount = Balance::from(DEFAULT_FREE_BALANCE + 1);

    build_test_externalities(config).execute_with(|| {
        assert_noop!(
            <Token as MultiCurrencyBase<AccountId, IssuanceParams>>::transfer(
                token_id, src, dst, amount
            ),
            Error::<Test>::InsufficientFreeBalanceForTransfer,
        );
    })
}

#[test]
fn base_transfer_ok_without_src_removal() {
    let config = GenesisConfigBuilder::new()
        .add_token_and_account_info()
        .add_account_info()
        .build();

    let token_id = One::one();
    let src = AccountId::from(DEFAULT_ACCOUNT_ID);
    let dst = AccountId::from(DEFAULT_ACCOUNT_ID + 1);
    let amount = Balance::one();

    build_test_externalities(config).execute_with(|| {
        let src_free_balance_pre =
            Token::account_info_by_token_and_account(token_id, src).free_balance;
        let dst_free_balance_pre =
            Token::account_info_by_token_and_account(token_id, dst).free_balance;
        let issuance_pre = Token::token_info_by_id(token_id).current_total_issuance;

        assert_ok!(
            <Token as MultiCurrencyBase<AccountId, IssuanceParams>>::transfer(
                token_id, src, dst, amount
            )
        );

        let src_free_balance_post =
            Token::account_info_by_token_and_account(token_id, src).free_balance;
        let dst_free_balance_post =
            Token::account_info_by_token_and_account(token_id, dst).free_balance;
        let issuance_post = Token::token_info_by_id(token_id).current_total_issuance;

        assert_eq!(issuance_pre, issuance_post);
        assert_eq!(
            dst_free_balance_pre.saturating_add(amount),
            dst_free_balance_post
        );
        assert_eq!(
            src_free_balance_pre.saturating_sub(amount),
            src_free_balance_post
        );
    })
}

#[test]
fn base_transfer_ok_with_src_removal() {
    let config = GenesisConfigBuilder::new()
        .add_token_and_account_info()
        .add_account_info()
        .build();

    let token_id = One::one();
    let src = AccountId::from(DEFAULT_ACCOUNT_ID);
    let dst = AccountId::from(DEFAULT_ACCOUNT_ID + 1);
    let amount = Balance::from(DEFAULT_FREE_BALANCE - DEFAULT_EXISTENTIAL_DEPOSIT + 1);

    build_test_externalities(config).execute_with(|| {
        let src_account_config = Token::account_info_by_token_and_account(token_id, src);
        let dust = src_account_config.total_balance().saturating_sub(amount);
        let dst_free_balance_pre =
            Token::account_info_by_token_and_account(token_id, dst).free_balance;
        let issuance_pre = Token::token_info_by_id(token_id).current_total_issuance;

        assert_ok!(
            <Token as MultiCurrencyBase<AccountId, IssuanceParams>>::transfer(
                token_id, src, dst, amount
            )
        );

        let dst_free_balance_post =
            Token::account_info_by_token_and_account(token_id, dst).free_balance;
        let issuance_post = Token::token_info_by_id(token_id).current_total_issuance;

        assert_eq!(issuance_pre.saturating_sub(dust), issuance_post);
        assert_eq!(
            dst_free_balance_pre.saturating_add(amount),
            dst_free_balance_post
        );
        assert!(!<crate::AccountInfoByTokenAndAccount<Test>>::contains_key(
            token_id, src
        ));
    })
}
// multi output
#[test]
fn multiout_transfer_fails_with_non_existing_token() {
    let config = GenesisConfigBuilder::new()
        .add_token_and_account_info()
        .add_account_info()
        .add_account_info()
        .build();
    let src = AccountId::from(DEFAULT_ACCOUNT_ID);
    let outputs = vec![
        (
            Simple::new(AccountId::from(DEFAULT_ACCOUNT_ID + 1)),
            Balance::one(),
        ),
        (
            Simple::new(AccountId::from(DEFAULT_ACCOUNT_ID + 2)),
            Balance::one(),
        ),
    ];

    build_test_externalities(config).execute_with(|| {
        let token_id = Token::next_token_id();

        assert_noop!(
            <Token as ControlledTransfer<AccountId, Policy, IssuanceParams>>::controlled_multi_output_transfer(
                token_id, src, &outputs
            ),
            Error::<Test>::TokenDoesNotExist,
        );
    })
}

#[test]
fn multiout_transfer_fails_with_non_existing_src() {
    let config = GenesisConfigBuilder::new()
        .add_token_and_account_info()
        .add_account_info()
        .add_account_info()
        .build();
    let token_id = TokenId::one();
    let src = AccountId::from(DEFAULT_ACCOUNT_ID + 3);
    let outputs = vec![
        (
            Simple::new(AccountId::from(DEFAULT_ACCOUNT_ID + 1)),
            Balance::one(),
        ),
        (
            Simple::new(AccountId::from(DEFAULT_ACCOUNT_ID + 2)),
            Balance::one(),
        ),
    ];

    build_test_externalities(config).execute_with(|| {
        assert_noop!(
            <Token as ControlledTransfer<AccountId, Policy, IssuanceParams>>::controlled_multi_output_transfer(
                token_id, src, &outputs
            ),
            Error::<Test>::AccountInformationDoesNotExist,
        );
    })
}

#[test]
fn multiout_transfer_fails_with_non_existing_dst() {
    let config = GenesisConfigBuilder::new()
        .add_token_and_account_info()
        .add_account_info()
        .build();
    let token_id = TokenId::one();
    let src = AccountId::from(DEFAULT_ACCOUNT_ID);
    let outputs = vec![
        (
            Simple::new(AccountId::from(DEFAULT_ACCOUNT_ID + 1)),
            Balance::one(),
        ),
        (
            Simple::new(AccountId::from(DEFAULT_ACCOUNT_ID + 2)),
            Balance::one(),
        ),
    ];

    build_test_externalities(config).execute_with(|| {
        assert_noop!(
            <Token as ControlledTransfer<AccountId, Policy, IssuanceParams>>::controlled_multi_output_transfer(
                token_id, src, &outputs
            ),
            Error::<Test>::AccountInformationDoesNotExist,
        );
    })
}

#[test]
fn multiout_transfer_fails_with_insufficient_balance() {
    let config = GenesisConfigBuilder::new()
        .add_token_and_account_info()
        .add_account_info()
        .add_account_info()
        .build();
    let token_id = TokenId::one();
    let src = AccountId::from(DEFAULT_ACCOUNT_ID);
    let outputs = vec![
        (
            Simple::new(AccountId::from(DEFAULT_ACCOUNT_ID + 1)),
            Balance::from(DEFAULT_FREE_BALANCE),
        ),
        (
            Simple::new(AccountId::from(DEFAULT_ACCOUNT_ID + 2)),
            Balance::one(),
        ),
    ];

    build_test_externalities(config).execute_with(|| {
        assert_noop!(
            <Token as ControlledTransfer<AccountId, Policy, IssuanceParams>>::controlled_multi_output_transfer(
                token_id, src, &outputs
            ),
            Error::<Test>::InsufficientFreeBalanceForTransfer,
        );
    })
}

#[test]
fn multiout_transfer_fails_with_same_source_and_destination() {
    let config = GenesisConfigBuilder::new()
        .add_token_and_account_info()
        .add_account_info()
        .add_account_info()
        .build();
    let token_id = TokenId::one();
    let src = AccountId::from(DEFAULT_ACCOUNT_ID);
    let outputs = vec![
        (
            Simple::new(AccountId::from(DEFAULT_ACCOUNT_ID)),
            Balance::from(DEFAULT_FREE_BALANCE),
        ),
        (
            Simple::new(AccountId::from(DEFAULT_ACCOUNT_ID + 2)),
            Balance::one(),
        ),
    ];

    build_test_externalities(config).execute_with(|| {
        assert_noop!(
            <Token as ControlledTransfer<AccountId, Policy, IssuanceParams>>::controlled_multi_output_transfer(
                token_id, src, &outputs
            ),
            Error::<Test>::SameSourceAndDestinationLocations,
        );
    })
}

#[test]
fn multiout_transfer_ok_without_src_removal() {
    let config = GenesisConfigBuilder::new()
        .add_token_and_account_info()
        .add_account_info()
        .add_account_info()
        .build();
    let token_id = TokenId::one();
    let src = AccountId::from(DEFAULT_ACCOUNT_ID);
    let outputs = vec![
        (
            Simple::new(AccountId::from(DEFAULT_ACCOUNT_ID + 1)),
            Balance::one(),
        ),
        (
            Simple::new(AccountId::from(DEFAULT_ACCOUNT_ID + 2)),
            Balance::one(),
        ),
    ];

    let total_amount = outputs.iter().fold(Balance::zero(), |acc, (_, amount)| {
        acc.saturating_add(*amount)
    });

    build_test_externalities(config).execute_with(|| {
        let src_pre = Token::account_info_by_token_and_account(token_id, src).free_balance;
        let issuance_pre = Token::token_info_by_id(token_id).current_total_issuance;
        let dst_pre = outputs
            .iter()
            .map(|(dst, _)| {
                Token::account_info_by_token_and_account(token_id, dst.account).free_balance
            })
            .collect::<Vec<_>>();

        assert_ok!(<Token as ControlledTransfer<
            AccountId,
            Policy,
            IssuanceParams,
        >>::controlled_multi_output_transfer(
            token_id, src, &outputs
        ));

        let issuance_post = Token::token_info_by_id(token_id).current_total_issuance;
        let src_post = Token::account_info_by_token_and_account(token_id, src).free_balance;
        let dst_post = outputs.iter().map(|(dst, _)| {
            Token::account_info_by_token_and_account(token_id, dst.account).free_balance
        });

        assert_eq!(issuance_pre, issuance_post);
        assert_eq!(src_pre, src_post.saturating_add(total_amount));
        assert!(dst_pre
            .iter()
            .zip(dst_post)
            .zip(outputs.iter().map(|(_, amount)| *amount))
            .all(|((pre, post), amount)| { pre.saturating_add(amount) == post }));
    })
}

#[test]
fn multiout_transfer_ok_with_src_removal() {
    let config = GenesisConfigBuilder::new()
        .add_token_and_account_info()
        .add_account_info()
        .add_account_info()
        .build();
    let token_id = TokenId::one();
    let src = AccountId::from(DEFAULT_ACCOUNT_ID);
    let outputs = vec![
        (
            Simple::new(AccountId::from(DEFAULT_ACCOUNT_ID + 1)),
            Balance::one(),
        ),
        (
            Simple::new(AccountId::from(DEFAULT_ACCOUNT_ID + 2)),
            Balance::from(DEFAULT_FREE_BALANCE - DEFAULT_EXISTENTIAL_DEPOSIT),
        ),
    ];

    let total_amount = outputs.iter().fold(Balance::zero(), |acc, (_, amount)| {
        acc.saturating_add(*amount)
    });

    build_test_externalities(config).execute_with(|| {
        let src_pre = Token::account_info_by_token_and_account(token_id, src).free_balance;
        let issuance_pre = Token::token_info_by_id(token_id).current_total_issuance;
        let dust = src_pre.saturating_sub(total_amount);
        let dst_pre = outputs
            .iter()
            .map(|(dst, _)| {
                Token::account_info_by_token_and_account(token_id, dst.account).free_balance
            })
            .collect::<Vec<_>>();

        assert_ok!(<Token as ControlledTransfer<
            AccountId,
            Policy,
            IssuanceParams,
        >>::controlled_multi_output_transfer(
            token_id, src, &outputs
        ));

        let issuance_post = Token::token_info_by_id(token_id).current_total_issuance;
        let dst_post = outputs.iter().map(|(dst, _)| {
            Token::account_info_by_token_and_account(token_id, dst.account).free_balance
        });

        assert_eq!(issuance_pre, issuance_post.saturating_add(dust));
        assert!(!<crate::AccountInfoByTokenAndAccount<Test>>::contains_key(
            token_id, src
        ));
        assert!(dst_pre
            .iter()
            .zip(dst_post)
            .zip(outputs.iter().map(|(_, amount)| *amount))
            .all(|((pre, post), amount)| { pre.saturating_add(amount) == post }));
    })
}
