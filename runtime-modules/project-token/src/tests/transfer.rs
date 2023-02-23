#![cfg(test)]
use crate::errors::Error;
use crate::tests::test_utils::new_issuer_transfers;
use crate::tests::{fixtures::*, mock::*};
use crate::types::{Transfers, Validated, VestingSource};
use crate::{last_event_eq, RawEvent, RepayableBloatBond};
use frame_support::traits::Currency;
use frame_support::{assert_err, assert_ok};
use sp_runtime::traits::Zero;
use sp_runtime::{DispatchError, Permill};
use test_case::test_case;

// helpers
macro_rules! validated_outputs {
    [$(($a:expr, $b: expr, $c: expr, $d: expr)),*] => {
        Transfers::<_,_>::new_validated(vec![$(($a, $b, $c, $d),)*])
    };
}

// permissionless transfer tests
#[test]
fn transfer_fails_with_non_existing_token() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, 2 * DEFAULT_BLOAT_BOND + ed());

        let result = TransferFixture::new()
            .with_token_id(DEFAULT_TOKEN_ID + 1)
            .execute_call();

        assert_err!(result, Error::<Test>::TokenDoesNotExist);
    })
}

#[test]
fn transfer_fails_with_non_existing_source() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, 2 * DEFAULT_BLOAT_BOND + ed());

        let result = TransferFixture::new()
            .with_sender(SECOND_USER_ACCOUNT_ID)
            .with_src_member_id(SECOND_USER_MEMBER_ID)
            .execute_call();

        assert_err!(result, Error::<Test>::AccountInformationDoesNotExist);
    })
}

// TODO(Integration Test): move this to integration tests as it should fail because we are not able to peek inside Membership pallet
#[test]
fn transfer_fails_with_non_existing_dst_member() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, 2 * DEFAULT_BLOAT_BOND + ed());

        // !! second member id should not exist on the Membership pallet
        let result = TransferFixture::new()
            .with_output(9999, DEFAULT_USER_BALANCE)
            .execute_call();

        assert_err!(result, Error::<Test>::TransferDestinationMemberDoesNotExist);
    })
}

#[test]
fn transfer_fails_with_invalid_src_member_controller() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, 2 * DEFAULT_BLOAT_BOND + ed());

        let result = TransferFixture::new()
            .with_sender(SECOND_USER_ACCOUNT_ID)
            .execute_call();

        assert_err!(
            result,
            DispatchError::Other("origin signer not a member controller account")
        );
    })
}

#[test]
fn permissionless_transfer_fails_with_src_having_insufficient_funds_for_bloat_bond() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();

        let result = TransferFixture::new().execute_call();

        assert_err!(result, Error::<Test>::InsufficientJoyBalance);
    })
}

#[test]
fn permissionless_transfer_ok_with_non_existing_destination() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed());

        let result = TransferFixture::new().execute_call();

        assert_ok!(result);
    })
}

#[test]
fn permissionless_transfer_ok_with_new_destination_created() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed());

        TransferFixture::new().run();

        assert_ok!(
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &SECOND_USER_MEMBER_ID),
            AccountData::new_with_amount_and_bond(
                DEFAULT_USER_BALANCE,
                RepayableBloatBond::new(DEFAULT_BLOAT_BOND, None)
            )
        );
    })
}

#[test]
fn transfer_ok_with_new_destinations_created_and_account_number_incremented() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed());
        let accounts_number_pre = Token::token_info_by_id(DEFAULT_TOKEN_ID).accounts_number;

        TransferFixture::new().run();

        assert_eq!(
            accounts_number_pre + 1,
            Token::token_info_by_id(DEFAULT_TOKEN_ID).accounts_number,
        );
    })
}

#[test]
fn permissionless_transfer_ok_for_new_destination_with_bloat_bond_slashed_from_src() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed());

        TransferFixture::new().run();

        assert_eq!(Balances::usable_balance(&FIRST_USER_ACCOUNT_ID), ed());
    })
}

#[test]
fn permissionless_transfer_ok_for_new_destination_with_bloat_bond_transferred_to_treasury() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed());
        let treasury_balance_pre = Balances::usable_balance(&Token::module_treasury_account());

        TransferFixture::new().run();

        assert_eq!(
            Balances::usable_balance(&Token::module_treasury_account()),
            treasury_balance_pre + DEFAULT_BLOAT_BOND,
        );
    })
}

#[test]
fn permissionless_transfer_fails_with_source_not_having_sufficient_free_balance() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::new()
            .with_first_user_balance(Some(Zero::zero()))
            .build();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed());

        let result = TransferFixture::new().execute_call();

        assert_err!(result, Error::<Test>::InsufficientTransferrableBalance);
    })
}

#[test]
fn permissionless_transfer_ok() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed());

        let result = TransferFixture::new().execute_call();

        assert_ok!(result);
    })
}

#[test]
fn permissionless_transfer_ok_with_event_deposit() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_users();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed());

        TransferFixture::new().run();

        last_event_eq!(RawEvent::TokenAmountTransferred(
            DEFAULT_TOKEN_ID,
            FIRST_USER_MEMBER_ID,
            validated_outputs![(
                Validated::<_>::Existing(SECOND_USER_MEMBER_ID),
                DEFAULT_USER_BALANCE,
                None,
                None
            )],
            b"metadata".to_vec()
        ));
    })
}

#[test]
fn permissionless_transfer_ok_with_destination_receiving_funds() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_users();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed());
        let balance_pre =
            Token::account_info_by_token_and_member(DEFAULT_TOKEN_ID, SECOND_USER_MEMBER_ID)
                .transferrable::<Test>(System::block_number());

        TransferFixture::new().run();

        assert_eq!(
            Token::account_info_by_token_and_member(DEFAULT_TOKEN_ID, SECOND_USER_MEMBER_ID)
                .transferrable::<Test>(System::block_number()),
            balance_pre + DEFAULT_USER_BALANCE
        );
    })
}

#[test]
fn transfer_ok_without_change_in_token_supply() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_users();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed());
        let supply_pre = Token::token_info_by_id(DEFAULT_TOKEN_ID).total_supply;

        TransferFixture::new()
            .with_output(SECOND_USER_MEMBER_ID, DEFAULT_USER_BALANCE)
            .run();

        assert_eq!(
            Token::token_info_by_id(DEFAULT_TOKEN_ID).total_supply,
            supply_pre
        );
    })
}

// multi output

#[test]
fn multiout_transfer_ok_with_non_existing_destination() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();

        let result = TransferFixture::new()
            .with_sender(DEFAULT_ISSUER_ACCOUNT_ID)
            .with_src_member_id(DEFAULT_ISSUER_MEMBER_ID)
            .with_multioutput_and_same_amount(
                FIRST_USER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE,
            )
            .execute_call();

        assert_ok!(result);
    })
}

#[test]
fn multiout_transfer_fails_with_src_having_insufficient_funds_for_bloat_bond() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();
        Balances::make_free_balance_be(&DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed());

        let result = TransferFixture::new()
            .with_sender(DEFAULT_ISSUER_ACCOUNT_ID)
            .with_src_member_id(DEFAULT_ISSUER_MEMBER_ID)
            .with_multioutput_and_same_amount(
                FIRST_USER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE,
            )
            .execute_call();

        assert_err!(result, Error::<Test>::InsufficientJoyBalance);
    })
}

#[test]
fn multiout_transfer_ok() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();

        let result = TransferFixture::new()
            .with_sender(DEFAULT_ISSUER_ACCOUNT_ID)
            .with_src_member_id(DEFAULT_ISSUER_MEMBER_ID)
            .with_multioutput_and_same_amount(
                FIRST_USER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE,
            )
            .execute_call();

        assert_ok!(result);
    })
}

#[test]
fn multiout_transfer_ok_with_event_deposit() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();

        TransferFixture::new()
            .with_sender(DEFAULT_ISSUER_ACCOUNT_ID)
            .with_src_member_id(DEFAULT_ISSUER_MEMBER_ID)
            .with_multioutput_and_same_amount(
                FIRST_USER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE,
            )
            .run();

        last_event_eq!(RawEvent::TokenAmountTransferred(
            DEFAULT_TOKEN_ID,
            DEFAULT_ISSUER_MEMBER_ID,
            validated_outputs![
                (
                    Validated::<_>::Existing(FIRST_USER_MEMBER_ID),
                    DEFAULT_USER_BALANCE,
                    None,
                    None
                ),
                (
                    Validated::<_>::NonExisting(SECOND_USER_MEMBER_ID),
                    DEFAULT_USER_BALANCE,
                    None,
                    None
                )
            ],
            b"metadata".to_vec()
        ));
    })
}

#[test]
fn transfer_ok_and_source_left_with_zero_token_balance() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed());

        TransferFixture::new()
            .with_multioutput_and_same_amount(
                DEFAULT_ISSUER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE / 2,
            )
            .run();

        assert!(
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID)
                .unwrap()
                .transferrable::<Test>(System::block_number())
                .is_zero()
        );
    })
}

#[test]
fn multiout_transfer_fails_with_source_having_insufficient_balance() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();

        let result = TransferFixture::new()
            .with_multioutput_and_same_amount(
                DEFAULT_ISSUER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE,
            )
            .execute_call();

        assert_err!(result, Error::<Test>::InsufficientTransferrableBalance);
    })
}

#[test]
fn multiout_transfer_ok_with_same_source_and_destination() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_users();

        let result = TransferFixture::new()
            .with_multioutput_and_same_amount(
                FIRST_USER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE / 2,
            )
            .execute_call();

        assert_ok!(result);
    })
}

#[test]
fn multiout_transfer_ok_with_new_destinations_created() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();

        TransferFixture::new()
            .with_sender(DEFAULT_ISSUER_ACCOUNT_ID)
            .with_src_member_id(DEFAULT_ISSUER_MEMBER_ID)
            .with_multioutput_and_same_amount(
                FIRST_USER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE,
            )
            .run();

        assert_ok!(
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID),
            AccountData::new_with_amount_and_bond(
                DEFAULT_USER_BALANCE,
                RepayableBloatBond::new(DEFAULT_BLOAT_BOND, None)
            )
        );
        assert_ok!(
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &SECOND_USER_MEMBER_ID),
            AccountData::new_with_amount_and_bond(
                DEFAULT_USER_BALANCE,
                RepayableBloatBond::new(DEFAULT_BLOAT_BOND, None)
            )
        );
    })
}

#[test]
fn multiout_transfer_ok_with_bloat_bond_for_new_destinations_slashed_from_src() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();
        let balance_pre = Balances::usable_balance(&DEFAULT_ISSUER_ACCOUNT_ID);

        TransferFixture::new()
            .with_sender(DEFAULT_ISSUER_ACCOUNT_ID)
            .with_src_member_id(DEFAULT_ISSUER_MEMBER_ID)
            .with_multioutput_and_same_amount(
                FIRST_USER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE,
            )
            .run();

        assert_eq!(
            Balances::usable_balance(&DEFAULT_ISSUER_ACCOUNT_ID),
            balance_pre - 2 * DEFAULT_BLOAT_BOND
        );
    })
}

#[test]
fn multiout_transfer_ok_with_bloat_bond_transferred_to_treasury() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();
        let treasury_balance_pre = Balances::usable_balance(&Token::module_treasury_account());

        TransferFixture::new()
            .with_sender(DEFAULT_ISSUER_ACCOUNT_ID)
            .with_src_member_id(DEFAULT_ISSUER_MEMBER_ID)
            .with_multioutput_and_same_amount(
                FIRST_USER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE,
            )
            .run();

        assert_eq!(
            Balances::usable_balance(&Token::module_treasury_account()),
            treasury_balance_pre + 2 * DEFAULT_BLOAT_BOND
        );
    })
}

#[test]
fn transfer_ok_with_same_source_and_destination() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        increase_account_balance(&FIRST_USER_MEMBER_ID, DEFAULT_BLOAT_BOND + ed());
        let token_amount_pre =
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID)
                .unwrap()
                .transferrable::<Test>(System::block_number());

        TransferFixture::new()
            .with_output(FIRST_USER_MEMBER_ID, DEFAULT_USER_BALANCE)
            .run();

        let token_amount_post =
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID)
                .unwrap()
                .transferrable::<Test>(System::block_number());
        assert_eq!(token_amount_pre, token_amount_post);
    })
}

#[test]
fn permissioned_transfer_ok() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_users_permissioned();

        let result = TransferFixture::new()
            .with_multioutput_and_same_amount(
                DEFAULT_ISSUER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE / 2,
            )
            .execute_call();

        assert_ok!(result);
    })
}

#[test]
fn permissioned_transfer_ok_with_event_deposit() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_users_permissioned();

        TransferFixture::new()
            .with_output(SECOND_USER_MEMBER_ID, DEFAULT_USER_BALANCE)
            .run();

        last_event_eq!(RawEvent::TokenAmountTransferred(
            DEFAULT_TOKEN_ID,
            FIRST_USER_MEMBER_ID,
            validated_outputs![(
                Validated::<_>::Existing(SECOND_USER_MEMBER_ID),
                DEFAULT_USER_BALANCE,
                None,
                None
            )],
            b"metadata".to_vec()
        ));
    })
}

#[test]
fn permissioned_transfer_fails_with_invalid_destination() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user_permissioned();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND);

        let result = TransferFixture::new()
            .with_output(SECOND_USER_MEMBER_ID, DEFAULT_USER_BALANCE)
            .execute_call();

        assert_err!(result, Error::<Test>::AccountInformationDoesNotExist);
    })
}

#[test]
fn permissioned_multi_out_transfer_fails_with_invalid_destination() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user_permissioned();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND);

        let result = TransferFixture::new()
            .with_multioutput_and_same_amount(
                DEFAULT_ISSUER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE / 2,
            )
            .execute_call();

        assert_err!(result, Error::<Test>::AccountInformationDoesNotExist);
    })
}

#[test]
fn permissioned_multi_out_transfer_fails_with_insufficient_token_tranferrable_balance() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user_permissioned();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND);

        let result = TransferFixture::new()
            .with_multioutput_and_same_amount(
                DEFAULT_ISSUER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE,
            )
            .execute_call();

        assert_err!(result, Error::<Test>::InsufficientTransferrableBalance);
    })
}

#[test]
fn permissioned_multi_out_transfer_ok() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_users_permissioned();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND);

        let result = TransferFixture::new()
            .with_multioutput_and_same_amount(
                DEFAULT_ISSUER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE / 2,
            )
            .execute_call();

        assert_ok!(result);
    })
}

#[test]
fn permissioned_multi_out_transfer_ok_with_event_deposit() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_users_permissioned();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND);

        TransferFixture::new()
            .with_multioutput_and_same_amount(
                DEFAULT_ISSUER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE / 2,
            )
            .run();

        last_event_eq!(RawEvent::TokenAmountTransferred(
            DEFAULT_TOKEN_ID,
            FIRST_USER_MEMBER_ID,
            validated_outputs![
                (
                    Validated::<_>::Existing(DEFAULT_ISSUER_MEMBER_ID),
                    DEFAULT_USER_BALANCE / 2,
                    None,
                    None
                ),
                (
                    Validated::<_>::Existing(SECOND_USER_MEMBER_ID),
                    DEFAULT_USER_BALANCE / 2,
                    None,
                    None
                )
            ],
            b"metadata".to_vec()
        ));
    })
}

#[test_case(ed(), (None,None,None); "just_ed")]
#[test_case(ed() + 1 , (Some(DEFAULT_ISSUER_ACCOUNT_ID),None,None); "more_than_ed")]
#[test_case(ed() + DEFAULT_BLOAT_BOND , (Some(DEFAULT_ISSUER_ACCOUNT_ID),None,None); "ed_and_bloat_bond")]
#[test_case(ed() + DEFAULT_BLOAT_BOND + 1, (Some(DEFAULT_ISSUER_ACCOUNT_ID), Some(DEFAULT_ISSUER_ACCOUNT_ID),None); "more_than_ed_and_bloat_bond")]
#[test_case(ed() + DEFAULT_BLOAT_BOND * 2, (Some(DEFAULT_ISSUER_ACCOUNT_ID), Some(DEFAULT_ISSUER_ACCOUNT_ID),None); "ed_and_twice_bloat_bond")]
#[test_case(ed() + DEFAULT_BLOAT_BOND * 2 + 1, (Some(DEFAULT_ISSUER_ACCOUNT_ID), Some(DEFAULT_ISSUER_ACCOUNT_ID),Some(DEFAULT_ISSUER_ACCOUNT_ID)); "more_than_ed_and_twice_bloat_bond")]
#[test_case(ed() + DEFAULT_BLOAT_BOND * 3, (Some(DEFAULT_ISSUER_ACCOUNT_ID), Some(DEFAULT_ISSUER_ACCOUNT_ID),Some(DEFAULT_ISSUER_ACCOUNT_ID)); "ed_and_trice_bloat_bond")]
fn transfer_ok_with_invitation_locked_funds(
    locked_balance: JoyBalance,
    expected_bloat_bond_restricted_to: (Option<AccountId>, Option<AccountId>, Option<AccountId>),
) {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();
        set_invitation_lock(&DEFAULT_ISSUER_ACCOUNT_ID, locked_balance);

        TransferFixture::new()
            .with_sender(DEFAULT_ISSUER_ACCOUNT_ID)
            .with_src_member_id(DEFAULT_ISSUER_MEMBER_ID)
            .with_multioutput_and_same_amount(
                FIRST_USER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE / 2,
            )
            .run();

        assert_eq!(
            Balances::usable_balance(Token::module_treasury_account()),
            3 * DEFAULT_BLOAT_BOND + ed()
        );
        assert_eq!(
            System::account(DEFAULT_ISSUER_ACCOUNT_ID).data,
            balances::AccountData {
                free: ed(),
                reserved: 0,
                misc_frozen: locked_balance,
                fee_frozen: 0
            }
        );
        assert_eq!(
            Token::account_info_by_token_and_member(DEFAULT_TOKEN_ID, FIRST_USER_MEMBER_ID)
                .bloat_bond,
            RepayableBloatBond::new(DEFAULT_BLOAT_BOND, expected_bloat_bond_restricted_to.0)
        );
        assert_eq!(
            Token::account_info_by_token_and_member(DEFAULT_TOKEN_ID, SECOND_USER_MEMBER_ID)
                .bloat_bond,
            RepayableBloatBond::new(DEFAULT_BLOAT_BOND, expected_bloat_bond_restricted_to.1)
        );
    });
}

#[test]
fn transfer_fails_with_insufficient_locked_funds() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND);
        set_invitation_lock(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed() - 1);
        let transfer_fixture = TransferFixture::new().with_multioutput_and_same_amount(
            DEFAULT_ISSUER_MEMBER_ID,
            SECOND_USER_MEMBER_ID,
            DEFAULT_USER_BALANCE / 2,
        );

        let result_after_first_lock = transfer_fixture.execute_call();

        assert_err!(
            result_after_first_lock,
            Error::<Test>::InsufficientJoyBalance
        );

        // Increase balance by 1, but lock ED and those funds with another, not-allowed lock
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, 1);
        set_staking_candidate_lock(&FIRST_USER_ACCOUNT_ID, ed() + 1);

        let result_after_second_lock = transfer_fixture.execute_call();

        assert_err!(
            result_after_second_lock,
            Error::<Test>::InsufficientJoyBalance
        );
    });
}

#[test]
fn transfer_fails_with_incompatible_locked_funds() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND);
        set_staking_candidate_lock(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed() - 1);

        let result = TransferFixture::new()
            .with_multioutput_and_same_amount(
                DEFAULT_ISSUER_MEMBER_ID,
                SECOND_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE / 2,
            )
            .execute_call();

        assert_err!(result, Error::<Test>::InsufficientJoyBalance);
    });
}

#[test]
fn change_to_permissionless_fails_with_invalid_token_id() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();

        let result = ChangeToPermissionlessFixture::new()
            .with_token_id(DEFAULT_TOKEN_ID + 1)
            .execute_call();

        assert_err!(result, Error::<Test>::TokenDoesNotExist);
    })
}

#[test]
fn change_to_permissionless_ok_from_permissioned_state() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only_permissioned();

        let result = ChangeToPermissionlessFixture::new().execute_call();

        assert_ok!(result);
    })
}

// Issuer transfers

#[test]
fn issuer_transfer_fails_with_non_existing_token() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();

        let result = IssuerTransferFixture::new()
            .with_token_id(DEFAULT_TOKEN_ID + 1)
            .execute_call();

        assert_err!(result, Error::<Test>::TokenDoesNotExist);
    })
}

#[test]
fn issuer_transfer_fails_with_non_existing_source() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();

        let result = IssuerTransferFixture::new()
            .with_src_member_id(99999u64)
            .execute_call();

        assert_err!(result, Error::<Test>::AccountInformationDoesNotExist);
    })
}

#[test]
fn issuer_transfer_fails_with_destination_member_id_not_existing_in_the_membership_pallet() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();

        let result = IssuerTransferFixture::new()
            .with_outputs(new_issuer_transfers(vec![(
                9999u64,
                DEFAULT_USER_BALANCE,
                None,
            )]))
            .execute_call();

        assert_err!(result, Error::<Test>::TransferDestinationMemberDoesNotExist);
    })
}

#[test]
fn issuer_transfer_fails_with_src_not_having_enough_joys_for_bloat_bond() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();
        Balances::make_free_balance_be(&DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed() - 1);

        let result = IssuerTransferFixture::new().execute_call();

        assert_err!(result, Error::<Test>::InsufficientJoyBalance);
    })
}

#[test_case(|| { TokenContext::with_issuer_only_permissioned() }; "given only issuer in permissioned mode")]
#[test_case(|| { TokenContext::with_issuer_and_first_user_permissioned() }; "given issuer and first member permissioned")]
fn issuer_permissioned_token_transfer_fails_with_source_not_having_enough_tokens(
    either_issuer_only_or_issuer_with_first_user_permissioned: fn(),
) {
    build_default_test_externalities().execute_with(|| {
        either_issuer_only_or_issuer_with_first_user_permissioned();
        let issuer_balance =
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &DEFAULT_ISSUER_MEMBER_ID)
                .unwrap()
                .amount;

        let result = IssuerTransferFixture::new()
            .with_outputs(new_issuer_transfers(vec![(
                FIRST_USER_MEMBER_ID,
                issuer_balance + 1,
                None,
            )]))
            .execute_call();

        assert_err!(result, Error::<Test>::InsufficientTransferrableBalance);
    })
}

#[test]
fn issuer_permissioned_token_transfer_fails_with_dst_vesting_schedules_limit_exceeded() {
    let max_vesting_schedules_num = MaxVestingSchedulesPerAccountPerToken::get();
    let vesting = VestingScheduleParams {
        blocks_before_cliff: 200,
        cliff_amount_percentage: Permill::from_percent(20),
        linear_vesting_duration: 200,
    };
    assert!(
        DEFAULT_USER_BALANCE * (max_vesting_schedules_num as u128) <= DEFAULT_INITIAL_ISSUANCE,
        "issuer balance too low"
    );
    build_default_test_externalities().execute_with(|| {
        // Arrange
        TokenContext::with_issuer_and_first_user();
        // Create max vesting schedules
        for _ in 0u64..max_vesting_schedules_num.into() {
            IssuerTransferFixture::new()
                .with_output(
                    FIRST_USER_MEMBER_ID,
                    DEFAULT_USER_BALANCE,
                    Some(vesting.clone()),
                )
                .run();
        }

        // Act: try to add one extra vesting schedule
        let result = IssuerTransferFixture::new()
            .with_output(
                FIRST_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE,
                Some(vesting.clone()),
            )
            .execute_call();

        assert_err!(
            result,
            Error::<Test>::MaxVestingSchedulesPerAccountPerTokenReached
        );
    })
}

#[test_case(Some(
VestingScheduleParams {
        blocks_before_cliff: 100,
        cliff_amount_percentage: Permill::from_percent(10),
        linear_vesting_duration: 100,
    }); "with_vesting_schedule")]
#[test_case(None; "without_vesting_schedule")]
fn issuer_transfer_ok_with_event_deposit_given_existing_user(
    vesting: Option<VestingScheduleParams>,
) {
    assert!(DEFAULT_INITIAL_ISSUANCE >= 2 * DEFAULT_USER_BALANCE);
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();

        IssuerTransferFixture::new()
            .with_outputs(new_issuer_transfers(vec![(
                FIRST_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE,
                vesting.clone(),
            )]))
            .run();

        last_event_eq!(RawEvent::TokenAmountTransferredByIssuer(
            DEFAULT_TOKEN_ID,
            DEFAULT_ISSUER_MEMBER_ID,
            validated_outputs![(
                Validated::<_>::Existing(FIRST_USER_MEMBER_ID),
                DEFAULT_USER_BALANCE,
                vesting,
                None
            )],
            b"metadata".to_vec()
        ));
    })
}

#[test_case(Some(
VestingScheduleParams {
        blocks_before_cliff: 100,
        cliff_amount_percentage: Permill::from_percent(10),
        linear_vesting_duration: 100,
    }); "with_vesting_schedule")]
#[test_case(None; "without_vesting_schedule")]
fn issuer_transfer_ok_with_event_deposit_given_non_existing_user(
    vesting: Option<VestingScheduleParams>,
) {
    assert!(DEFAULT_INITIAL_ISSUANCE >= 2 * DEFAULT_USER_BALANCE);
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();

        IssuerTransferFixture::new()
            .with_outputs(new_issuer_transfers(vec![(
                FIRST_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE,
                vesting.clone(),
            )]))
            .run();

        last_event_eq!(RawEvent::TokenAmountTransferredByIssuer(
            DEFAULT_TOKEN_ID,
            DEFAULT_ISSUER_MEMBER_ID,
            validated_outputs![(
                Validated::<_>::NonExisting(FIRST_USER_MEMBER_ID),
                DEFAULT_USER_BALANCE,
                vesting,
                None
            )],
            b"metadata".to_vec()
        ));
    })
}

#[test]
fn issuer_transfer_ok_with_token_supply_changed() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        let token_supply_pre = Token::token_info_by_id(DEFAULT_TOKEN_ID).total_supply;

        IssuerTransferFixture::new().run();

        assert_eq!(
            Token::token_info_by_id(DEFAULT_TOKEN_ID).total_supply,
            token_supply_pre
        );
    })
}

#[test]
fn issuer_transfer_ok_with_src_funds_decreased() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        let src_funds_pre =
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &DEFAULT_ISSUER_MEMBER_ID)
                .unwrap()
                .amount;

        IssuerTransferFixture::new().run();

        assert_eq!(
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &DEFAULT_ISSUER_MEMBER_ID)
                .unwrap()
                .amount,
            src_funds_pre - DEFAULT_USER_BALANCE,
        );
    })
}

#[test]
fn issuer_transfer_ok_with_non_existing_destination_and_account_number_increased() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();
        let accounts_number_pre = Token::token_info_by_id(DEFAULT_TOKEN_ID).accounts_number;

        IssuerTransferFixture::new().run();

        assert_eq!(
            Token::token_info_by_id(DEFAULT_TOKEN_ID).accounts_number,
            accounts_number_pre + 1u64
        );
    })
}

#[test]
fn issuer_transfer_ok_with_bloat_bond_deposited_into_tresury() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();
        let issuer_balance_pre = Balances::usable_balance(&DEFAULT_ISSUER_ACCOUNT_ID);
        let treasury_balance_pre = Balances::usable_balance(&Token::module_treasury_account());

        IssuerTransferFixture::new().run();

        assert_eq!(
            Balances::usable_balance(&DEFAULT_ISSUER_ACCOUNT_ID),
            issuer_balance_pre - DEFAULT_BLOAT_BOND,
            "issuer joy balance not decreased by bloat bond"
        );
        assert_eq!(
            Balances::usable_balance(&Token::module_treasury_account()),
            treasury_balance_pre + DEFAULT_BLOAT_BOND,
            "treasury balance not increased by bloat bond"
        );
    })
}

#[test]
fn issuer_transfer_ok_with_account_created_with_vesting() {
    let vesting = VestingScheduleParams {
        blocks_before_cliff: 100,
        cliff_amount_percentage: Permill::from_percent(10),
        linear_vesting_duration: 100,
    };
    build_default_test_externalities().execute_with(|| {
        let account_data = AccountData {
            next_vesting_transfer_id: 1,
            ..AccountData::new_with_vesting_and_bond::<Test>(
                VestingSource::IssuerTransfer(0),
                VestingSchedule::from_params(
                    System::block_number(),
                    DEFAULT_USER_BALANCE,
                    vesting.clone(),
                ),
                RepayableBloatBond::new(DEFAULT_BLOAT_BOND, None),
            )
            .unwrap()
        };
        TokenContext::with_issuer_only();

        IssuerTransferFixture::new()
            .with_outputs(new_issuer_transfers(vec![(
                FIRST_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE,
                Some(vesting.clone()),
            )]))
            .run();

        assert_eq!(
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID).unwrap(),
            account_data,
        )
    })
}

#[test]
fn issuer_transfer_ok_with_vested_tranfer_and_existing_account() {
    let vesting = VestingScheduleParams {
        blocks_before_cliff: 100,
        cliff_amount_percentage: Permill::from_percent(10),
        linear_vesting_duration: 100,
    };
    build_default_test_externalities().execute_with(|| {
        let mut account_data = AccountData {
            next_vesting_transfer_id: 1,
            ..AccountData::new_with_vesting_and_bond::<Test>(
                VestingSource::IssuerTransfer(0),
                VestingSchedule::from_params(
                    System::block_number(),
                    DEFAULT_USER_BALANCE,
                    vesting.clone(),
                ),
                RepayableBloatBond::new(DEFAULT_BLOAT_BOND, None),
            )
            .unwrap()
        };
        account_data.amount += DEFAULT_USER_BALANCE;
        TokenContext::with_issuer_and_first_user();

        IssuerTransferFixture::new()
            .with_outputs(new_issuer_transfers(vec![(
                FIRST_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE,
                Some(vesting.clone()),
            )]))
            .run();

        assert_eq!(
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID).unwrap(),
            account_data,
        )
    })
}
#[test_case(true; "given_existing_destination")]
#[test_case(false; "given_non_existing_destination")]
fn issuer_transfer_ok_with_non_vesting_transfer(existing_destination: bool) {
    let amount = DEFAULT_USER_BALANCE
        + if existing_destination {
            DEFAULT_USER_BALANCE
        } else {
            0
        };
    build_default_test_externalities().execute_with(|| {
        if existing_destination {
            TokenContext::with_issuer_and_first_user()
        } else {
            TokenContext::with_issuer_only()
        }

        IssuerTransferFixture::new().run();

        assert_eq!(
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID).unwrap(),
            AccountData::new_with_amount_and_bond(
                amount,
                RepayableBloatBond::new(DEFAULT_BLOAT_BOND, None)
            )
        )
    })
}

#[test]
fn issuer_multiple_permissioned_token_transfers_ok_with_vesting_cleanup_executed() {
    let max_vesting_schedules_num = MaxVestingSchedulesPerAccountPerToken::get();
    let vesting = VestingScheduleParams {
        blocks_before_cliff: 200,
        cliff_amount_percentage: Permill::from_percent(20),
        linear_vesting_duration: 200,
    };
    assert!(DEFAULT_USER_BALANCE * (max_vesting_schedules_num as u128) <= DEFAULT_INITIAL_ISSUANCE);
    build_default_test_externalities().execute_with(|| {
        // Arrange
        TokenContext::with_issuer_and_first_user();
        // Create max vesting schedules
        for i in 0u64..max_vesting_schedules_num.into() {
            IssuerTransferFixture::new()
                .with_output(
                    FIRST_USER_MEMBER_ID,
                    DEFAULT_USER_BALANCE,
                    Some(vesting.clone()),
                )
                .run();
            let dst_acc_data =
                Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID).unwrap();
            assert_eq!(dst_acc_data.next_vesting_transfer_id, i + 1);
        }
        // Go to vesting end block
        System::set_block_number(1 + vesting.blocks_before_cliff + vesting.linear_vesting_duration);

        // Act
        IssuerTransferFixture::new()
            .with_output(
                FIRST_USER_MEMBER_ID,
                DEFAULT_USER_BALANCE,
                Some(vesting.clone()),
            )
            .run();

        // Assert
        let dst_acc_data =
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID).unwrap();
        assert_eq!(
            dst_acc_data.next_vesting_transfer_id,
            1u64.saturating_add(max_vesting_schedules_num.into())
        );
        assert_eq!(
            dst_acc_data.vesting_schedules.len() as u64,
            max_vesting_schedules_num as u64
        );
        last_event_eq!(RawEvent::TokenAmountTransferredByIssuer(
            DEFAULT_TOKEN_ID,
            DEFAULT_ISSUER_MEMBER_ID,
            validated_outputs![(
                Validated::<_>::Existing(FIRST_USER_MEMBER_ID),
                DEFAULT_USER_BALANCE,
                Some(vesting),
                Some(VestingSource::IssuerTransfer(0))
            )],
            b"metadata".to_vec()
        ));
    })
}

#[test_case(ed(), (None,None,None); "just_ed")]
#[test_case(ed() + 1, (Some(DEFAULT_ISSUER_ACCOUNT_ID),None,None); "more_than_ed")]
#[test_case(ed() + DEFAULT_BLOAT_BOND, (Some(DEFAULT_ISSUER_ACCOUNT_ID),None,None); "ed_and_bloat_bond")]
#[test_case(ed() + DEFAULT_BLOAT_BOND + 1, (Some(DEFAULT_ISSUER_ACCOUNT_ID), Some(DEFAULT_ISSUER_ACCOUNT_ID),None); "more_than_ed_and_bloat_bond")]
#[test_case(ed() + DEFAULT_BLOAT_BOND * 2, (Some(DEFAULT_ISSUER_ACCOUNT_ID), Some(DEFAULT_ISSUER_ACCOUNT_ID),None); "ed_and_twice_bloat_bond")]
#[test_case(ed() + DEFAULT_BLOAT_BOND * 2 + 1, (Some(DEFAULT_ISSUER_ACCOUNT_ID), Some(DEFAULT_ISSUER_ACCOUNT_ID),Some(DEFAULT_ISSUER_ACCOUNT_ID)); "more_than_ed_and_twice_bloat_bond")]
#[test_case(ed() + DEFAULT_BLOAT_BOND * 3, (Some(DEFAULT_ISSUER_ACCOUNT_ID), Some(DEFAULT_ISSUER_ACCOUNT_ID),Some(DEFAULT_ISSUER_ACCOUNT_ID)); "ed_and_trice_bloat_bond")]
fn issuer_transfer_ok_with_invitation_locked_funds_with_locked_balance(
    locked_balance: Balance,
    expected_bloat_bond_restricted_to: (Option<AccountId>, Option<AccountId>, Option<AccountId>),
) {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();
        set_invitation_lock(&DEFAULT_ISSUER_ACCOUNT_ID, locked_balance);

        IssuerTransferFixture::new()
            .with_outputs(new_issuer_transfers(vec![
                (FIRST_USER_MEMBER_ID, DEFAULT_USER_BALANCE, None),
                (SECOND_USER_MEMBER_ID, DEFAULT_USER_BALANCE, None),
            ]))
            .run();

        assert_eq!(
            Balances::usable_balance(Token::module_treasury_account()),
            3 * DEFAULT_BLOAT_BOND + ed()
        );
        assert_eq!(
            System::account(DEFAULT_ISSUER_ACCOUNT_ID).data,
            balances::AccountData {
                free: ed(),
                reserved: 0,
                misc_frozen: locked_balance,
                fee_frozen: 0
            }
        );
        assert_eq!(
            Token::account_info_by_token_and_member(DEFAULT_TOKEN_ID, FIRST_USER_MEMBER_ID)
                .bloat_bond,
            RepayableBloatBond::new(DEFAULT_BLOAT_BOND, expected_bloat_bond_restricted_to.0)
        );
        assert_eq!(
            Token::account_info_by_token_and_member(DEFAULT_TOKEN_ID, SECOND_USER_MEMBER_ID)
                .bloat_bond,
            RepayableBloatBond::new(DEFAULT_BLOAT_BOND, expected_bloat_bond_restricted_to.1)
        );
    });
}

#[test]
fn issuer_transfer_fails_with_insufficient_locked_funds() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();
        Balances::make_free_balance_be(&DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed() - 1);
        set_invitation_lock(&DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed() - 1);

        let result_after_first_lock = IssuerTransferFixture::new().execute_call();

        assert_err!(
            result_after_first_lock,
            Error::<Test>::InsufficientJoyBalance
        );

        // Increase balance by 1, but lock ED and those funds with another, not-allowed lock
        increase_account_balance(&DEFAULT_ISSUER_ACCOUNT_ID, 1);
        set_staking_candidate_lock(&DEFAULT_ISSUER_ACCOUNT_ID, ed() + 1);

        let result_after_second_lock = IssuerTransferFixture::new().execute_call();

        assert_err!(
            result_after_second_lock,
            Error::<Test>::InsufficientJoyBalance
        );
    });
}

#[test]
fn issuer_transfer_fails_with_incompatible_locked_funds() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();
        Balances::make_free_balance_be(&DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed());
        set_staking_candidate_lock(&DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed());

        let result = IssuerTransferFixture::new().execute_call();

        assert_err!(result, Error::<Test>::InsufficientJoyBalance);
    });
}
