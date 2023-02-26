#![cfg(test)]

use crate::balance;
use crate::errors::Error;
use crate::tests::fixtures::*;
use crate::tests::mock::*;
use crate::types::Joy;
use frame_support::{assert_err, assert_ok};
use sp_arithmetic::Permill;
use sp_runtime::{traits::Zero, DispatchError};
use test_case::test_case;

/////////////////////////////////////////////////////////
////////////////// SALE INITIALIZATION //////////////////
/////////////////////////////////////////////////////////

#[test]
fn unsuccesful_token_sale_init_with_invalid_token_id() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();

        let result = InitTokenSaleFixture::new()
            .with_token_id(DEFAULT_TOKEN_ID + 1)
            .execute_call();

        assert_err!(result, Error::<Test>::TokenDoesNotExist)
    })
}

#[test]
fn unsuccesful_token_sale_init_with_start_block_in_the_past() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();

        let result = InitTokenSaleFixture::new()
            .with_start_block(Some(Zero::zero()))
            .execute_call();

        assert_err!(result, Error::<Test>::SaleStartingBlockInThePast)
    })
}

#[test]
fn unsuccesful_token_sale_init_with_zero_duration() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();

        let result = InitTokenSaleFixture::new().with_duration(0).execute_call();

        assert_err!(result, Error::<Test>::SaleDurationIsZero);
    })
}

#[test]
fn unsuccesful_token_sale_init_with_zero_upper_bound_quantity() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        let result = InitTokenSaleFixture::new()
            .with_upper_bound_quantity(0)
            .execute_call();

        assert_err!(result, Error::<Test>::SaleUpperBoundQuantityIsZero)
    })
}

#[test]
fn unsuccesful_token_sale_init_with_zero_unit_price() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();

        let result = InitTokenSaleFixture::new()
            .with_unit_price(balance!(0))
            .execute_call();

        assert_err!(result, Error::<Test>::SaleUnitPriceIsZero);
    })
}

#[test]
fn unsuccesful_token_sale_init_with_zero_cap_per_member() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();

        let result = InitTokenSaleFixture::new()
            .with_cap_per_member(Some(Zero::zero()))
            .execute_call();

        assert_err!(result, Error::<Test>::SaleCapPerMemberIsZero);
    })
}

#[test]
fn unsuccesful_token_sale_init_with_duration_too_short() {
    let min_sale_duration: BlockNumber = 10u64;
    let config = GenesisConfigBuilder::new_empty()
        .with_min_sale_duration(min_sale_duration)
        .build();

    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        let result = InitTokenSaleFixture::new()
            .with_duration(min_sale_duration - 1)
            .execute_call();

        assert_err!(result, Error::<Test>::SaleDurationTooShort)
    })
}

#[test]
fn unsuccesful_token_sale_init_with_upper_bound_quantity_exceeding_source_balance() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();

        let result = InitTokenSaleFixture::new()
            .with_upper_bound_quantity(DEFAULT_INITIAL_ISSUANCE + 1)
            .execute_call();

        assert_err!(result, Error::<Test>::InsufficientTransferrableBalance);
    })
}

#[test]
fn unsuccesful_token_sale_init_with_invalid_source() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();

        let result = InitTokenSaleFixture::new()
            .with_member_id(FIRST_USER_MEMBER_ID)
            .execute_call();

        assert_err!(result, Error::<Test>::AccountInformationDoesNotExist);
    })
}

#[test]
fn unsuccesful_token_sale_init_when_token_not_idle() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();

        let result = InitTokenSaleFixture::new().execute_call();

        assert_err!(result, Error::<Test>::TokenIssuanceNotInIdleState);
    })
}

#[test]
fn unsuccesful_token_sale_init_when_previous_sale_not_finalized() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        InitTokenSaleFixture::new().run();
        increase_account_balance(
            &FIRST_USER_MEMBER_ID,
            <Test as crate::Config>::JoyExistentialDeposit::get()
                + DEFAULT_SALE_UNIT_PRICE * DEFAULT_SALE_PURCHASE_AMOUNT,
        );
        PurchaseTokensOnSaleFixture::new().run();
        increase_block_number_by(DEFAULT_SALE_DURATION);
        let result = InitTokenSaleFixture::new().execute_call();

        assert_err!(result, Error::<Test>::PreviousSaleNotFinalized);
    })
}

#[test]
fn succesful_token_sale_init() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();

        InitTokenSaleFixture::new().run();

        // Assert Idle state after sale ends
        increase_block_number_by(DEFAULT_SALE_DURATION);
        let token = Token::token_info_by_id(1);
        assert_eq!(IssuanceState::of::<Test>(&token), IssuanceState::Idle);
    })
}

#[test]
fn succesful_token_sale_init_with_custom_start_block() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();

        InitTokenSaleFixture::new()
            .with_start_block(Some(100))
            .run();

        // Assert sale begins at block 100
        increase_block_number_by(99);

        let token = Token::token_info_by_id(DEFAULT_TOKEN_ID);
        matches!(
            IssuanceState::of::<Test>(&token),
            IssuanceState::Sale(TokenSale {
                start_block: 100,
                ..
            })
        );

        // Assert Idle state at block block 100 + DEFAULT_SALE_DURATION
        increase_block_number_by(DEFAULT_SALE_DURATION);
        let token = Token::token_info_by_id(DEFAULT_TOKEN_ID);
        assert_eq!(IssuanceState::of::<Test>(&token), IssuanceState::Idle);
    })
}

/////////////////////////////////////////////////////////
///////////////// UPCOMING SALE UPDATE //////////////////
/////////////////////////////////////////////////////////

#[test]
fn unsuccesful_upcoming_sale_update_with_invalid_token_id() {
    build_default_test_externalities().execute_with(|| {
        let result = UpdateUpcomingSaleFixture::new().execute_call();
        assert_err!(result, Error::<Test>::TokenDoesNotExist)
    })
}

#[test]
fn unsuccesful_upcoming_sale_update_when_token_is_idle() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();

        let result = UpdateUpcomingSaleFixture::new().execute_call();

        assert_err!(result, Error::<Test>::NoUpcomingSale)
    })
}

#[test]
fn unsuccesful_upcoming_sale_update_when_sale_is_ongoing() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        InitTokenSaleFixture::new().run();

        let result = UpdateUpcomingSaleFixture::new().execute_call();

        assert_err!(result, Error::<Test>::NoUpcomingSale)
    })
}

#[test]
fn unsuccesful_upcoming_sale_update_with_start_block_in_the_past() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        InitTokenSaleFixture::new()
            .with_start_block(Some(100))
            .run();

        let result = UpdateUpcomingSaleFixture::new()
            .with_new_start_block(Some(0))
            .execute_call();

        assert_err!(result, Error::<Test>::SaleStartingBlockInThePast);
    })
}

#[test]
fn unsuccesful_upcoming_sale_update_with_zero_duration() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        InitTokenSaleFixture::new()
            .with_start_block(Some(100))
            .run();
        let result = UpdateUpcomingSaleFixture::new()
            .with_new_duration(Some(0))
            .execute_call();

        assert_err!(result, Error::<Test>::SaleDurationIsZero);
    })
}

#[test]
fn unsuccesful_upcoming_sale_update_with_duration_too_short() {
    let min_sale_duration = 10u64;
    let config = GenesisConfigBuilder::new_empty()
        .with_min_sale_duration(min_sale_duration)
        .build();

    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        InitTokenSaleFixture::new()
            .with_start_block(Some(100))
            .run();
        let result = UpdateUpcomingSaleFixture::new()
            .with_new_duration(Some(min_sale_duration - 1))
            .execute_call();
        assert_err!(result, Error::<Test>::SaleDurationTooShort);
    })
}

#[test]
fn successful_upcoming_sale_update() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        InitTokenSaleFixture::new()
            .with_start_block(Some(100))
            .run();

        UpdateUpcomingSaleFixture::new()
            .with_new_start_block(Some(20))
            .with_new_duration(Some(50))
            .run();

        let token = Token::token_info_by_id(1);
        assert!(matches!(
            IssuanceState::of::<Test>(&token),
            IssuanceState::UpcomingSale { .. }
        ));

        increase_block_number_by(19);
        let token = Token::token_info_by_id(1);
        assert!(matches!(
            IssuanceState::of::<Test>(&token),
            IssuanceState::Sale { .. }
        ));

        increase_block_number_by(50);
        let token = Token::token_info_by_id(1);
        assert_eq!(IssuanceState::of::<Test>(&token), IssuanceState::Idle);
    })
}

/////////////////////////////////////////////////////////
//////////////////// SALE PURCHASES /////////////////////
/////////////////////////////////////////////////////////

#[test]
fn unsuccesful_sale_purchase_non_existing_token() {
    build_default_test_externalities().execute_with(|| {
        let result = PurchaseTokensOnSaleFixture::new().execute_call();

        assert_err!(result, Error::<Test>::TokenDoesNotExist);
    })
}

#[test]
fn unsuccesful_sale_purchase_when_no_sale() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();

        let result = PurchaseTokensOnSaleFixture::new().execute_call();

        assert_err!(result, Error::<Test>::NoActiveSale);
    })
}

#[test]
fn unsuccesful_sale_purchase_when_sale_not_started_yet() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        InitTokenSaleFixture::new().with_start_block(Some(10)).run();

        let result = PurchaseTokensOnSaleFixture::new().execute_call();

        assert_err!(result, Error::<Test>::NoActiveSale);
    })
}

#[test]
fn unsuccesful_sale_purchase_after_sale_finished() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        InitTokenSaleFixture::new().run();
        increase_block_number_by(DEFAULT_SALE_DURATION);

        let result = PurchaseTokensOnSaleFixture::new().execute_call();

        assert_err!(result, Error::<Test>::NoActiveSale);
    })
}

#[test]
fn unsuccesful_sale_purchase_insufficient_joy_balance_new_account() {
    build_default_test_externalities().execute_with(|| {
        increase_account_balance(
            &FIRST_USER_MEMBER_ID,
            <Test as crate::Config>::JoyExistentialDeposit::get()
                + DEFAULT_SALE_UNIT_PRICE * DEFAULT_SALE_PURCHASE_AMOUNT
                + DEFAULT_BLOAT_BOND
                - 1,
        );
        IssueTokenFixture::new().run();
        InitTokenSaleFixture::new().run();
        let result = PurchaseTokensOnSaleFixture::new().execute_call();

        assert_err!(result, Error::<Test>::InsufficientJoyBalance);
    })
}

#[test]
fn unsuccesful_sale_purchase_insufficient_joy_balance_existing_account() {
    build_default_test_externalities().execute_with(|| {
        increase_account_balance(
            &FIRST_USER_ACCOUNT_ID,
            <Test as crate::Config>::JoyExistentialDeposit::get()
                + DEFAULT_SALE_UNIT_PRICE * DEFAULT_SALE_PURCHASE_AMOUNT
                - 1,
        );
        IssueTokenFixture::new().run();
        InitTokenSaleFixture::new().run();
        let result = PurchaseTokensOnSaleFixture::new().execute_call();

        assert_err!(result, Error::<Test>::InsufficientJoyBalance);
    })
}

#[test]
fn unsuccesful_sale_purchase_amount_exceeds_quantity_left() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        InitTokenSaleFixture::new().run();
        increase_account_balance(
            &FIRST_USER_MEMBER_ID,
            <Test as crate::Config>::JoyExistentialDeposit::get()
                + DEFAULT_SALE_UNIT_PRICE * (DEFAULT_INITIAL_ISSUANCE + 1),
        );
        let result = PurchaseTokensOnSaleFixture::new()
            .with_amount(DEFAULT_INITIAL_ISSUANCE + 1)
            .execute_call();

        assert_err!(result, Error::<Test>::NotEnoughTokensOnSale);
    })
}

#[test]
fn unsuccesful_sale_purchase_amount_is_zero() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        InitTokenSaleFixture::new().run();
        let result = PurchaseTokensOnSaleFixture::new()
            .with_amount(0)
            .execute_call();

        assert_err!(result, Error::<Test>::SalePurchaseAmountIsZero);
    })
}

#[test]
fn unsuccesful_sale_purchase_vesting_balances_limit_reached() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        let max_vesting_schedules =
            <Test as crate::Config>::MaxVestingSchedulesPerAccountPerToken::get();
        increase_account_balance(
            &FIRST_USER_MEMBER_ID,
            ed() + DEFAULT_SALE_PURCHASE_AMOUNT
                .saturating_mul(DEFAULT_SALE_UNIT_PRICE)
                .saturating_mul((max_vesting_schedules + 1).into()),
        );
        for _ in 0..<Test as crate::Config>::MaxVestingSchedulesPerAccountPerToken::get() {
            InitTokenSaleFixture::new()
                .with_upper_bound_quantity(DEFAULT_SALE_PURCHASE_AMOUNT)
                .with_vesting_blocks_before_cliff(
                    DEFAULT_SALE_DURATION * (max_vesting_schedules + 1) as u64,
                )
                .with_linear_vesting_duration(100)
                .with_cliff_amount_percentage(Permill::from_percent(0))
                .run();
            PurchaseTokensOnSaleFixture::new().run();
            increase_block_number_by(DEFAULT_SALE_DURATION);
        }
        InitTokenSaleFixture::new()
            .with_upper_bound_quantity(DEFAULT_SALE_PURCHASE_AMOUNT)
            .run();

        let result = PurchaseTokensOnSaleFixture::new().execute_call();

        assert_err!(
            result,
            Error::<Test>::MaxVestingSchedulesPerAccountPerTokenReached,
        );
    })
}

#[test]
fn unsuccesful_sale_purchase_with_cap_exceeded_with_vesting() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        InitTokenSaleFixture::new()
            .with_cap_per_member(Some(DEFAULT_SALE_PURCHASE_AMOUNT))
            .run();
        increase_account_balance(
            &FIRST_USER_MEMBER_ID,
            <Test as crate::Config>::JoyExistentialDeposit::get()
                + DEFAULT_SALE_UNIT_PRICE * (DEFAULT_SALE_PURCHASE_AMOUNT + 1),
        );
        // Make succesful purchase
        PurchaseTokensOnSaleFixture::new().run();

        // Try making another one (that would exceed the cap)
        let result = PurchaseTokensOnSaleFixture::new()
            .with_amount(1)
            .execute_call();

        assert_err!(result, Error::<Test>::SalePurchaseCapExceeded);
    })
}

#[test]
fn unsuccesful_sale_purchase_with_cap_exceeded_no_vesting() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        InitTokenSaleFixture::new()
            .with_cap_per_member(Some(DEFAULT_SALE_PURCHASE_AMOUNT))
            .run();
        increase_account_balance(
            &FIRST_USER_MEMBER_ID,
            ed() + DEFAULT_SALE_UNIT_PRICE * (DEFAULT_SALE_PURCHASE_AMOUNT + 1),
        );
        // Make succesful purchase
        PurchaseTokensOnSaleFixture::new().run();

        // Try making another one (that would exceed the cap)
        let result = PurchaseTokensOnSaleFixture::new()
            .with_amount(1)
            .execute_call();

        assert_err!(result, Error::<Test>::SalePurchaseCapExceeded);
    })
}

#[test]
fn unsuccesful_sale_purchase_with_permissioned_token_and_non_existing_account() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only_permissioned();
        InitTokenSaleFixture::new().run();
        increase_account_balance(
            &FIRST_USER_MEMBER_ID,
            ed() + DEFAULT_SALE_UNIT_PRICE * DEFAULT_SALE_PURCHASE_AMOUNT,
        );

        let result = PurchaseTokensOnSaleFixture::new().execute_call();

        assert_err!(result, Error::<Test>::AccountInformationDoesNotExist);
    })
}

#[test]
fn unsuccesful_sale_purchase_with_invalid_member_controller() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        InitTokenSaleFixture::new().run();
        increase_account_balance(
            &FIRST_USER_MEMBER_ID,
            <Test as crate::Config>::JoyExistentialDeposit::get()
                + (DEFAULT_SALE_UNIT_PRICE * DEFAULT_SALE_PURCHASE_AMOUNT),
        );

        let result = PurchaseTokensOnSaleFixture::new()
            .with_sender(DEFAULT_ISSUER_MEMBER_ID)
            .execute_call();

        assert_err!(
            result,
            DispatchError::Other("origin signer not a member controller account",)
        );
    })
}

#[test]
fn succesful_sale_purchases_non_existing_account_no_vesting_schedule() {
    build_default_test_externalities().execute_with(|| {
        increase_account_balance(&DEFAULT_ISSUER_MEMBER_ID, ed() + DEFAULT_BLOAT_BOND);
        IssueTokenFixture::new().run();
        InitTokenSaleFixture::new().run();
        increase_account_balance(
            &FIRST_USER_ACCOUNT_ID,
            ed() + (DEFAULT_SALE_UNIT_PRICE * DEFAULT_SALE_PURCHASE_AMOUNT * 2)
                + DEFAULT_BLOAT_BOND,
        );
        PurchaseTokensOnSaleFixture::new().run();

        let buyer_acc_info =
            Token::account_info_by_token_and_member(DEFAULT_TOKEN_ID, FIRST_USER_MEMBER_ID);
        assert_eq!(
            buyer_acc_info.transferrable::<Test>(System::block_number()),
            DEFAULT_SALE_PURCHASE_AMOUNT * 2
        );
    })
}

#[test]
fn succesful_sale_purchases_non_existing_account_vesting_schedule() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_only();
        InitTokenSaleFixture::new()
            .with_vesting_blocks_before_cliff(100)
            .with_linear_vesting_duration(200)
            .with_cliff_amount_percentage(Permill::from_percent(30))
            .run();
        increase_account_balance(
            &FIRST_USER_ACCOUNT_ID,
            ed() + (DEFAULT_SALE_UNIT_PRICE * DEFAULT_SALE_PURCHASE_AMOUNT * 2)
                + DEFAULT_BLOAT_BOND,
        );
        PurchaseTokensOnSaleFixture::new().run();
        PurchaseTokensOnSaleFixture::new().run();

        // At sale end block expect 0 tokens in available balance (due to 100 blocks remaining until cliff)
        increase_block_number_by(DEFAULT_SALE_DURATION);
        let buyer_acc_info = Token::account_info_by_token_and_member(1, FIRST_USER_MEMBER_ID);
        assert_eq!(
            buyer_acc_info.transferrable::<Test>(System::block_number()),
            0
        );

        // After cliff expect 30% of tokens in available balance (cliff_amount_percentage)
        increase_block_number_by(100);
        let buyer_acc_info = Token::account_info_by_token_and_member(1, FIRST_USER_MEMBER_ID);
        assert_eq!(
            buyer_acc_info.transferrable::<Test>(System::block_number()),
            Permill::from_percent(30) * DEFAULT_SALE_PURCHASE_AMOUNT * 2
        );

        // After 50% of duration (100 blocks), expect 30% + (50% * 70%) = 65% of tokens in available balance
        increase_block_number_by(100);
        let buyer_acc_info = Token::account_info_by_token_and_member(1, FIRST_USER_MEMBER_ID);
        assert_eq!(
            buyer_acc_info.transferrable::<Test>(System::block_number()),
            Permill::from_percent(65) * DEFAULT_SALE_PURCHASE_AMOUNT * 2
        );

        // At the end of vesting expect 100% of tokens in available balance
        increase_block_number_by(100);
        let buyer_acc_info = Token::account_info_by_token_and_member(1, FIRST_USER_MEMBER_ID);
        assert_eq!(
            buyer_acc_info.transferrable::<Test>(System::block_number()),
            DEFAULT_SALE_PURCHASE_AMOUNT * 2
        );
    })
}

#[test]
fn succesful_sale_purchase_existing_account_permissioned_token() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user_permissioned();
        InitTokenSaleFixture::new().run();
        increase_account_balance(
            &FIRST_USER_ACCOUNT_ID,
            DEFAULT_SALE_UNIT_PRICE * DEFAULT_SALE_PURCHASE_AMOUNT + ed(),
        );
        let result = PurchaseTokensOnSaleFixture::new().execute_call();

        assert_ok!(result);
    })
}

#[test]
fn succesful_sale_purchases_equal_to_member_cap_on_subsequent_sales() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        increase_account_balance(
            &FIRST_USER_ACCOUNT_ID,
            DEFAULT_SALE_UNIT_PRICE * DEFAULT_SALE_PURCHASE_AMOUNT * 2 + ed(),
        );
        let buyer_amount_pre =
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID)
                .unwrap()
                .amount;
        for _ in 0..2 {
            InitTokenSaleFixture::new()
                .with_cap_per_member(Some(DEFAULT_SALE_PURCHASE_AMOUNT))
                .with_upper_bound_quantity(DEFAULT_SALE_PURCHASE_AMOUNT)
                .run();

            PurchaseTokensOnSaleFixture::new()
                .with_amount(DEFAULT_SALE_PURCHASE_AMOUNT)
                .run();
        }
        let buyer_acc_info =
            Token::account_info_by_token_and_member(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID);
        assert_eq!(
            buyer_acc_info.amount,
            DEFAULT_SALE_PURCHASE_AMOUNT * 2 + buyer_amount_pre
        );
    })
}

#[test]
fn succesful_sale_purchases_with_platform_fee() {
    let sale_platform_fee = Permill::from_percent(30);
    let config = GenesisConfigBuilder::new_empty()
        .with_sale_platform_fee(sale_platform_fee)
        .build();

    build_test_externalities_with_balances(
        config,
        vec![(DEFAULT_ISSUER_ACCOUNT_ID, 2 * DEFAULT_BLOAT_BOND + ed())],
    )
    .execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        InitTokenSaleFixture::new().with_unit_price(1).run();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, 100 + ed());
        let mut issuer_balance_pre = Balances::usable_balance(&DEFAULT_ISSUER_ACCOUNT_ID);
        PurchaseTokensOnSaleFixture::new().with_amount(99).run();
        // 99 tokens bought for 1 JOY each - expect `99 - floor(99 * 30%) = 99 - 29 = 70` JOY transferred
        assert_eq!(
            Joy::<Test>::usable_balance(DEFAULT_ISSUER_ACCOUNT_ID),
            issuer_balance_pre + 70
        );

        issuer_balance_pre = Balances::usable_balance(&DEFAULT_ISSUER_ACCOUNT_ID);

        PurchaseTokensOnSaleFixture::new().with_amount(1).run();
        // 1 token bought for 1 JOY - expect `1 - floor(1 * 30%) = 1 - 0 = 1` JOY transferred
        assert_eq!(
            Joy::<Test>::usable_balance(DEFAULT_ISSUER_ACCOUNT_ID),
            issuer_balance_pre + 1
        );
        // expect "empty" buyer JOY balance
        assert_eq!(Joy::<Test>::usable_balance(FIRST_USER_ACCOUNT_ID), ed())
    })
}

#[test]
fn succesful_sale_purchases_with_no_sale_earnings_destination_provided() {
    let sale_platform_fee = Permill::from_percent(30);
    let config = GenesisConfigBuilder::new_empty()
        .with_sale_platform_fee(sale_platform_fee)
        .build();
    build_test_externalities_with_balances(
        config,
        vec![(DEFAULT_ISSUER_ACCOUNT_ID, 2 * DEFAULT_BLOAT_BOND + ed())],
    )
    .execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        InitTokenSaleFixture::new()
            .with_unit_price(1)
            .with_earnings_destination(None)
            .run();
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, 100 + ed());
        let joy_supply_pre = Joy::<Test>::total_issuance();
        PurchaseTokensOnSaleFixture::new().with_amount(100).run();

        // expect "empty" buyer JOY balance
        assert_eq!(Joy::<Test>::usable_balance(FIRST_USER_ACCOUNT_ID), ed());

        // expect JOY supply decreased by 100
        assert_eq!(
            Joy::<Test>::total_issuance(),
            joy_supply_pre.saturating_sub(100)
        );
    })
}

#[test]
fn succesful_sale_purchase_auto_finalizing_the_sale() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        InitTokenSaleFixture::new()
            .with_upper_bound_quantity(DEFAULT_SALE_PURCHASE_AMOUNT)
            .run();
        increase_account_balance(
            &FIRST_USER_ACCOUNT_ID,
            DEFAULT_SALE_PURCHASE_AMOUNT * DEFAULT_SALE_UNIT_PRICE + ed(),
        );

        PurchaseTokensOnSaleFixture::new().run();

        let token_data = Token::token_info_by_id(DEFAULT_TOKEN_ID);
        assert!(token_data.sale.is_none());
    })
}

#[test]
fn succesful_sale_purchase_not_auto_finalizing_the_sale() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        InitTokenSaleFixture::new()
            .with_auto_finalize(false)
            .with_upper_bound_quantity(DEFAULT_SALE_PURCHASE_AMOUNT)
            .run();
        increase_account_balance(
            &FIRST_USER_ACCOUNT_ID,
            DEFAULT_SALE_PURCHASE_AMOUNT * DEFAULT_SALE_UNIT_PRICE + ed(),
        );

        PurchaseTokensOnSaleFixture::new().run();

        let token_data = Token::token_info_by_id(DEFAULT_TOKEN_ID);
        assert!(token_data.sale.is_some());
    })
}

#[test_case(true; "with_platform_fee")]
#[test_case(false; "without_platform_fee")]
fn unsuccesful_sale_purchase_with_invitation_locked_funds(use_platform_fee: bool) {
    let sale_platform_fee = Permill::from_percent(30);
    let config = if use_platform_fee {
        GenesisConfigBuilder::new_empty()
            .with_sale_platform_fee(sale_platform_fee)
            .build()
    } else {
        GenesisConfigBuilder::new_empty().build()
    };
    build_test_externalities_with_balances(
        config,
        vec![(DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed())],
    )
    .execute_with(|| {
        TokenContext::with_issuer_only();
        increase_account_balance(
            &FIRST_USER_MEMBER_ID,
            DEFAULT_BLOAT_BOND + DEFAULT_SALE_UNIT_PRICE * DEFAULT_SALE_PURCHASE_AMOUNT + ed(),
        );
        InitTokenSaleFixture::new().run();
        set_invitation_lock(&FIRST_USER_MEMBER_ID, ed() + 1);

        let result = PurchaseTokensOnSaleFixture::new().execute_call();

        assert_err!(result, Error::<Test>::InsufficientJoyBalance);
    })
}

/////////////////////////////////////////////////////////
////////////////// FINALIZE TOKEN SALE //////////////////
/////////////////////////////////////////////////////////
#[test]
fn unsuccesful_finalize_token_sale_non_existing_token() {
    build_default_test_externalities().execute_with(|| {
        let result = FinalizeTokenSaleFixture::new().execute_call();

        assert_err!(result, Error::<Test>::TokenDoesNotExist);
    })
}

#[test]
fn unsuccesful_finalize_token_sale_no_sale() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();

        let result = FinalizeTokenSaleFixture::new().execute_call();

        assert_err!(result, Error::<Test>::NoTokensToRecover);
    })
}

#[test]
fn unsuccesful_finalize_token_sale_during_active_sale() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        InitTokenSaleFixture::new().run();

        let result = FinalizeTokenSaleFixture::new().execute_call();

        assert_err!(result, Error::<Test>::TokenIssuanceNotInIdleState);
    })
}

#[test]
fn unsuccesful_finalize_token_sale_when_no_tokens_left() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        InitTokenSaleFixture::new()
            .with_upper_bound_quantity(DEFAULT_SALE_PURCHASE_AMOUNT)
            .run();
        increase_account_balance(
            &FIRST_USER_ACCOUNT_ID,
            DEFAULT_SALE_UNIT_PRICE * DEFAULT_SALE_PURCHASE_AMOUNT + ed(),
        );
        PurchaseTokensOnSaleFixture::new().run();
        increase_block_number_by(DEFAULT_SALE_DURATION);

        let result = FinalizeTokenSaleFixture::new().execute_call();

        assert_err!(result, Error::<Test>::NoTokensToRecover);
    })
}

#[test]
fn succesful_finalize_token_sale() {
    build_default_test_externalities().execute_with(|| {
        TokenContext::with_issuer_and_first_user();
        InitTokenSaleFixture::new().run();
        increase_account_balance(
            &FIRST_USER_ACCOUNT_ID,
            DEFAULT_SALE_UNIT_PRICE * DEFAULT_SALE_PURCHASE_AMOUNT + ed(),
        );
        PurchaseTokensOnSaleFixture::new().execute_call().unwrap();
        increase_block_number_by(DEFAULT_SALE_DURATION);

        let result = FinalizeTokenSaleFixture::new().execute_call();

        assert_ok!(result);
    })
}
