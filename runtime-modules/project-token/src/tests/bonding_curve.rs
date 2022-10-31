#![cfg(test)]

use crate::tests::fixtures::*;
use crate::tests::mock::*;
use crate::{member, token};
use frame_support::{assert_noop, assert_ok};

// --------------------- BONDING -------------------------------

#[test]
fn bonding_order_noop_ok_with_zero_requested_amount() {}

#[test]
fn bonding_order_fails_with_requested_amount_exceeding_boundaries() {}

#[test]
fn bonding_order_fails_with_past_timestamp() {}

#[test]
fn bonding_order_fails_with_zero_price_point_and_non_zero_requested_amount() {}

#[test]
fn bonding_order_fails_with_price_point_exceeding_boundaries() {}

#[test]
fn bonding_pricing_computation_works_with_zero_request() {}

#[test]
fn bonding_pricing_computation_works_with_max_request_amount() {}

#[test]
fn bonding_pricing_computation_works_with_selected_random_points() {}

// TODO: provide easy to use correct pairs (x, y = F(x)) for testing
#[test]
fn slippage_tolerance_respected_during_bonding() {}

#[test]
fn deadline_time_limit_respected_during_bonding() {}

#[test]
fn tx_fees_correctly_accounted_during_bonding() {}

#[test]
fn crt_issuance_increased_by_amount_during_bonding() {}

#[test]
fn amm_treasury_balance_increased_during_bonding() {}

#[test]
fn bonding_fails_with_user_not_having_sufficient_usable_joy_required() {}

#[test]
fn user_joy_balance_correctly_decreased_during_bonding() {}

#[test]
fn crt_correctly_minted_to_user_during_bonding() {}

// --------------- ACTIVATION ----------------------------------

#[test]
fn amm_activation_fails_with_invalid_member_id() {
    let (user_member_id, user_account_id) = member!(2);
    build_default_test_externalities_with_balances(vec![]).execute_with(|| {
        IssueTokenFixture::default().execute_call().unwrap();
        let result = ActivateAmmFixture::default()
            .with_sender(user_account_id)
            .execute_call();

        assert_noop!(result, Error::<T>::UserNotAuthorizedToActivateAMM)
    })
}

#[test]
fn amm_activation_fails_with_invalid_token_id() {
    let token_id = token!(2);
    build_default_test_externalities_with_balances(vec![]).execute_with(|| {
        IssueTokenFixture::default().execute_call().unwrap();
        let result = ActivateAmmFixture::default()
            .with_token_id(token_id)
            .execute_call();

        assert_noop!(result, Error::<T>::TokenDoesNotExist)
    })
}

#[test]
fn amm_activated_with_status_updated() {}

#[test]
fn activation_fails_when_there_are_ongoing_active_sales() {}

#[test]
fn activation_fails_when_there_are_ongoing_active_revenue_splits() {}

#[test]
fn activation_fails_when_amm_status_already_active() {}

// --------------------- UNBONDING -------------------------------

#[test]
fn unbonding_order_noop_ok_with_zero_requested_amount() {}

#[test]
fn unbonding_order_fails_with_requested_amount_exceeding_boundaries() {}

#[test]
fn unbonding_order_fails_with_past_timestamp() {}

#[test]
fn unbonding_order_fails_with_zero_price_point_and_non_zero_requested_amount() {}

#[test]
fn unbonding_order_fails_with_price_point_exceeding_boundaries() {}

#[test]
fn unbonding_pricing_computation_works_with_zero_request() {}

#[test]
fn unbonding_pricing_computation_works_with_max_request_amount() {}

#[test]
fn unbonding_pricing_computation_works_with_selected_random_points() {}

// TODO: provide easy to use correct pairs (x, y = F(x)) for testing
#[test]
fn slippage_tolerance_respected_during_unbonding() {}

#[test]
fn deadline_time_limit_respected_during_unbonding() {}

#[test]
fn tx_fees_correctly_accounted_during_unbonding() {}

#[test]
fn crt_issuance_decreased_by_amount_during_unbonding() {}

#[test]
fn amm_treasury_balance_decreased_during_unbonding() {}

#[test]
fn unbonding_fails_with_user_not_having_sufficient_usable_crt_required() {}

#[test]
fn user_joy_balance_correctly_increased_during_unbonding() {}

#[test]
fn crt_correctly_burned_to_user_during_unbonding() {}
