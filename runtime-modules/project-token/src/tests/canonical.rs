#![cfg(test)]
use frame_support::{assert_err, assert_noop, assert_ok, StorageDoubleMap, StorageMap};
use sp_runtime::traits::Zero;
use sp_runtime::DispatchError;
use sp_runtime::{testing::H256, Permill};
use test_case::test_case;

use crate::tests::fixtures::*;
use crate::tests::mock::*;
use crate::types::{
    PatronageData, RevenueSplitState, TokenAllocationOf, TokenIssuanceParametersOf, TransferPolicy,
    VestingSource, YearlyRate,
};
use crate::{
    balance, block, last_event_eq, merkle_proof, merkle_root, yearly_rate, Error, RawEvent,
    RepayableBloatBond, TokenDataOf,
};
use frame_support::traits::Currency;
use sp_runtime::traits::Hash;

#[test]
fn join_whitelist_fails_with_token_id_not_valid() {
    let commitment = merkle_root![FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID];
    let proof = merkle_proof!(0, [FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID]);
    build_default_test_externalities_with_balances(vec![
        (FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
        (DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
    ])
    .execute_with(|| {
        IssueTokenFixture::new()
            .with_transfer_policy_params(TransferPolicyParams::Permissioned(WhitelistParams {
                commitment,
                payload: None,
            }))
            .run();

        let result = JoinWhitelistFixture::new()
            .with_token_id(DEFAULT_TOKEN_ID + 1)
            .with_merkle_proof(proof)
            .execute_call();

        assert_err!(result, Error::<Test>::TokenDoesNotExist);
    })
}

#[test]
fn join_whitelist_fails_with_existing_account() {
    build_default_test_externalities_with_balances(vec![
        (FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
        (DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
    ])
    .execute_with(|| {
        IssueTokenFixture::new()
            .with_transfer_policy_params(TransferPolicyParams::Permissioned(WhitelistParams {
                commitment: merkle_root![FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID],
                payload: None,
            }))
            .execute_call()
            .unwrap();
        TransferFixture::new().run();

        let result = JoinWhitelistFixture::new().execute_call();

        assert_noop!(result, Error::<Test>::AccountAlreadyExists,);
    })
}

#[test]
fn join_whitelist_fails_with_invalid_member_controller() {
    build_default_test_externalities_with_balances(vec![
        (FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
        (DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
    ])
    .execute_with(|| {
        IssueTokenFixture::new()
            .with_transfer_policy_params(TransferPolicyParams::Permissioned(WhitelistParams {
                commitment: merkle_root![FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID],
                payload: None,
            }))
            .execute_call()
            .unwrap();

        let result = JoinWhitelistFixture::new()
            .with_sender(SECOND_USER_ACCOUNT_ID)
            .execute_call();

        assert_noop!(
            result,
            DispatchError::Other("origin signer not a member controller account")
        );
    })
}

#[test]
fn join_whitelist_fails_with_invalid_proof() {
    build_default_test_externalities_with_balances(vec![
        (FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
        (DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
    ])
    .execute_with(|| {
        IssueTokenFixture::new()
            .with_transfer_policy_params(TransferPolicyParams::Permissioned(WhitelistParams {
                commitment: merkle_root![FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID],
                payload: None,
            }))
            .execute_call()
            .unwrap();

        let result = JoinWhitelistFixture::new()
            .with_merkle_proof(merkle_proof![0, [SECOND_USER_MEMBER_ID]])
            .execute_call();

        assert_noop!(result, Error::<Test>::MerkleProofVerificationFailure);
    })
}

#[test]
fn join_whitelist_fails_with_insufficent_joy_balance_for_bloat_bond() {
    build_default_test_externalities_with_balances(vec![(
        DEFAULT_ISSUER_ACCOUNT_ID,
        DEFAULT_BLOAT_BOND + ed(),
    )])
    .execute_with(|| {
        IssueTokenFixture::new()
            .with_transfer_policy_params(TransferPolicyParams::Permissioned(WhitelistParams {
                commitment: merkle_root![FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID],
                payload: None,
            }))
            .execute_call()
            .unwrap();

        let result = JoinWhitelistFixture::new()
            .with_merkle_proof(merkle_proof![
                0,
                [FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID]
            ])
            .execute_call();

        assert_noop!(result, Error::<Test>::InsufficientJoyBalance);
    })
}

#[test]
fn join_whitelist_fails_in_permissionless_mode() {
    build_default_test_externalities_with_balances(vec![
        (FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
        (DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
    ])
    .execute_with(|| {
        IssueTokenFixture::new().run();

        let result = JoinWhitelistFixture::new().execute_call();

        assert_noop!(
            result,
            Error::<Test>::CannotJoinWhitelistInPermissionlessMode,
        );
    })
}

#[test]
fn join_whitelist_ok() {
    build_default_test_externalities_with_balances(vec![
        (FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
        (DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
    ])
    .execute_with(|| {
        IssueTokenFixture::new()
            .with_transfer_policy_params(TransferPolicyParams::Permissioned(WhitelistParams {
                commitment: merkle_root![FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID],
                payload: None,
            }))
            .execute_call()
            .unwrap();

        let result = JoinWhitelistFixture::new()
            .with_merkle_proof(merkle_proof![
                0,
                [FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID]
            ])
            .execute_call();

        assert_ok!(result);
    })
}

#[test]
fn join_whitelist_ok_with_bloat_bond_slashed_from_caller() {
    build_default_test_externalities_with_balances(vec![
        (FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
        (DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
    ])
    .execute_with(|| {
        IssueTokenFixture::new()
            .with_transfer_policy_params(TransferPolicyParams::Permissioned(WhitelistParams {
                commitment: merkle_root![FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID],
                payload: None,
            }))
            .execute_call()
            .unwrap();

        JoinWhitelistFixture::new()
            .with_merkle_proof(merkle_proof![
                0,
                [FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID]
            ])
            .execute_call()
            .unwrap();

        assert_eq!(
            Balances::usable_balance(&FIRST_USER_ACCOUNT_ID),
            ExistentialDeposit::get()
        );
    })
}

#[test]
fn join_whitelist_ok_with_bloat_bond_transferred_to_treasury() {
    build_default_test_externalities_with_balances(vec![
        (FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
        (DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
    ])
    .execute_with(|| {
        IssueTokenFixture::new()
            .with_transfer_policy_params(TransferPolicyParams::Permissioned(WhitelistParams {
                commitment: merkle_root![FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID],
                payload: None,
            }))
            .execute_call()
            .unwrap();

        JoinWhitelistFixture::new()
            .with_merkle_proof(merkle_proof![
                0,
                [FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID]
            ])
            .execute_call()
            .unwrap();

        assert_eq!(
            Balances::usable_balance(&Token::module_treasury_account()),
            ExistentialDeposit::get() + DEFAULT_BLOAT_BOND
        );
    })
}

#[test]
fn join_whitelist_ok_with_accounts_number_incremented() {
    build_default_test_externalities_with_balances(vec![
        (FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
        (DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
    ])
    .execute_with(|| {
        IssueTokenFixture::new()
            .with_transfer_policy_params(TransferPolicyParams::Permissioned(WhitelistParams {
                commitment: merkle_root![FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID],
                payload: None,
            }))
            .execute_call()
            .unwrap();
        let token_number_pre = Token::token_info_by_id(DEFAULT_TOKEN_ID).accounts_number;

        JoinWhitelistFixture::new()
            .with_merkle_proof(merkle_proof![
                0,
                [FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID]
            ])
            .execute_call()
            .unwrap();

        assert_eq!(
            Token::token_info_by_id(DEFAULT_TOKEN_ID).accounts_number,
            token_number_pre + 1,
        );
    })
}

#[test]
fn join_whitelist_ok_with_event_deposit() {
    build_default_test_externalities_with_balances(vec![
        (FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
        (DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
    ])
    .execute_with(|| {
        let commitment = merkle_root![FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID];
        IssueTokenFixture::new()
            .with_transfer_policy_params(TransferPolicyParams::Permissioned(WhitelistParams {
                commitment,
                payload: None,
            }))
            .execute_call()
            .unwrap();

        JoinWhitelistFixture::new()
            .with_merkle_proof(merkle_proof![
                0,
                [FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID]
            ])
            .execute_call()
            .unwrap();

        last_event_eq!(RawEvent::MemberJoinedWhitelist(
            DEFAULT_TOKEN_ID,
            FIRST_USER_MEMBER_ID,
            Policy::Permissioned(commitment)
        ));
    })
}

#[test]
fn join_whitelist_ok_with_new_account_correctly_created() {
    build_default_test_externalities_with_balances(vec![
        (FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
        (DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
    ])
    .execute_with(|| {
        IssueTokenFixture::new()
            .with_transfer_policy_params(TransferPolicyParams::Permissioned(WhitelistParams {
                commitment: merkle_root![FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID],
                payload: None,
            }))
            .execute_call()
            .unwrap();

        JoinWhitelistFixture::new()
            .with_merkle_proof(merkle_proof![
                0,
                [FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID]
            ])
            .execute_call()
            .unwrap();

        assert_ok!(
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID),
            AccountData {
                bloat_bond: RepayableBloatBond::new(DEFAULT_BLOAT_BOND, None),
                ..Default::default()
            }
        );
    })
}

#[test]
fn join_whitelist_ok_with_invitation_locked_funds_used_for_bloat_bond() {
    build_default_test_externalities_with_balances(vec![
        (FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
        (DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
    ])
    .execute_with(|| {
        let commitment = merkle_root![FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID];
        IssueTokenFixture::new()
            .with_transfer_policy_params(TransferPolicyParams::Permissioned(WhitelistParams {
                commitment,
                payload: None,
            }))
            .execute_call()
            .unwrap();
        set_invitation_lock(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND);

        JoinWhitelistFixture::new()
            .with_merkle_proof(merkle_proof![
                0,
                [FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID]
            ])
            .execute_call()
            .unwrap();

        assert_eq!(
            Balances::usable_balance(Token::module_treasury_account()),
            2 * DEFAULT_BLOAT_BOND + ed(), // issuer + first member + treasury account ed
        );
        assert_eq!(
            System::account(FIRST_USER_ACCOUNT_ID).data,
            balances::AccountData {
                free: ed(),
                reserved: 0,
                misc_frozen: DEFAULT_BLOAT_BOND,
                fee_frozen: 0
            }
        );
        assert_eq!(
            Token::account_info_by_token_and_member(DEFAULT_TOKEN_ID, FIRST_USER_MEMBER_ID)
                .bloat_bond,
            RepayableBloatBond::new(DEFAULT_BLOAT_BOND, Some(FIRST_USER_ACCOUNT_ID))
        )
    });
}

#[test]
fn join_whitelist_fails_with_insufficient_locked_funds() {
    build_default_test_externalities_with_balances(vec![
        (FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed() - 1),
        (DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
    ])
    .execute_with(|| {
        IssueTokenFixture::new()
            .with_transfer_policy_params(TransferPolicyParams::Permissioned(WhitelistParams {
                commitment: merkle_root![FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID],
                payload: None,
            }))
            .execute_call()
            .unwrap();
        let proof = merkle_proof![0, [FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID]];
        let whitelist_fixture = JoinWhitelistFixture::new().with_merkle_proof(proof);
        set_invitation_lock(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND);

        assert_err!(
            whitelist_fixture.execute_call(),
            Error::<Test>::InsufficientJoyBalance
        );

        // Increase balance by 1, but lock ED and those funds with another, not-allowed lock
        increase_account_balance(&FIRST_USER_ACCOUNT_ID, 1);
        set_staking_candidate_lock(&FIRST_USER_ACCOUNT_ID, ed() + 1);

        assert_noop!(
            whitelist_fixture.execute_call(),
            Error::<Test>::InsufficientJoyBalance
        );
    });
}

#[test]
fn join_whitelist_fails_with_incompatible_locked_funds() {
    build_default_test_externalities_with_balances(vec![
        (FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed() - 1),
        (DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed()),
    ])
    .execute_with(|| {
        IssueTokenFixture::new()
            .with_transfer_policy_params(TransferPolicyParams::Permissioned(WhitelistParams {
                commitment: merkle_root![FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID],
                payload: None,
            }))
            .execute_call()
            .unwrap();
        set_staking_candidate_lock(&FIRST_USER_ACCOUNT_ID, DEFAULT_BLOAT_BOND + ed());

        let result = JoinWhitelistFixture::new()
            .with_merkle_proof(merkle_proof![
                0,
                [FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID]
            ])
            .execute_call();

        assert_err!(result, Error::<Test>::InsufficientJoyBalance);
    });
}

#[test]
fn dust_account_fails_with_invalid_token_id() {
    build_default_test_externalities().execute_with(|| {
        let result = DustAccountFixture::new().execute_call();

        assert_err!(result, Error::<Test>::TokenDoesNotExist);
    })
}

#[test]
fn dust_account_fails_with_invalid_member_id() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();

        let result = DustAccountFixture::new()
            .with_member_id(FIRST_USER_MEMBER_ID)
            .execute_call();

        assert_err!(result, Error::<Test>::AccountInformationDoesNotExist);
    })
}

#[test]
fn dust_account_fails_with_permissionless_mode_and_non_empty_account() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        TransferFixture::new().run();

        let result = DustAccountFixture::new()
            .with_user(FIRST_USER_ACCOUNT_ID, FIRST_USER_MEMBER_ID)
            .execute_call();

        assert_err!(result, Error::<Test>::AttemptToRemoveNonEmptyAccount);
    })
}

#[test]
fn dust_account_fails_with_permissioned_mode_and_non_owned_account() {
    let commitment = merkle_root![FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID];
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new()
            .with_transfer_policy_params(TransferPolicyParams::Permissioned(WhitelistParams {
                commitment,
                payload: None,
            }))
            .with_initial_allocation(
                vec![
                    (
                        DEFAULT_ISSUER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: DEFAULT_INITIAL_ISSUANCE,
                            vesting_schedule_params: None,
                        },
                    ),
                    (
                        FIRST_USER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: Zero::zero(),
                            vesting_schedule_params: None,
                        },
                    ),
                ]
                .iter()
                .cloned()
                .collect(),
            )
            .run();

        let result = DustAccountFixture::new()
            .with_member_id(FIRST_USER_MEMBER_ID)
            .execute_call();

        assert_err!(
            result,
            Error::<Test>::AttemptToRemoveNonOwnedAccountUnderPermissionedMode
        );
    })
}

#[test]
fn dust_account_ok_with_permissioned_mode_and_empty_owned_account() {
    let commitment = merkle_root![FIRST_USER_MEMBER_ID, SECOND_USER_MEMBER_ID];
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new()
            .with_transfer_policy_params(TransferPolicyParams::Permissioned(WhitelistParams {
                commitment,
                payload: None,
            }))
            .with_initial_allocation(
                vec![
                    (
                        DEFAULT_ISSUER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: DEFAULT_INITIAL_ISSUANCE,
                            vesting_schedule_params: None,
                        },
                    ),
                    (
                        FIRST_USER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: Zero::zero(),
                            vesting_schedule_params: None,
                        },
                    ),
                ]
                .iter()
                .cloned()
                .collect(),
            )
            .run();

        let result = DustAccountFixture::new()
            .with_user(FIRST_USER_ACCOUNT_ID, FIRST_USER_MEMBER_ID)
            .execute_call();

        assert_ok!(result);
    })
}

#[test]
fn dust_account_ok_with_permissionless_mode_and_empty_non_owned_account() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new()
            .with_initial_allocation(
                vec![
                    (
                        DEFAULT_ISSUER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: DEFAULT_INITIAL_ISSUANCE,
                            vesting_schedule_params: None,
                        },
                    ),
                    (
                        FIRST_USER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: Zero::zero(),
                            vesting_schedule_params: None,
                        },
                    ),
                ]
                .iter()
                .cloned()
                .collect(),
            )
            .run();

        let result = DustAccountFixture::new()
            .with_member_id(FIRST_USER_MEMBER_ID)
            .execute_call();

        assert_ok!(result);
    })
}

#[test]
fn dust_account_ok_with_event_deposit() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new()
            .with_initial_allocation(
                vec![
                    (
                        DEFAULT_ISSUER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: DEFAULT_INITIAL_ISSUANCE,
                            vesting_schedule_params: None,
                        },
                    ),
                    (
                        FIRST_USER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: Zero::zero(),
                            vesting_schedule_params: None,
                        },
                    ),
                ]
                .iter()
                .cloned()
                .collect(),
            )
            .run();

        DustAccountFixture::new()
            .with_member_id(FIRST_USER_MEMBER_ID)
            .run();

        last_event_eq!(RawEvent::AccountDustedBy(
            DEFAULT_TOKEN_ID,
            FIRST_USER_MEMBER_ID,
            DEFAULT_ISSUER_ACCOUNT_ID,
            Policy::Permissionless
        ));
    })
}

#[test]
fn dust_account_ok_accounts_number_decremented() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new()
            .with_initial_allocation(
                vec![
                    (
                        DEFAULT_ISSUER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: DEFAULT_INITIAL_ISSUANCE,
                            vesting_schedule_params: None,
                        },
                    ),
                    (
                        FIRST_USER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: Zero::zero(),
                            vesting_schedule_params: None,
                        },
                    ),
                ]
                .iter()
                .cloned()
                .collect(),
            )
            .run();

        DustAccountFixture::new()
            .with_member_id(FIRST_USER_MEMBER_ID)
            .run();

        assert_eq!(
            Token::token_info_by_id(DEFAULT_TOKEN_ID).accounts_number,
            1u64
        )
    })
}

#[test]
fn dust_account_ok_with_account_removed() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new()
            .with_initial_allocation(
                vec![
                    (
                        DEFAULT_ISSUER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: DEFAULT_INITIAL_ISSUANCE,
                            vesting_schedule_params: None,
                        },
                    ),
                    (
                        FIRST_USER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: Zero::zero(),
                            vesting_schedule_params: None,
                        },
                    ),
                ]
                .iter()
                .cloned()
                .collect(),
            )
            .run();

        DustAccountFixture::new()
            .with_member_id(FIRST_USER_MEMBER_ID)
            .run();

        assert!(!<crate::AccountInfoByTokenAndMember<Test>>::contains_key(
            DEFAULT_TOKEN_ID,
            FIRST_USER_MEMBER_ID
        ));
    })
}

#[test]
fn dust_account_ok_by_user_with_bloat_bond_refunded_to_controller() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new()
            .with_initial_allocation(
                vec![
                    (
                        DEFAULT_ISSUER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: DEFAULT_INITIAL_ISSUANCE,
                            vesting_schedule_params: None,
                        },
                    ),
                    (
                        FIRST_USER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: Zero::zero(),
                            vesting_schedule_params: None,
                        },
                    ),
                ]
                .iter()
                .cloned()
                .collect(),
            )
            .run();

        DustAccountFixture::new()
            .with_member_id(FIRST_USER_MEMBER_ID)
            .run();

        assert_eq!(
            Balances::usable_balance(FIRST_USER_ACCOUNT_ID),
            DEFAULT_BLOAT_BOND
        );
    })
}

// #[test]
// fn dust_account_ok_by_user_with_restricted_bloat_bond_refunded() {
//     let (token_id, init_supply) = (token!(1), balance!(100));
//     let treasury = Token::module_treasury_account();
//     let ((owner_id, _), (user_id, user_acc), (other_user_id, _), restricted_to) =
//         (member!(1), member!(2), member!(3), account!(1004));
//     let (bloat_bond, updated_bloat_bond) = (joy!(100), joy!(150));

//     let token_data = TokenDataBuilder::new_empty().build();

//     let config = GenesisConfigBuilder::new_empty()
//         .with_token_and_owner(token_id, token_data, owner_id, init_supply)
//         .with_bloat_bond(updated_bloat_bond)
//         .with_account(user_id, ConfigAccountData::new())
//         .with_account(
//             other_user_id,
//             ConfigAccountData::new_with_amount_and_bond(
//                 balance!(0),
//                 RepayableBloatBond::new(bloat_bond, Some(restricted_to)),
//             ),
//         )
//         .build();

//     build_test_externalities(config).execute_with(|| {
//         increase_account_balance(&treasury, bloat_bond);

//         let _ = Token::dust_account(origin!(user_acc), token_id, other_user_id);

//         assert_eq!(Balances::usable_balance(restricted_to), bloat_bond);
//     })
// }

#[test]
fn dust_account_ok_with_unregistered_member_doing_the_dusting() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new()
            .with_initial_allocation(
                vec![
                    (
                        DEFAULT_ISSUER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: DEFAULT_INITIAL_ISSUANCE,
                            vesting_schedule_params: None,
                        },
                    ),
                    (
                        FIRST_USER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: Zero::zero(),
                            vesting_schedule_params: None,
                        },
                    ),
                ]
                .iter()
                .cloned()
                .collect(),
            )
            .run();

        let result = DustAccountFixture::new()
            .with_sender(SECOND_USER_ACCOUNT_ID)
            .with_member_id(FIRST_USER_MEMBER_ID)
            .execute_call();

        assert_ok!(result);
    })
}

#[test]
fn dust_account_ok_with_bloat_bond_slashed_from_treasury() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new()
            .with_initial_allocation(
                vec![
                    (
                        DEFAULT_ISSUER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: DEFAULT_INITIAL_ISSUANCE,
                            vesting_schedule_params: None,
                        },
                    ),
                    (
                        FIRST_USER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: Zero::zero(),
                            vesting_schedule_params: None,
                        },
                    ),
                ]
                .iter()
                .cloned()
                .collect(),
            )
            .run();

        DustAccountFixture::new()
            .with_sender(SECOND_USER_ACCOUNT_ID)
            .with_member_id(FIRST_USER_MEMBER_ID)
            .run();

        assert_eq!(
            Balances::usable_balance(&Token::module_treasury_account()),
            DEFAULT_BLOAT_BOND + ed()
        );
    })
}

#[test]
fn deissue_token_fails_with_non_existing_token_id() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();

        let result = DeissueTokenFixture::new()
            .with_token_id(DEFAULT_TOKEN_ID + 1)
            .execute_call();

        assert_err!(result, Error::<Test>::TokenDoesNotExist);
    })
}

#[test]
fn deissue_token_fails_with_existing_accounts() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();

        let result = DeissueTokenFixture::new().execute_call();

        assert_err!(
            result,
            Error::<Test>::CannotDeissueTokenWithOutstandingAccounts
        );
    })
}

#[test]
fn deissue_token_ok() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().with_empty_allocation().run();

        let result = DeissueTokenFixture::new().execute_call();

        assert_ok!(result);
    })
}

#[test]
fn deissue_token_with_event_deposit() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().with_empty_allocation().run();

        DeissueTokenFixture::new().run();

        last_event_eq!(RawEvent::TokenDeissued(DEFAULT_TOKEN_ID));
    })
}

#[test]
fn deissue_token_with_symbol_removed() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().with_empty_allocation().run();

        DeissueTokenFixture::new().run();

        assert!(!<crate::SymbolsUsed<Test>>::contains_key(H256::zero()));
    })
}

#[test]
fn deissue_token_with_token_info_removed() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().with_empty_allocation().run();

        DeissueTokenFixture::new().run();

        assert!(!<crate::TokenInfoById<Test>>::contains_key(
            &DEFAULT_TOKEN_ID
        ));
    })
}

#[test]
fn issue_token_fails_with_existing_symbol() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().with_symbol(HashOut::zero()).run();

        let result = IssueTokenFixture::new()
            .with_symbol(HashOut::zero())
            .execute_call();

        assert_err!(result, Error::<Test>::TokenSymbolAlreadyInUse);
    })
}

#[test]
fn issue_token_fails_with_insufficient_balance_for_bloat_bond() {
    build_default_test_externalities().execute_with(|| {
        let _ = Balances::slash(&DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND);
        IssueTokenFixture::new().with_symbol(HashOut::zero()).run();

        let result = IssueTokenFixture::new()
            .with_symbol(HashOut::zero())
            .execute_call();

        assert_err!(result, Error::<Test>::InsufficientJoyBalance);
    })
}

#[test]
fn issue_token_ok_with_bloat_bond_transferred() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().with_symbol(HashOut::zero()).run();

        IssueTokenFixture::new().with_symbol(HashOut::zero()).run();

        assert_eq!(
            Balances::usable_balance(Token::module_treasury_account()),
            DEFAULT_BLOAT_BOND + ed()
        );
        assert_eq!(
            Balances::usable_balance(DEFAULT_ISSUER_ACCOUNT_ID),
            ExistentialDeposit::get()
        );
    })
}

#[test]
fn issue_token_ok_owner_having_already_issued_a_token() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        increase_account_balance(&DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND);

        IssueTokenFixture::new()
            .with_symbol(Hashing::hash_of(b"Other"))
            .execute_call()
            .unwrap();
    })
}

#[test]
fn issue_token_ok_with_token_id_increased() {
    build_default_test_externalities().execute_with(|| {
        let token_id = Token::next_token_id();
        IssueTokenFixture::new().run();

        assert_eq!(Token::next_token_id(), token_id + 1);
    })
}

#[test]
fn issue_token_ok() {
    build_default_test_externalities().execute_with(|| {
        let result = IssueTokenFixture::new().execute_call();

        assert_ok!(result);
    })
}

#[test]
fn issue_token_ok_with_event_deposit() {
    build_default_test_externalities().execute_with(|| {
        let params = TokenIssuanceParametersOf::<Test> {
            symbol: H256::zero(),
            transfer_policy: TransferPolicyParams::Permissionless,
            patronage_rate: yearly_rate!(0),
            revenue_split_rate: DEFAULT_SPLIT_RATE,
            initial_allocation: issuer_allocation(DEFAULT_INITIAL_ISSUANCE),
        };
        IssueTokenFixture::new().run();

        last_event_eq!(RawEvent::TokenIssued(DEFAULT_TOKEN_ID, params.clone()));
    })
}

#[test]
fn issue_token_ok_with_token_info_added() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();

        assert_eq!(
            <crate::TokenInfoById<Test>>::get(DEFAULT_TOKEN_ID),
            TokenDataOf::<Test> {
                tokens_issued: DEFAULT_INITIAL_ISSUANCE,
                total_supply: DEFAULT_INITIAL_ISSUANCE,
                transfer_policy: TransferPolicy::Permissionless,
                symbol: H256::zero(),
                accounts_number: 1u64, // owner account
                patronage_info: PatronageData::<Balance, BlockNumber> {
                    last_unclaimed_patronage_tally_block: System::block_number(),
                    unclaimed_patronage_tally_amount: balance!(0),
                    rate: yearly_rate!(0),
                },
                sale: None,
                next_sale_id: 0,
                next_revenue_split_id: 0,
                revenue_split: RevenueSplitState::Inactive,
                revenue_split_rate: DEFAULT_SPLIT_RATE,
                amm_curve: None,
            }
        );
    })
}

#[test]
fn issue_token_fails_with_zero_split_rate() {
    build_default_test_externalities().execute_with(|| {
        let result = IssueTokenFixture::new()
            .with_revenue_split_rate(Zero::zero())
            .execute_call();

        assert_noop!(result, Error::<Test>::RevenueSplitRateIsZero,);
    })
}

// TODO: move the following to integration tests
#[test]
fn issue_token_fails_with_non_existing_initial_allocation_member() {
    const INVALID_MEMBER_ID: u64 = 9999;
    build_default_test_externalities().execute_with(|| {
        let fixture = IssueTokenFixture::new().with_initial_allocation(
            vec![
                (
                    DEFAULT_ISSUER_MEMBER_ID,
                    TokenAllocationOf::<Test> {
                        amount: DEFAULT_INITIAL_ISSUANCE,
                        vesting_schedule_params: None,
                    },
                ),
                (
                    INVALID_MEMBER_ID,
                    TokenAllocationOf::<Test> {
                        amount: balance!(100),
                        vesting_schedule_params: None,
                    },
                ),
            ]
            .iter()
            .cloned()
            .collect(),
        );

        let result = fixture.execute_call();

        assert_err!(result, Error::<Test>::InitialAllocationToNonExistingMember);
    })
}

#[test]
fn issue_token_ok_with_symbol_added() {
    let symbol = Hashing::hash_of(b"test");
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new()
            .with_symbol(symbol)
            .execute_call()
            .unwrap();

        assert!(<crate::SymbolsUsed<Test>>::contains_key(symbol));
    })
}

#[test]
fn issue_token_ok_with_accounts_data_added() {
    let vesting_params = VestingScheduleParams {
        blocks_before_cliff: block!(100),
        cliff_amount_percentage: Permill::from_percent(50),
        linear_vesting_duration: block!(100),
    };

    build_default_test_externalities().execute_with(|| {
        let fixture = IssueTokenFixture::new().with_initial_allocation(
            vec![
                (
                    DEFAULT_ISSUER_MEMBER_ID,
                    TokenAllocationOf::<Test> {
                        amount: DEFAULT_INITIAL_ISSUANCE,
                        vesting_schedule_params: None,
                    },
                ),
                (
                    FIRST_USER_MEMBER_ID,
                    TokenAllocationOf::<Test> {
                        amount: DEFAULT_USER_BALANCE,
                        vesting_schedule_params: Some(vesting_params.clone()),
                    },
                ),
                (
                    SECOND_USER_MEMBER_ID,
                    TokenAllocationOf::<Test> {
                        amount: DEFAULT_USER_BALANCE,
                        vesting_schedule_params: Some(vesting_params.clone()),
                    },
                ),
            ]
            .iter()
            .cloned()
            .collect(),
        );

        fixture.run();

        assert_ok!(
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &DEFAULT_ISSUER_MEMBER_ID),
            AccountData::new_with_amount_and_bond(
                DEFAULT_INITIAL_ISSUANCE,
                RepayableBloatBond::new(DEFAULT_BLOAT_BOND, None)
            )
        );
        assert_ok!(
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID),
            AccountData::new_with_vesting_and_bond::<Test>(
                VestingSource::InitialIssuance,
                VestingSchedule::from_params(
                    System::block_number(),
                    DEFAULT_USER_BALANCE,
                    vesting_params.clone()
                ),
                RepayableBloatBond::new(DEFAULT_BLOAT_BOND, None)
            )
            .unwrap()
        );
        assert_ok!(
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &SECOND_USER_MEMBER_ID),
            AccountData::new_with_vesting_and_bond::<Test>(
                VestingSource::InitialIssuance,
                VestingSchedule::from_params(
                    System::block_number(),
                    DEFAULT_USER_BALANCE,
                    vesting_params
                ),
                RepayableBloatBond::new(DEFAULT_BLOAT_BOND, None)
            )
            .unwrap()
        );
    })
}

#[test_case(ed(), (None,None,None); "just_ed")]
#[test_case(ed() + 1 , (Some(DEFAULT_ISSUER_ACCOUNT_ID),None,None); "more_than_ed")]
#[test_case(ed() + DEFAULT_BLOAT_BOND , (Some(DEFAULT_ISSUER_ACCOUNT_ID),None,None); "ed_and_bloat_bond")]
#[test_case(ed() + DEFAULT_BLOAT_BOND + 1, (Some(DEFAULT_ISSUER_ACCOUNT_ID), Some(DEFAULT_ISSUER_ACCOUNT_ID),None); "more_than_ed_and_bloat_bond")]
#[test_case(ed() + DEFAULT_BLOAT_BOND * 2, (Some(DEFAULT_ISSUER_ACCOUNT_ID), Some(DEFAULT_ISSUER_ACCOUNT_ID),None); "ed_and_twice_bloat_bond")]
#[test_case(ed() + DEFAULT_BLOAT_BOND * 2 + 1, (Some(DEFAULT_ISSUER_ACCOUNT_ID), Some(DEFAULT_ISSUER_ACCOUNT_ID),Some(DEFAULT_ISSUER_ACCOUNT_ID)); "more_than_ed_and_twice_bloat_bond")]
#[test_case(ed() + DEFAULT_BLOAT_BOND * 3, (Some(DEFAULT_ISSUER_ACCOUNT_ID), Some(DEFAULT_ISSUER_ACCOUNT_ID),Some(DEFAULT_ISSUER_ACCOUNT_ID)); "ed_and_trice_bloat_bond")]
fn issue_token_ok_with_invitation_locked_funds(
    locked_balance: JoyBalance,
    expected_bloat_bond_restricted_to: (Option<AccountId>, Option<AccountId>, Option<AccountId>),
) {
    build_default_test_externalities().execute_with(|| {
        increase_account_balance(&DEFAULT_ISSUER_ACCOUNT_ID, 2 * DEFAULT_BLOAT_BOND);
        set_invitation_lock(&DEFAULT_ISSUER_ACCOUNT_ID, locked_balance);
        let fixture = IssueTokenFixture::new().with_initial_allocation(
            vec![
                (
                    DEFAULT_ISSUER_MEMBER_ID,
                    TokenAllocationOf::<Test> {
                        amount: DEFAULT_INITIAL_ISSUANCE,
                        vesting_schedule_params: None,
                    },
                ),
                (
                    FIRST_USER_MEMBER_ID,
                    TokenAllocationOf::<Test> {
                        amount: Zero::zero(),
                        vesting_schedule_params: None,
                    },
                ),
                (
                    SECOND_USER_MEMBER_ID,
                    TokenAllocationOf::<Test> {
                        amount: Zero::zero(),
                        vesting_schedule_params: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        );

        fixture.run();

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
            Token::account_info_by_token_and_member(DEFAULT_TOKEN_ID, DEFAULT_ISSUER_MEMBER_ID)
                .bloat_bond,
            RepayableBloatBond::new(DEFAULT_BLOAT_BOND, expected_bloat_bond_restricted_to.0)
        );
        assert_eq!(
            Token::account_info_by_token_and_member(DEFAULT_TOKEN_ID, FIRST_USER_MEMBER_ID)
                .bloat_bond,
            RepayableBloatBond::new(DEFAULT_BLOAT_BOND, expected_bloat_bond_restricted_to.1)
        );
        assert_eq!(
            Token::account_info_by_token_and_member(DEFAULT_TOKEN_ID, SECOND_USER_MEMBER_ID)
                .bloat_bond,
            RepayableBloatBond::new(DEFAULT_BLOAT_BOND, expected_bloat_bond_restricted_to.2)
        );
    })
}

#[test]
fn issue_token_fails_with_insufficient_locked_funds() {
    let issuer_joy_balance = 3 * DEFAULT_BLOAT_BOND + ed(); // for adding 3 users
    let issuer_locked_balance_for_invitation = issuer_joy_balance - 1;
    build_default_test_externalities_with_balances(vec![(
        DEFAULT_ISSUER_ACCOUNT_ID,
        issuer_locked_balance_for_invitation,
    )])
    .execute_with(|| {
        Balances::make_free_balance_be(
            &DEFAULT_ISSUER_ACCOUNT_ID,
            issuer_locked_balance_for_invitation,
        );
        let fixture = IssueTokenFixture::new().with_initial_allocation(
            vec![
                (
                    DEFAULT_ISSUER_MEMBER_ID,
                    TokenAllocationOf::<Test> {
                        amount: DEFAULT_INITIAL_ISSUANCE,
                        vesting_schedule_params: None,
                    },
                ),
                (
                    FIRST_USER_MEMBER_ID,
                    TokenAllocationOf::<Test> {
                        amount: Zero::zero(),
                        vesting_schedule_params: None,
                    },
                ),
                (
                    SECOND_USER_MEMBER_ID,
                    TokenAllocationOf::<Test> {
                        amount: Zero::zero(),
                        vesting_schedule_params: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        );
        set_invitation_lock(
            &DEFAULT_ISSUER_ACCOUNT_ID,
            issuer_locked_balance_for_invitation,
        );

        let result_after_first_lock = fixture.execute_call();

        assert_err!(
            result_after_first_lock,
            Error::<Test>::InsufficientJoyBalance
        );

        increase_account_balance(&DEFAULT_ISSUER_ACCOUNT_ID, 1);
        set_staking_candidate_lock(&DEFAULT_ISSUER_ACCOUNT_ID, ed() + 1);

        let result_after_second_lock = fixture.execute_call();

        assert_err!(
            result_after_second_lock,
            Error::<Test>::InsufficientJoyBalance
        );
    })
}

#[test]
fn issue_token_fails_with_incompatible_locked_funds() {
    let issuer_joy_balance = 3 * DEFAULT_BLOAT_BOND + ed(); // for adding 3 user
    build_default_test_externalities().execute_with(|| {
        Balances::make_free_balance_be(&DEFAULT_ISSUER_ACCOUNT_ID, issuer_joy_balance);
        let fixture = IssueTokenFixture::new().with_initial_allocation(
            vec![
                (
                    DEFAULT_ISSUER_MEMBER_ID,
                    TokenAllocationOf::<Test> {
                        amount: DEFAULT_INITIAL_ISSUANCE,
                        vesting_schedule_params: None,
                    },
                ),
                (
                    FIRST_USER_MEMBER_ID,
                    TokenAllocationOf::<Test> {
                        amount: Zero::zero(),
                        vesting_schedule_params: None,
                    },
                ),
                (
                    SECOND_USER_MEMBER_ID,
                    TokenAllocationOf::<Test> {
                        amount: Zero::zero(),
                        vesting_schedule_params: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        );
        set_staking_candidate_lock(&DEFAULT_ISSUER_ACCOUNT_ID, issuer_joy_balance);

        let result = fixture.execute_call();

        assert_err!(result, Error::<Test>::InsufficientJoyBalance);
    });
}

#[test]
fn burn_fails_with_invalid_token_id() {
    build_default_test_externalities().execute_with(|| {
        let result = BurnFixture::new().execute_call();

        assert_err!(result, Error::<Test>::TokenDoesNotExist);
    })
}

#[test]
fn burn_fails_with_non_existing_account() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();

        let result = BurnFixture::new().execute_call();

        assert_err!(result, Error::<Test>::AccountInformationDoesNotExist);
    })
}

#[test]
fn burn_fails_with_invalid_member_controller_account() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();

        let result = BurnFixture::new()
            .with_sender(FIRST_USER_ACCOUNT_ID)
            .execute_call();

        assert_err!(
            result,
            DispatchError::Other("origin signer not a member controller account")
        );
    })
}

#[test]
fn burn_fails_with_zero_amount() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();

        let result = BurnFixture::new().with_amount(Zero::zero()).execute_call();

        assert_err!(result, Error::<Test>::BurnAmountIsZero);
    })
}

#[test]
fn burn_fails_with_amount_exceeding_account_tokens() {
    build_default_test_externalities().execute_with(|| {
        increase_account_balance(&DEFAULT_ISSUER_ACCOUNT_ID, DEFAULT_BLOAT_BOND);
        IssueTokenFixture::new().run();
        TransferFixture::new().run();

        let result = BurnFixture::new()
            .with_user(FIRST_USER_ACCOUNT_ID, FIRST_USER_MEMBER_ID)
            .with_amount(DEFAULT_USER_BALANCE + 1)
            .execute_call();

        assert_err!(
            result,
            Error::<Test>::BurnAmountGreaterThanAccountTokensAmount
        );
    })
}

#[test]
fn burn_fails_with_active_revenue_split() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        TransferFixture::new()
            .with_output(
                FIRST_USER_MEMBER_ID,
                DEFAULT_SPLIT_PARTICIPATION + DEFAULT_USER_BALANCE,
            )
            .execute_call()
            .unwrap();
        IssueRevenueSplitFixture::new().run();
        increase_block_number_by(MIN_REVENUE_SPLIT_TIME_TO_START);
        ParticipateInSplitFixture::new().run();

        // Burn staked tokens partially
        let result = BurnFixture::new()
            .with_user(FIRST_USER_ACCOUNT_ID, FIRST_USER_MEMBER_ID)
            .with_amount(DEFAULT_USER_BURN_AMOUNT)
            .execute_call();

        assert_err!(
            result,
            Error::<Test>::CannotModifySupplyWhenRevenueSplitsAreActive
        );
    })
}

#[test]
fn burn_ok() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        TransferFixture::new().run();

        let result = BurnFixture::new()
            .with_user(FIRST_USER_ACCOUNT_ID, FIRST_USER_MEMBER_ID)
            .execute_call();

        assert_ok!(result);
    })
}

#[test]
fn burn_ok_with_account_tokens_amount_decreased() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        TransferFixture::new().run();

        BurnFixture::new()
            .with_user(FIRST_USER_ACCOUNT_ID, FIRST_USER_MEMBER_ID)
            .with_amount(DEFAULT_USER_BALANCE)
            .execute_call()
            .unwrap();

        assert_eq!(
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID)
                .unwrap()
                .amount,
            0
        );
    })
}

#[test]
fn burn_ok_with_token_supply_decreased() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();

        BurnFixture::new()
            .with_amount(DEFAULT_INITIAL_ISSUANCE)
            .execute_call()
            .unwrap();

        let token_data = Token::ensure_token_exists(DEFAULT_TOKEN_ID).unwrap();
        assert_eq!(token_data.tokens_issued, DEFAULT_INITIAL_ISSUANCE);
        assert_eq!(token_data.total_supply, 0);
    })
}

#[test]
fn burn_ok_with_event_emitted() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();

        BurnFixture::new()
            .with_user(FIRST_USER_ACCOUNT_ID, FIRST_USER_MEMBER_ID)
            .with_amount(DEFAULT_USER_BURN_AMOUNT)
            .execute_call()
            .unwrap();

        last_event_eq!(RawEvent::TokensBurned(
            DEFAULT_TOKEN_ID,
            FIRST_USER_MEMBER_ID,
            DEFAULT_USER_BURN_AMOUNT
        ));
    })
}

#[test]
fn burn_ok_with_staked_tokens_partially_burned() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        TransferFixture::new()
            .with_output(
                FIRST_USER_MEMBER_ID,
                DEFAULT_SPLIT_PARTICIPATION + DEFAULT_USER_BALANCE,
            )
            .run();
        IssueRevenueSplitFixture::new().run();
        increase_block_number_by(MIN_REVENUE_SPLIT_TIME_TO_START);
        ParticipateInSplitFixture::new().run();
        increase_block_number_by(DEFAULT_SPLIT_DURATION);
        FinalizeRevenueSplitFixture::new().run();

        // Burn staked tokens partially
        BurnFixture::new()
            .with_user(FIRST_USER_ACCOUNT_ID, FIRST_USER_MEMBER_ID)
            .with_amount(DEFAULT_SPLIT_PARTICIPATION / 2)
            .run();

        let account_data =
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID).unwrap();
        assert_eq!(account_data.staked(), DEFAULT_SPLIT_PARTICIPATION / 2);
        assert_eq!(
            account_data.transferrable::<Test>(System::block_number()),
            DEFAULT_USER_BALANCE
        );
    })
}

#[test]
fn burn_ok_with_burned_token_greater_than_staked_amount() {
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new().run();
        TransferFixture::new()
            .with_output(
                FIRST_USER_MEMBER_ID,
                DEFAULT_SPLIT_PARTICIPATION + DEFAULT_USER_BALANCE,
            )
            .run();
        IssueRevenueSplitFixture::new().run();
        increase_block_number_by(MIN_REVENUE_SPLIT_TIME_TO_START);
        ParticipateInSplitFixture::new().run();
        increase_block_number_by(DEFAULT_SPLIT_DURATION);
        FinalizeRevenueSplitFixture::new().run();

        // Burn staked tokens partially
        BurnFixture::new()
            .with_user(FIRST_USER_ACCOUNT_ID, FIRST_USER_MEMBER_ID)
            .with_amount(DEFAULT_SPLIT_PARTICIPATION + DEFAULT_USER_BURN_AMOUNT)
            .run();

        let account_data =
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID).unwrap();
        assert!(account_data.staked().is_zero());
        assert!(account_data.split_staking_status.is_some());
        assert_eq!(
            account_data.transferrable::<Test>(1),
            DEFAULT_USER_BALANCE - DEFAULT_USER_BURN_AMOUNT
        );
    })
}

#[test]
fn burn_ok_with_vesting_and_staked_tokens_burned_first() {
    let vesting_params = VestingScheduleParams {
        blocks_before_cliff: block!(0), // start vesting immediately
        cliff_amount_percentage: Permill::from_percent(50),
        linear_vesting_duration: block!(100),
    };

    build_default_test_externalities_with_balances(vec![(
        DEFAULT_ISSUER_ACCOUNT_ID,
        2 * DEFAULT_BLOAT_BOND + ed() + DEFAULT_SPLIT_REVENUE,
    )])
    .execute_with(|| {
        IssueTokenFixture::new()
            .with_initial_allocation(
                vec![
                    (
                        DEFAULT_ISSUER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: DEFAULT_INITIAL_ISSUANCE,
                            vesting_schedule_params: None,
                        },
                    ),
                    (
                        FIRST_USER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: 2 * DEFAULT_USER_BALANCE,
                            vesting_schedule_params: Some(vesting_params.clone()),
                        },
                    ),
                ]
                .iter()
                .cloned()
                .collect(),
            )
            .with_revenue_split_rate(Permill::from_percent(100))
            .run();
        IssueRevenueSplitFixture::new().run();
        increase_block_number_by(MIN_REVENUE_SPLIT_TIME_TO_START);
        ParticipateInSplitFixture::new()
            .with_member_id(FIRST_USER_MEMBER_ID)
            .with_amount(DEFAULT_USER_BALANCE)
            .execute_call()
            .unwrap();
        increase_block_number_by(DEFAULT_SPLIT_DURATION);
        FinalizeRevenueSplitFixture::new().run();

        BurnFixture::new()
            .with_user(FIRST_USER_ACCOUNT_ID, FIRST_USER_MEMBER_ID)
            .with_amount(DEFAULT_USER_BALANCE)
            .run();

        let acc_data =
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID).unwrap();
        assert_eq!(acc_data.amount, DEFAULT_USER_BALANCE);
        assert_eq!(
            acc_data.transferrable::<Test>(System::block_number()),
            DEFAULT_USER_BALANCE
        );
        assert_eq!(acc_data.vesting_schedules.len(), 0);
        assert_eq!(acc_data.staked(), 0);
    })
}

// #[test]
// fn burn_ok_with_vesting_and_staked_tokens_partially_burned() {
//     let vesting_schedule = default_vesting_schedule();
//     let account_data = ConfigAccountData::new()
//         .with_max_vesting_schedules(vesting_schedule)
//         .with_staked(2000);
//     let initial_account_amount = account_data.amount;
//     let initial_vesting_schedules = account_data.vesting_schedules.len();
//     let (token_id, burn_amount, (member_id, account)) = (token!(1), balance!(1400), member!(1));
//     let token_data = TokenDataBuilder::new_empty().build();

//     let config = GenesisConfigBuilder::new_empty()
//         .with_token(token_id, token_data)
//         .with_account(member_id, account_data)
//         .build();

//     build_test_externalities(config).execute_with(|| {
//         let result = Token::burn(origin!(account), token_id, member_id, burn_amount);

//         assert_ok!(result);
//         let acc_data = Token::ensure_account_data_exists(token_id, &member_id).unwrap();
//         assert_eq!(acc_data.amount, initial_account_amount - burn_amount);
//         assert_eq!(acc_data.staked(), 600);
//         assert_eq!(acc_data.transferrable::<Test>(1), 0);
//         assert_eq!(
//             acc_data.vesting_schedules.len(),
//             initial_vesting_schedules - 1
//         );
//         let first_vesting_schedule = acc_data.vesting_schedules.iter().next().unwrap().1;
//         assert_eq!(first_vesting_schedule.burned_amount, 400);
//         assert_eq!(first_vesting_schedule.locks::<Test>(1), 600);
//         assert_eq!(first_vesting_schedule.non_burned_amount(), 600);
//     })
// }

#[test]
fn burn_ok_with_vesting_schedule_partially_burned_twice() {
    let vesting_params = VestingScheduleParams {
        blocks_before_cliff: block!(0), // start vesting immediately
        cliff_amount_percentage: Permill::from_percent(50),
        linear_vesting_duration: block!(100),
    };
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new()
            .with_initial_allocation(
                vec![
                    (
                        DEFAULT_ISSUER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: DEFAULT_INITIAL_ISSUANCE,
                            vesting_schedule_params: None,
                        },
                    ),
                    (
                        FIRST_USER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: 8 * DEFAULT_USER_BURN_AMOUNT,
                            vesting_schedule_params: Some(vesting_params.clone()),
                        },
                    ),
                ]
                .iter()
                .cloned()
                .collect(),
            )
            .run();

        BurnFixture::new()
            .with_user(FIRST_USER_ACCOUNT_ID, FIRST_USER_MEMBER_ID)
            .with_amount(DEFAULT_USER_BURN_AMOUNT)
            .run();
        BurnFixture::new()
            .with_user(FIRST_USER_ACCOUNT_ID, FIRST_USER_MEMBER_ID)
            .with_amount(2 * DEFAULT_USER_BURN_AMOUNT)
            .run();

        let acc_data =
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID).unwrap();
        assert_eq!(acc_data.amount, 5 * DEFAULT_USER_BURN_AMOUNT);
        assert_eq!(acc_data.vesting_schedules.len(), 1);
        let first_vesting_schedule = acc_data.vesting_schedules.iter().next().unwrap().1;
        assert_eq!(
            first_vesting_schedule.burned_amount,
            3 * DEFAULT_USER_BURN_AMOUNT
        );
        assert_eq!(
            first_vesting_schedule.locks::<Test>(1),
            DEFAULT_USER_BURN_AMOUNT
        );
        assert_eq!(
            first_vesting_schedule.non_burned_amount(),
            5 * DEFAULT_USER_BURN_AMOUNT
        );
    })
}

#[test]
fn burn_ok_with_partially_burned_vesting_schedule_amounts_working_as_expected() {
    let cliff_blocks = block!(10);
    let vesting_duration = block!(100);
    let half_vesting_duration = block!(50);
    let vesting_params = VestingScheduleParams {
        blocks_before_cliff: cliff_blocks,
        cliff_amount_percentage: Permill::from_percent(50),
        linear_vesting_duration: vesting_duration,
    };
    build_default_test_externalities().execute_with(|| {
        IssueTokenFixture::new()
            .with_initial_allocation(
                vec![
                    (
                        DEFAULT_ISSUER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: DEFAULT_INITIAL_ISSUANCE,
                            vesting_schedule_params: None,
                        },
                    ),
                    (
                        FIRST_USER_MEMBER_ID,
                        TokenAllocationOf::<Test> {
                            amount: 2 * DEFAULT_USER_BALANCE,
                            vesting_schedule_params: Some(vesting_params.clone()),
                        },
                    ),
                ]
                .iter()
                .cloned()
                .collect(),
            )
            .run();
        let now = System::block_number();

        let account_data =
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID).unwrap();

        // Before cliff amount is zero
        assert_eq!(
            account_data.transferrable::<Test>(System::block_number()),
            Zero::zero()
        );

        // token balance right after cliff
        assert_eq!(
            account_data.transferrable::<Test>(now + cliff_blocks + block!(10)),
            DEFAULT_USER_BALANCE + DEFAULT_USER_BALANCE / 10
        );

        // Expect linear increase after 10 blocks
        assert_eq!(
            account_data.transferrable::<Test>(now + cliff_blocks + block!(10)),
            DEFAULT_USER_BALANCE + DEFAULT_USER_BALANCE / 10
        );
        // Expect linear increase after half duration blocks
        assert_eq!(
            account_data.transferrable::<Test>(now + cliff_blocks + half_vesting_duration),
            DEFAULT_USER_BALANCE + DEFAULT_USER_BALANCE / 2
        );

        // - right at the original's vesting end_block
        assert_eq!(
            account_data.transferrable::<Test>(now + cliff_blocks + vesting_duration),
            2 * DEFAULT_USER_BALANCE
        );
        // - after the original's vesting `end_block`
        assert_eq!(
            account_data.transferrable::<Test>(now + cliff_blocks + vesting_duration + 1),
            2 * DEFAULT_USER_BALANCE
        );

        // Go after half vesting duration is passed
        System::set_block_number(now + cliff_blocks + vesting_duration / 2);

        // Burn tokens and re-fetch account_data
        BurnFixture::new()
            .with_amount(DEFAULT_USER_BALANCE / 2)
            .with_user(FIRST_USER_ACCOUNT_ID, FIRST_USER_MEMBER_ID)
            .run();

        let account_data =
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID).unwrap();

        // Expect transferrable balance at current block to still be 400
        assert_eq!(
            account_data.transferrable::<Test>(System::block_number()),
            DEFAULT_USER_BALANCE + DEFAULT_USER_BALANCE / 2
        );
        // Expect transferrable balance after 100 blocks to be 450 (1 token / block rate preserved)
        assert_eq!(
            account_data.transferrable::<Test>(System::block_number() + half_vesting_duration / 2),
            DEFAULT_USER_BALANCE + DEFAULT_USER_BALANCE / 2
        );

        // Expect transferrable balance to be 500:
        // after 100 blocks
        assert_eq!(
            account_data.transferrable::<Test>(System::block_number() + half_vesting_duration / 10),
            DEFAULT_USER_BALANCE + DEFAULT_USER_BALANCE / 2
        );
        // right at the original vesting's `end_block`
        assert_eq!(
            account_data.transferrable::<Test>(now + cliff_blocks + vesting_duration),
            2 * DEFAULT_USER_BALANCE - DEFAULT_USER_BALANCE / 2
        );
        // after the original vesting's `end_block`
        assert_eq!(
            account_data.transferrable::<Test>(now + cliff_blocks + vesting_duration + 1),
            2 * DEFAULT_USER_BALANCE - DEFAULT_USER_BALANCE / 2
        );

        // Burn tokens and re-fetch account_data
        BurnFixture::new()
            .with_user(FIRST_USER_ACCOUNT_ID, FIRST_USER_MEMBER_ID)
            .with_amount(DEFAULT_USER_BALANCE / 2)
            .run();

        let account_data =
            Token::ensure_account_data_exists(DEFAULT_TOKEN_ID, &FIRST_USER_MEMBER_ID).unwrap();
        // expect vesting schedule to be gone
        assert_eq!(account_data.vesting_schedules.len(), 0);
        // expect transferrable balance at current block to be 300
        assert_eq!(
            account_data.transferrable::<Test>(System::block_number()),
            DEFAULT_USER_BALANCE
        );
    })
}
