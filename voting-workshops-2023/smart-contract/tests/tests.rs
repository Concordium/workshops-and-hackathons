//! This module contains integration tests for the voting contract.
//!
//! The best way to run these tests are with `cargo concordium test --out concordium-out/module.wasm.v1`
//! as that will make sure to compile the smart contract module before running the tests.

use concordium_smart_contract_testing::*;
use concordium_std::Timestamp;
use voting_contract::*;

/// An account address of all 0s.
const ACC_0: AccountAddress = AccountAddress([0; 32]);
/// An account address of all 1s.
const ACC_1: AccountAddress = AccountAddress([1; 32]);
/// A `Signer` used for signing the transactions while testing.
const SIGNER: Signer = Signer::with_one_key();
/// The unix epoch time in milliseconds for noon at Christmas eve 2023.
const CHRISTMAS_EVE_EPOCH: u64 = 1701873444000;

/// Helper function that sets up a chain, account, and initialized contract.
/// The contract is initialized with:
///  - `end_time` = `CHRISTMAS_EVE_EPOCH`
///  - `options` = ["DK", "DE", "IT"]
fn setup_chain_and_contract(block_time: Timestamp) -> (Chain, ContractInitSuccess) {
    // Setup the test chain struct.
    let mut chain = Chain::new_with_time(block_time);

    // Create two accounts.
    chain.create_account(Account::new(ACC_0, Amount::from_ccd(10000)));
    chain.create_account(Account::new(ACC_1, Amount::from_ccd(10000)));

    // Load the module.
    let module =
        module_load_v1("./concordium-out/module.wasm.v1").expect("Module file should exist");

    // Deploy the module.
    let deployment = chain
        .module_deploy_v1(SIGNER, ACC_0, module)
        .expect("Deploying valid module should succeed");

    // Initialize the contract.
    let initialization = chain
        .contract_init(
            SIGNER,
            ACC_0,
            Energy::from(10000),
            InitContractPayload {
                amount: Amount::zero(),
                mod_ref: deployment.module_reference,
                init_name: OwnedContractName::new_unchecked(String::from("init_voting")),
                param: OwnedParameter::from_serial(&InitParameter {
                    description: String::from("Concordium EuroVision"),
                    options: vec![String::from("DK"), String::from("DE"), String::from("IT")],
                    end_time: Timestamp::from_timestamp_millis(CHRISTMAS_EVE_EPOCH), // Noon on Christmas eve.
                })
                .expect("Valid parameter size"),
            },
        )
        .expect("Initialization should succeed");

    (chain, initialization)
}

/// Test that an account cannot vote if it is past the `end_time` of the election.
#[test]
fn test_vote_after_end_time() {
    // Set up the chain with a block time later than `CHRISTMAS_EVE_EPOCH`, such voting is no longer permitted.
    let (mut chain, initialization) =
        setup_chain_and_contract(Timestamp::from_timestamp_millis(CHRISTMAS_EVE_EPOCH + 1));

    // Try to vote
    let update = chain
        .contract_update(
            SIGNER,
            ACC_0,
            Address::Account(ACC_0),
            Energy::from(10000),
            UpdateContractPayload {
                amount: Amount::zero(),
                address: initialization.contract_address,
                receive_name: OwnedReceiveName::new_unchecked(String::from("voting.vote")),
                message: OwnedParameter::from_serial(&VotingOption::from("DE"))
                    .expect("Parameter has valid length"),
            },
        )
        .expect_err("Vote fails");
    // Parse the returned error.
    let error: VotingError = update
        .parse_return_value()
        .expect("Return value should be a `VotingError`");
    // Check that it failed for the right reason.
    assert_eq!(error, VotingError::VotingFinished);
}

/// Test that voting on an unknown option fails.
#[test]
fn test_vote_on_unknown_option_fails() {
    // Set up the chain with a block time below the end time.
    let (mut chain, initialization) = setup_chain_and_contract(Timestamp::from_timestamp_millis(0));

    // Try to vote on an invalid option.
    let update = chain
        .contract_update(
            SIGNER,
            ACC_0,
            Address::Account(ACC_0),
            Energy::from(10000),
            UpdateContractPayload {
                amount: Amount::zero(),
                address: initialization.contract_address,
                receive_name: OwnedReceiveName::new_unchecked(String::from("voting.vote")),
                message: OwnedParameter::from_serial(&VotingOption::from("IN")) // India is a valid option.
                    .expect("Parameter has valid length"),
            },
        )
        .expect_err("Vote fails");
    // Parse the returned error.
    let error: VotingError = update
        .parse_return_value()
        .expect("Return value should be a `VotingError`");
    // Check that it failed for the right reason.
    assert_eq!(error, VotingError::InvalidVotingOption);
}

/// Test that voting works.
/// - This checks that voting with a valid option is stored correctly,
/// - That you can change your vote,
/// - And that votes by multiple accounts are stored correctly.
///
/// The test works by alternating between using `chain.contract_update` for updating the contract when voting,
/// and using `chain.contract_invoke` when invoking the view function.
#[test]
fn test_valid_voting_with_multiple_accounts() {
    // Set up the chain with a block time below the end time.
    let (mut chain, initialization) = setup_chain_and_contract(Timestamp::from_timestamp_millis(0));

    // ACC_0 votes on Germany.
    chain
        .contract_update(
            SIGNER,
            ACC_0,
            Address::Account(ACC_0), // ACC_0 is the sender.
            Energy::from(10000),
            UpdateContractPayload {
                amount: Amount::zero(),
                address: initialization.contract_address,
                receive_name: OwnedReceiveName::new_unchecked(String::from("voting.vote")),
                message: OwnedParameter::from_serial(&VotingOption::from("DE")) // Voting on Germany.
                    .expect("Parameter has valid length"),
            },
        )
        .expect("Voting succeeds");

    // Use `contract_invoke` to get the `VotingView`.
    let view_0 = chain
        .contract_invoke(
            ACC_0,
            Address::Account(ACC_0),
            Energy::from(10000),
            UpdateContractPayload {
                amount: Amount::zero(),
                address: initialization.contract_address,
                receive_name: OwnedReceiveName::new_unchecked(String::from("voting.view")),
                message: OwnedParameter::empty(),
            },
        )
        .expect("Invoke succeeds.");
    let voting_view_0: VotingView = view_0
        .parse_return_value()
        .expect("Return values should be a `VotingView`");
    // There is only a single entry.
    assert_eq!(voting_view_0.tally.len(), 1);
    // There is one vote on Germany.
    assert_eq!(voting_view_0.tally.get("DE"), Some(&1));

    // ACC_1 votes on Denmark.
    chain
        .contract_update(
            SIGNER,
            ACC_1,
            Address::Account(ACC_1), // ACC_1 is now the sender.
            Energy::from(10000),
            UpdateContractPayload {
                amount: Amount::zero(),
                address: initialization.contract_address,
                receive_name: OwnedReceiveName::new_unchecked(String::from("voting.vote")),
                message: OwnedParameter::from_serial(&VotingOption::from("DK")) // Voting on Denmark.
                    .expect("Parameter has valid length"),
            },
        )
        .expect("Voting succeeds");
    let view_1 = chain
        .contract_invoke(
            ACC_1,
            Address::Account(ACC_1), // The account used here doesn't matter, as it is just an invoke, not an update.
            Energy::from(10000),
            UpdateContractPayload {
                amount: Amount::zero(),
                address: initialization.contract_address,
                receive_name: OwnedReceiveName::new_unchecked(String::from("voting.view")),
                message: OwnedParameter::empty(),
            },
        )
        .expect("Invoke succeeds.");
    let voting_view_1: VotingView = view_1
        .parse_return_value()
        .expect("Return values should be a `VotingView`");
    // There are now two entries.
    assert_eq!(voting_view_1.tally.len(), 2);
    // There is one vote on Germany.
    assert_eq!(voting_view_1.tally.get("DE"), Some(&1));
    // .. And one vote on Denmark.
    assert_eq!(voting_view_1.tally.get("DK"), Some(&1));

    // ACC_0 changes votes to Denmark.
    chain
        .contract_update(
            SIGNER,
            ACC_0,
            Address::Account(ACC_0), // ACC_0 is the sender.
            Energy::from(10000),
            UpdateContractPayload {
                amount: Amount::zero(),
                address: initialization.contract_address,
                receive_name: OwnedReceiveName::new_unchecked(String::from("voting.vote")),
                message: OwnedParameter::from_serial(&VotingOption::from("DK")) // Changing vote to Denmark.
                    .expect("Parameter has valid length"),
            },
        )
        .expect("Voting succeeds");
    let view_2 = chain
        .contract_invoke(
            ACC_1,
            Address::Account(ACC_1), // The account used here doesn't matter, as it is just an invoke, not an update.
            Energy::from(10000),
            UpdateContractPayload {
                amount: Amount::zero(),
                address: initialization.contract_address,
                receive_name: OwnedReceiveName::new_unchecked(String::from("voting.view")),
                message: OwnedParameter::empty(),
            },
        )
        .expect("Invoke succeeds.");
    let voting_view_2: VotingView = view_2
        .parse_return_value()
        .expect("Return values should be a `VotingView`");
    // There is only one entry again.
    assert_eq!(voting_view_2.tally.len(), 1);
    // There are two votes on Denmark.
    assert_eq!(voting_view_2.tally.get("DK"), Some(&2));
}
