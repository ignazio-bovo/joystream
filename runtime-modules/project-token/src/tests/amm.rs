#![cfg(test)]

use crate::tests::fixtures::*;
use crate::tests::mock::*;
use crate::types::{AmmCurve, AmmOperation};
use crate::{joy, last_event_eq, member, Error, RawEvent, RepayableBloatBondOf};
use frame_support::{assert_err, assert_ok};
use sp_runtime::{traits::Zero, DispatchError, Permill};

// --------------------- amm_buy -------------------------------

#[test]
fn amm_buy_noop_ok_with_zero_requested_amount() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().execute_call().unwrap();
        ActivateAmmFixture::new().execute_call().unwrap();
        let state_pre = sp_io::storage::root(sp_storage::StateVersion::V1);

        let result = AmmBuyFixture::new().with_amount(0u32.into()).execute_call();

        let state_post = sp_io::storage::root(sp_storage::StateVersion::V1);
        assert_ok!(result);
        assert_eq!(state_pre, state_post);
    })
}

#[test]
fn amm_buy_fails_with_invalid_token_specified() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().execute_call().unwrap();
        ActivateAmmFixture::new().execute_call().unwrap();

        let result = AmmBuyFixture::new()
            .with_token_id(DEFAULT_TOKEN_ID + 1)
            .execute_call();

        assert_err!(result, Error::<Test>::TokenDoesNotExist);
    })
}

#[test]
fn amm_buy_fails_with_member_and_origin_auth() {
    let config = GenesisConfigBuilder::new_empty().build();
    let (_, sender) = member!(3);
    build_test_externalities(config).execute_with(|| {
        IssueTokenFixture::new().execute_call().unwrap();
        ActivateAmmFixture::new().execute_call().unwrap();

        let result = AmmBuyFixture::new().with_sender(sender).execute_call();

        assert_err!(
            result,
            DispatchError::Other("origin signer not a member controller account")
        );
    })
}

#[test]
fn amm_buy_succeeds_with_new_user() {
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();
            let account_number_pre = Token::token_info_by_id(DEFAULT_TOKEN_ID).accounts_number;

            AmmBuyFixture::new().execute_call().unwrap();

            let account_number_post = Token::token_info_by_id(DEFAULT_TOKEN_ID).accounts_number;
            let account_data =
                Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID).unwrap();
            assert_eq!(account_number_post - account_number_pre, 1);
            assert_eq!(account_data.amount, DEFAULT_AMM_BUY_AMOUNT);
            assert_eq!(
                account_data.bloat_bond,
                RepayableBloatBondOf::<Test>::new(Token::bloat_bond(), None)
            );
        })
}

#[test]
fn amm_buy_fails_with_token_not_in_amm_state() {
    let config = GenesisConfigBuilder::new_empty().build();
    build_test_externalities(config).execute_with(|| {
        IssueTokenFixture::new().execute_call().unwrap();

        let result = AmmBuyFixture::new().execute_call();

        assert_err!(result, Error::<Test>::NotInAmmState);
    })
}

#[test]
fn amm_buy_succeeds_with_existing_user() {
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();
            InitTokenSaleFixture::new().execute_call().unwrap();
            PurchaseTokensOnSaleFixture::new().execute_call().unwrap();
            increase_block_number_by(DEFAULT_SALE_DURATION);
            FinalizeTokenSaleFixture::new().execute_call().unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();
            let user_amount_pre =
                Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID)
                    .unwrap()
                    .amount;

            AmmBuyFixture::new().execute_call().unwrap();

            let user_amount_post =
                Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID)
                    .unwrap()
                    .amount;
            assert_eq!(user_amount_post - user_amount_pre, DEFAULT_AMM_BUY_AMOUNT);
        })
}

#[test]
fn amm_buy_failed_with_slippage_constraint_violated() {
    let slippage_tolerance = (Permill::zero(), Balance::zero());
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();

            let result = AmmBuyFixture::new()
                .with_slippage_tolerance(Some(slippage_tolerance))
                .execute_call();

            assert_err!(result, Error::<Test>::SlippageToleranceExceeded);
        })
}

#[test]
fn amm_buy_fails_with_pricing_function_overflow() {
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();

            let result = AmmBuyFixture::new()
                .with_amount(Balance::max_value())
                .execute_call();

            assert_err!(result, Error::<Test>::ArithmeticError);
        })
}

#[test]
fn amm_buy_ok_with_creator_token_issuance_increased() {
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();
            let supply_pre = Token::token_info_by_id(DEFAULT_TOKEN_ID).total_supply;

            AmmBuyFixture::new().execute_call().unwrap();

            let supply_post = Token::token_info_by_id(DEFAULT_TOKEN_ID).total_supply;
            assert_eq!(supply_post, supply_pre + DEFAULT_AMM_BUY_AMOUNT);
        })
}

#[test]
fn amm_treasury_balance_correctly_increased_during_amm_buy() {
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();
            let amm_reserve_account = Token::amm_treasury_account(DEFAULT_TOKEN_ID);
            let amm_reserve_pre = Balances::usable_balance(amm_reserve_account);
            let correctly_computed_joy_amount =
                amm_function_values(DEFAULT_AMM_BUY_AMOUNT, DEFAULT_TOKEN_ID, AmmOperation::Buy);

            AmmBuyFixture::new().execute_call().unwrap();

            let amm_reserve_post = Balances::usable_balance(amm_reserve_account);
            assert_eq!(
                amm_reserve_post - amm_reserve_pre,
                correctly_computed_joy_amount
            );
        })
}

#[test]
fn amm_buy_fails_with_user_not_having_sufficient_usable_joy_required() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().execute_call().unwrap();
        ActivateAmmFixture::new().execute_call().unwrap();

        let result = AmmBuyFixture::new().execute_call();

        assert_err!(result, Error::<Test>::InsufficientJoyBalance);
    })
}

#[test]
fn user_joy_balance_correctly_decreased_during_amm_buy() {
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();
            let correctly_computed_joy_amount =
                amm_function_values(DEFAULT_AMM_BUY_AMOUNT, DEFAULT_TOKEN_ID, AmmOperation::Buy);
            let user_reserve_pre = Balances::usable_balance(FIRST_USER_ACCOUNT_ID);

            AmmBuyFixture::new()
                .with_amount(DEFAULT_AMM_BUY_AMOUNT)
                .execute_call()
                .unwrap();

            let user_reserve_post = Balances::usable_balance(FIRST_USER_ACCOUNT_ID);
            assert_eq!(
                user_reserve_pre - user_reserve_post,
                correctly_computed_joy_amount
            );
        })
}

#[test]
fn amm_buy_ok_with_event_deposit() {
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();
            let price =
                amm_function_values(DEFAULT_AMM_BUY_AMOUNT, DEFAULT_TOKEN_ID, AmmOperation::Buy);

            AmmBuyFixture::new()
                .with_amount(DEFAULT_AMM_BUY_AMOUNT)
                .execute_call()
                .unwrap();

            last_event_eq!(RawEvent::TokensBoughtOnAmm(
                DEFAULT_TOKEN_ID,
                FIRST_USER_MEMBER_ID,
                DEFAULT_AMM_BUY_AMOUNT,
                price,
            ));
        })
}

// --------------- ACTIVATION ----------------------------------

#[test]
fn amm_activation_fails_with_invalid_token_id() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().execute_call().unwrap();

        let result = ActivateAmmFixture::new()
            .with_token_id(DEFAULT_TOKEN_ID + 1)
            .execute_call();

        assert_err!(result, Error::<Test>::TokenDoesNotExist);
    })
}

#[test]
fn activation_fails_when_status_is_not_idle() {
    let config = GenesisConfigBuilder::new_empty().build();
    build_test_externalities(config).execute_with(|| {
        IssueTokenFixture::new().execute_call().unwrap();
        InitTokenSaleFixture::new().execute_call().unwrap();

        let result = ActivateAmmFixture::new().execute_call();

        assert_err!(result, Error::<Test>::TokenIssuanceNotInIdleState);
    })
}

#[test]
fn activation_fails_when_amm_status_already_active() {
    let config = GenesisConfigBuilder::new_empty().build();
    build_test_externalities(config).execute_with(|| {
        IssueTokenFixture::new()
            .with_empty_allocation()
            .execute_call()
            .unwrap();
        ActivateAmmFixture::new().execute_call().unwrap();

        let result = ActivateAmmFixture::new().execute_call();

        assert_err!(result, Error::<Test>::TokenIssuanceNotInIdleState);
    })
}

#[test]
fn amm_activation_successful() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().execute_call().unwrap();

        ActivateAmmFixture::new().execute_call().unwrap();

        let token = Token::token_info_by_id(DEFAULT_TOKEN_ID);
        assert_eq!(
            IssuanceState::of::<Test>(&token),
            IssuanceState::Amm(AmmCurve {
                slope: AMM_CURVE_SLOPE,
                intercept: AMM_CURVE_INTERCEPT,
                provided_supply: 0u32.into(),
            })
        );
    })
}

#[test]
fn amm_activation_ok_with_amm_treasury_account_having_existential_deposit() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().execute_call().unwrap();

        ActivateAmmFixture::new().execute_call().unwrap();

        let amm_treasury_account = Token::amm_treasury_account(DEFAULT_TOKEN_ID);
        assert_eq!(
            Balances::usable_balance(amm_treasury_account),
            ExistentialDeposit::get()
        );
    })
}

#[test]
fn amm_activation_ok_with_event_deposit() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().execute_call().unwrap();

        ActivateAmmFixture::new()
            .with_slope(AMM_CURVE_SLOPE)
            .with_intercept(AMM_CURVE_INTERCEPT)
            .execute_call()
            .unwrap();

        last_event_eq!(RawEvent::AmmActivated(
            DEFAULT_TOKEN_ID,
            DEFAULT_ISSUER_MEMBER_ID,
            AmmCurve {
                slope: AMM_CURVE_SLOPE,
                intercept: AMM_CURVE_INTERCEPT,
                provided_supply: 0u32.into(),
            }
        ));
    })
}

// --------------------- amm_sell -------------------------------

#[test]
fn amm_sell_noop_ok_with_zero_requested_amount() {
    let (user_account_id, user_balance) = (member!(2).1, joy!(5_000_000));
    build_default_test_externalities_with_balances(vec![(user_account_id, user_balance)])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();
            AmmBuyFixture::new().execute_call().unwrap();
            let state_pre = sp_io::storage::root(sp_storage::StateVersion::V1);

            let result = AmmSellFixture::new()
                .with_amount(0u32.into())
                .execute_call();

            let state_post = sp_io::storage::root(sp_storage::StateVersion::V1);
            assert_ok!(result);
            assert_eq!(state_pre, state_post);
        })
}

#[test]
fn amm_sell_fails_with_user_not_having_leaking_funds_from_vesting_schedule() {
    const DURATION: u64 = 2 * DEFAULT_SALE_DURATION;
    build_default_test_externalities_with_balances(vec![
        (FIRST_USER_ACCOUNT_ID, joy!(5_000_000)),
        (SECOND_USER_ACCOUNT_ID, joy!(5_000_000)),
    ])
    .execute_with(|| {
        // ------------ arrange -----------------

        // 1. Create token
        IssueTokenFixture::new().execute_call().unwrap();

        // 2. issue a sale and have first user vested
        InitTokenSaleFixture::new()
            .with_linear_vesting_duration(DURATION)
            .with_blocks_before_cliff(Zero::zero())
            .with_cliff_amount_percentage(Zero::zero())
            .execute_call()
            .unwrap();
        PurchaseTokensOnSaleFixture::new()
            .with_amount(DEFAULT_AMM_BUY_AMOUNT)
            .execute_call()
            .unwrap();
        increase_block_number_by(DEFAULT_SALE_DURATION);
        FinalizeTokenSaleFixture::new().execute_call().unwrap();

        // 3. activate amm and have second user minting some tokens
        ActivateAmmFixture::new().execute_call().unwrap();
        AmmBuyFixture::new()
            .with_sender(SECOND_USER_ACCOUNT_ID)
            .with_member_id(SECOND_USER_MEMBER_ID)
            .execute_call()
            .unwrap();

        // ----------------- act -------------------
        let result = AmmSellFixture::new().execute_call();

        // ---------------- assert -----------------
        // Alice is now being vested but she has 0 transferrable amount
        assert_err!(result, Error::<Test>::InsufficientTokenBalance);
    })
}
#[test]
fn amm_sell_fails_with_user_not_having_enough_token_balance() {
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();
            AmmBuyFixture::new().execute_call().unwrap();

            let result = AmmSellFixture::new()
                .with_amount(2 * DEFAULT_AMM_BUY_AMOUNT)
                .execute_call();

            assert_err!(result, Error::<Test>::InsufficientTokenBalance);
        })
}

#[test]
fn amm_sell_fails_with_invalid_token_id() {
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();
            AmmBuyFixture::new().execute_call().unwrap();

            let result = AmmSellFixture::new()
                .with_token_id(DEFAULT_TOKEN_ID + 1)
                .execute_call();

            assert_err!(result, Error::<Test>::TokenDoesNotExist);
        })
}

#[test]
fn amm_sell_fails_with_invalid_account_info_specified() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().execute_call().unwrap();
        ActivateAmmFixture::new().execute_call().unwrap();

        let result = AmmSellFixture::new()
            .with_sender(SECOND_USER_ACCOUNT_ID + 1)
            .with_member_id(SECOND_USER_MEMBER_ID + 1)
            .execute_call();

        assert_err!(result, Error::<Test>::AccountInformationDoesNotExist);
    })
}

#[test]
fn amm_sell_fails_with_member_and_origin_auth() {
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();
            AmmBuyFixture::new().execute_call().unwrap();

            let result = AmmSellFixture::new()
                .with_sender(SECOND_USER_ACCOUNT_ID)
                .execute_call();

            assert_err!(
                result,
                DispatchError::Other("origin signer not a member controller account")
            );
        })
}

#[test]
fn amm_sell_fails_with_token_not_in_amm_state() {
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();

            let result = AmmSellFixture::new().execute_call();

            assert_err!(result, Error::<Test>::NotInAmmState);
        })
}

#[test]
fn amm_sell_failed_with_slippage_constraint_violated() {
    let slippage_tolerance = (Permill::zero(), joy!(1_000_000_000));
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();
            AmmBuyFixture::new().execute_call().unwrap();

            let result = AmmSellFixture::new()
                .with_slippage_tolerance(Some(slippage_tolerance))
                .execute_call();

            assert_err!(result, Error::<Test>::SlippageToleranceExceeded);
        })
}

#[test]
fn amm_treasury_balance_correctly_decreased_during_amm_sell() {
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();
            AmmBuyFixture::new().execute_call().unwrap();
            let amm_reserve_account = Token::amm_treasury_account(DEFAULT_TOKEN_ID);
            let amm_reserve_pre = Balances::usable_balance(amm_reserve_account);
            let correctly_computed_joy_amount = amm_function_values(
                DEFAULT_AMM_SELL_AMOUNT,
                DEFAULT_TOKEN_ID,
                AmmOperation::Sell,
            );

            AmmSellFixture::new().execute_call().unwrap();

            let amm_reserve_post = Balances::usable_balance(amm_reserve_account);
            assert_eq!(
                amm_reserve_pre - amm_reserve_post,
                correctly_computed_joy_amount
            );
        })
}

#[test]
fn amm_sell_ok_with_crt_issuance_decreased() {
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();
            AmmBuyFixture::new().execute_call().unwrap();
            let supply_pre = Token::token_info_by_id(DEFAULT_TOKEN_ID).total_supply;

            AmmSellFixture::new()
                .with_amount(DEFAULT_AMM_SELL_AMOUNT)
                .execute_call()
                .unwrap();

            let supply_post = Token::token_info_by_id(DEFAULT_TOKEN_ID).total_supply;
            assert_eq!(supply_pre - supply_post, DEFAULT_AMM_SELL_AMOUNT);
        })
}

#[test]
fn amm_sell_fails_with_amm_treasury_not_having_sufficient_usable_joy_required() {
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();
            AmmBuyFixture::new().execute_call().unwrap();
            // setting the balance of teh amm_buy curve reserve to 0
            Balances::set_balance(
                Origin::root(),
                Token::amm_treasury_account(DEFAULT_TOKEN_ID),
                Balance::zero(),
                Balance::zero(),
            )
            .unwrap();

            let result = AmmSellFixture::new().execute_call();

            assert_err!(result, Error::<Test>::InsufficientJoyBalance);
        })
}

#[test]
fn amm_sell_ok_with_user_joy_balance_correctly_increased() {
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new()
                .with_empty_allocation()
                .execute_call()
                .unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();
            AmmBuyFixture::new().execute_call().unwrap();
            let user_reserve_pre = Balances::usable_balance(FIRST_USER_ACCOUNT_ID);
            let correctly_computed_joy_amount = amm_function_values(
                DEFAULT_AMM_SELL_AMOUNT,
                DEFAULT_TOKEN_ID,
                AmmOperation::Sell,
            );

            AmmSellFixture::new().execute_call().unwrap();

            let user_reserve_post = Balances::usable_balance(FIRST_USER_ACCOUNT_ID);
            assert_eq!(
                user_reserve_post - user_reserve_pre,
                correctly_computed_joy_amount
            );
        })
}

#[test]
fn amm_sell_ok_with_user_crt_amount_correctly_decreased() {
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();
            AmmBuyFixture::new().execute_call().unwrap();
            let user_crt_pre =
                Token::account_info_by_token_and_member(DEFAULT_TOKEN_ID, FIRST_USER_MEMBER_ID)
                    .amount;

            AmmSellFixture::new().execute_call().unwrap();

            let user_crt_post =
                Token::account_info_by_token_and_member(DEFAULT_TOKEN_ID, FIRST_USER_MEMBER_ID)
                    .amount;
            assert_eq!(user_crt_pre - user_crt_post, DEFAULT_AMM_SELL_AMOUNT);
        })
}

#[test]
fn amm_sell_ok_with_event_deposited() {
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new()
                .with_empty_allocation()
                .execute_call()
                .unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();
            AmmBuyFixture::new().execute_call().unwrap();
            let price = amm_function_values(
                DEFAULT_AMM_SELL_AMOUNT,
                DEFAULT_TOKEN_ID,
                AmmOperation::Sell,
            );

            AmmSellFixture::new().execute_call().unwrap();

            last_event_eq!(RawEvent::TokensSoldOnAmm(
                DEFAULT_TOKEN_ID,
                FIRST_USER_MEMBER_ID,
                DEFAULT_AMM_SELL_AMOUNT,
                price,
            ));
        })
}

// ------------------- DEACTIVATE ---------------------------------------

#[test]
fn deactivate_fails_with_token_not_in_amm_state() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().execute_call().unwrap();

        let result = DeactivateAmmFixture::new().execute_call();

        assert_err!(result, Error::<Test>::NotInAmmState);
    })
}

#[test]
fn deactivate_fails_with_invalid_token_id() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().execute_call().unwrap();
        ActivateAmmFixture::new().execute_call().unwrap();

        let result = DeactivateAmmFixture::new()
            .with_token_id(DEFAULT_TOKEN_ID + 1)
            .execute_call();

        assert_err!(result, Error::<Test>::TokenDoesNotExist);
    })
}

#[test]
fn deactivate_fails_with_too_much_amm_provided_supply_outstanding() {
    let amount = Permill::from_percent(10).mul_floor(DEFAULT_INITIAL_ISSUANCE);
    build_default_test_externalities_with_balances(vec![(
        FIRST_USER_ACCOUNT_ID,
        joy!(10_000_000_000_000_000_000),
    )])
    .execute_with(|| {
        IssueTokenFixture::new()
            .with_initial_supply(1_000u128)
            .execute_call()
            .unwrap();
        ActivateAmmFixture::new().execute_call().unwrap();
        AmmBuyFixture::new()
            .with_amount(amount)
            .execute_call()
            .unwrap();

        let result = DeactivateAmmFixture::new().execute_call();

        assert_err!(result, Error::<Test>::OutstandingAmmProvidedSupplyTooLarge);
    })
}

#[test]
fn deactivate_ok_with_status_set_to_idle() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().execute_call().unwrap();
        ActivateAmmFixture::new().execute_call().unwrap();

        DeactivateAmmFixture::new().execute_call().unwrap();

        let token = Token::token_info_by_id(DEFAULT_TOKEN_ID);
        assert_eq!(IssuanceState::of::<Test>(&token), IssuanceState::Idle);
    })
}

#[test]
fn deactivate_ok_with_amm_buy_curve_params_set_to_none() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().execute_call().unwrap();
        ActivateAmmFixture::new().execute_call().unwrap();

        DeactivateAmmFixture::new().execute_call().unwrap();

        let token = Token::token_info_by_id(DEFAULT_TOKEN_ID);
        assert!(token.amm_curve.is_none());
    })
}

#[test]
fn deactivate_ok_with_full_cycle_from_activation() {
    build_default_test_externalities_with_balances(vec![(FIRST_USER_ACCOUNT_ID, joy!(5_000_000))])
        .execute_with(|| {
            IssueTokenFixture::new().execute_call().unwrap();
            ActivateAmmFixture::new().execute_call().unwrap();
            AmmBuyFixture::new().execute_call().unwrap();
            AmmSellFixture::new().execute_call().unwrap();

            DeactivateAmmFixture::new().execute_call().unwrap();

            let amm_treasury_account = Token::amm_treasury_account(DEFAULT_TOKEN_ID);
            assert_eq!(
                Balances::usable_balance(amm_treasury_account),
                ExistentialDeposit::get()
            );
        })
}

#[test]
fn amm_deactivation_ok_with_event_deposit() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().execute_call().unwrap();
        ActivateAmmFixture::new().execute_call().unwrap();

        DeactivateAmmFixture::new().execute_call().unwrap();

        last_event_eq!(RawEvent::AmmDeactivated(
            DEFAULT_TOKEN_ID,
            DEFAULT_ISSUER_MEMBER_ID,
            Zero::zero(),
        ));
    })
}
