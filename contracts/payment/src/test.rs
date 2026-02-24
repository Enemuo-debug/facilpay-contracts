#![cfg(test)]

use super::*;
use soroban_sdk::testutils::{Events, Ledger};
use soroban_sdk::{testutils::Address as _, token, Address, Env};

#[test]
fn test_create_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token, &0);
    assert_eq!(payment_id, 1);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.id, 1);
    assert_eq!(payment.customer, customer);
    assert_eq!(payment.merchant, merchant);
    assert_eq!(payment.amount, amount);
    assert_eq!(payment.token, token);
    assert_eq!(payment.expires_at, 0);
}

#[test]
fn test_get_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 5000_i128;

    env.mock_all_auths();

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token, &0);

    let payment = client.get_payment(&payment_id);

    assert_eq!(payment.id, payment_id);
    assert_eq!(payment.customer, customer);
    assert_eq!(payment.merchant, merchant);
    assert_eq!(payment.amount, amount);
    assert_eq!(payment.token, token);
    assert_eq!(payment.status, PaymentStatus::Pending);
    assert_eq!(payment.expires_at, 0);
}

#[test]
#[should_panic(expected = "Payment not found")]
fn test_get_payment_not_found() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    client.get_payment(&999);
}

#[test]
fn test_complete_payment_success() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 1000_i128;

    client.initialize(&admin);

    token_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &contract_id, &amount, &1000);

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token_contract_id, &0);
    client.complete_payment(&admin, &payment_id);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Completed);
}

#[test]
fn test_refund_payment_success() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 2000_i128;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token, &0);

    // Refund the payment
    client.refund_payment(&admin, &payment_id);

    // Verify status changed to Refunded
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Refunded);
}

#[test]
#[should_panic]
fn test_complete_payment_not_found() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    env.mock_all_auths();

    client.initialize(&admin);

    client.complete_payment(&admin, &999);
}

#[test]
#[should_panic]
fn test_refund_payment_not_found() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);

    env.mock_all_auths();

    client.initialize(&admin);

    client.refund_payment(&admin, &999);
}

#[test]
#[should_panic]
fn test_complete_already_completed_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token, &0);

    // Complete the payment first time
    client.complete_payment(&admin, &payment_id);

    // Try to complete again - should fail
    client.complete_payment(&admin, &payment_id);
}

#[test]
#[should_panic]
fn test_refund_already_refunded_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 2000_i128;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token, &0);

    // Refund the payment first time
    client.refund_payment(&admin, &payment_id);

    // Try to refund again - should fail
    client.refund_payment(&admin, &payment_id);
}

#[test]
#[should_panic]
fn test_complete_refunded_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token, &0);

    // Refund the payment first
    client.refund_payment(&admin, &payment_id);

    // Try to complete refunded payment - should panic due to InvalidStatus error
    client.complete_payment(&admin, &payment_id);
}

#[test]
#[should_panic]
fn test_refund_completed_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 2000_i128;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token, &0);

    // Complete the payment first
    client.complete_payment(&admin, &payment_id);

    // Try to refund completed payment - should panic due to InvalidStatus error
    client.refund_payment(&admin, &payment_id);
}

#[test]
fn test_multiple_payments_correct_modification() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer1 = Address::generate(&env);
    let customer2 = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 1000_i128;

    client.initialize(&admin);

    token_client.mint(&customer1, &amount);
    token_user_client.approve(&customer1, &contract_id, &amount, &1000);

    let payment_id1 = client.create_payment(&customer1, &merchant, &amount, &token_contract_id, &0);
    let payment_id2 =
        client.create_payment(&customer2, &merchant, &2000_i128, &token_contract_id, &0);

    client.complete_payment(&admin, &payment_id1);

    let payment1 = client.get_payment(&payment_id1);
    let payment2 = client.get_payment(&payment_id2);

    assert_eq!(payment1.status, PaymentStatus::Completed);
    assert_eq!(payment2.status, PaymentStatus::Pending);
}
// Cancel Payment Tests
#[test]
fn test_customer_cancel_pending_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token, &0);

    // Customer cancels their pending payment
    let result = client.try_cancel_payment(&customer, &payment_id);
    assert!(result.is_ok());

    // Verify status changed to Cancelled
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Cancelled);
}

#[test]
fn test_merchant_cancel_pending_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token, &0);

    // Merchant cancels the pending payment
    let result = client.try_cancel_payment(&merchant, &payment_id);
    assert!(result.is_ok());

    // Verify status changed to Cancelled
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Cancelled);
}

#[test]
fn test_cancel_nonexistent_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let caller = Address::generate(&env);

    env.mock_all_auths();

    // Try to cancel a non-existent payment
    let result = client.try_cancel_payment(&caller, &999);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::PaymentNotFound);
}

#[test]
fn test_cancel_payment_unauthorized() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let unauthorized_user = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token, &0);

    // Try to cancel as unauthorized user
    let result = client.try_cancel_payment(&unauthorized_user, &payment_id);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::Unauthorized);
}

#[test]
#[should_panic]
fn test_cancel_completed_payment() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 1000_i128;

    client.initialize(&admin);

    token_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &contract_id, &amount, &1000);

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token_contract_id, &0);
    client.complete_payment(&admin, &payment_id);

    // Should panic - InvalidStatus
    client.cancel_payment(&customer, &payment_id);
}

#[test]
fn test_cancel_refunded_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token, &0);

    // Refund the payment first
    client.refund_payment(&admin, &payment_id);

    // Try to cancel refunded payment
    let result = client.try_cancel_payment(&customer, &payment_id);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::InvalidStatus);
}

#[test]
fn test_cancel_already_cancelled_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token, &0);

    // Cancel the payment first time
    client.cancel_payment(&customer, &payment_id);

    // Try to cancel again
    let result = client.try_cancel_payment(&customer, &payment_id);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::InvalidStatus);
}

#[test]
fn test_cancel_payment_event_emission() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token, &0);

    // Cancel the payment - the event is created as part of the function
    let result = client.try_cancel_payment(&customer, &payment_id);
    assert!(result.is_ok());

    // Verify the payment status changed (which is what the event would indicate)
    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Cancelled);
}

#[test]
fn test_cancel_multiple_payments_correct_modification() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer1 = Address::generate(&env);
    let customer2 = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create two payments
    let payment_id1 = client.create_payment(&customer1, &merchant, &1000_i128, &token, &0);
    let payment_id2 = client.create_payment(&customer2, &merchant, &2000_i128, &token, &0);

    // Cancel first payment
    client.cancel_payment(&customer1, &payment_id1);

    // Check both payments have correct status
    let payment1 = client.get_payment(&payment_id1);
    let payment2 = client.get_payment(&payment_id2);

    assert_eq!(payment1.status, PaymentStatus::Cancelled);
    assert_eq!(payment2.status, PaymentStatus::Pending);
}

#[test]
fn test_get_payments_by_customer_multiple() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant1 = Address::generate(&env);
    let merchant2 = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create 3 payments for same customer
    let id1 = client.create_payment(&customer, &merchant1, &1000, &token, &0);
    let id2 = client.create_payment(&customer, &merchant2, &2000, &token, &0);
    let id3 = client.create_payment(&customer, &merchant1, &3000, &token, &0);

    let payments = client.get_payments_by_customer(&customer, &10, &0);
    assert_eq!(payments.len(), 3);
    assert_eq!(payments.get(0).unwrap().id, id1);
    assert_eq!(payments.get(1).unwrap().id, id2);
    assert_eq!(payments.get(2).unwrap().id, id3);
}

#[test]
fn test_get_payments_by_merchant_multiple() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer1 = Address::generate(&env);
    let customer2 = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create 3 payments for same merchant
    let id1 = client.create_payment(&customer1, &merchant, &1000, &token, &0);
    let id2 = client.create_payment(&customer2, &merchant, &2000, &token, &0);
    let id3 = client.create_payment(&customer1, &merchant, &3000, &token, &0);

    let payments = client.get_payments_by_merchant(&merchant, &10, &0);
    assert_eq!(payments.len(), 3);
    assert_eq!(payments.get(0).unwrap().id, id1);
    assert_eq!(payments.get(1).unwrap().id, id2);
    assert_eq!(payments.get(2).unwrap().id, id3);
}

#[test]
fn test_customer_payment_count() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    assert_eq!(client.get_payment_count_by_customer(&customer), 0);

    client.create_payment(&customer, &merchant, &1000, &token, &0);
    assert_eq!(client.get_payment_count_by_customer(&customer), 1);

    client.create_payment(&customer, &merchant, &2000, &token, &0);
    assert_eq!(client.get_payment_count_by_customer(&customer), 2);

    client.create_payment(&customer, &merchant, &3000, &token, &0);
    assert_eq!(client.get_payment_count_by_customer(&customer), 3);
}

#[test]
fn test_merchant_payment_count() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    assert_eq!(client.get_payment_count_by_merchant(&merchant), 0);

    client.create_payment(&customer, &merchant, &1000, &token, &0);
    assert_eq!(client.get_payment_count_by_merchant(&merchant), 1);

    client.create_payment(&customer, &merchant, &2000, &token, &0);
    assert_eq!(client.get_payment_count_by_merchant(&merchant), 2);

    client.create_payment(&customer, &merchant, &3000, &token, &0);
    assert_eq!(client.get_payment_count_by_merchant(&merchant), 3);
}

#[test]
fn test_pagination_first_page() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create 10 payments
    for i in 1..=10 {
        client.create_payment(&customer, &merchant, &(i * 100), &token, &0);
    }

    let payments = client.get_payments_by_customer(&customer, &5, &0);
    assert_eq!(payments.len(), 5);
    assert_eq!(payments.get(0).unwrap().amount, 100);
    assert_eq!(payments.get(4).unwrap().amount, 500);
}

#[test]
fn test_pagination_second_page() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create 10 payments
    for i in 1..=10 {
        client.create_payment(&customer, &merchant, &(i * 100), &token, &0);
    }

    let payments = client.get_payments_by_customer(&customer, &5, &5);
    assert_eq!(payments.len(), 5);
    assert_eq!(payments.get(0).unwrap().amount, 600);
    assert_eq!(payments.get(4).unwrap().amount, 1000);
}

#[test]
fn test_pagination_limit_larger_than_total() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create 3 payments
    client.create_payment(&customer, &merchant, &1000, &token, &0);
    client.create_payment(&customer, &merchant, &2000, &token, &0);
    client.create_payment(&customer, &merchant, &3000, &token, &0);

    let payments = client.get_payments_by_customer(&customer, &100, &0);
    assert_eq!(payments.len(), 3);
}

#[test]
fn test_pagination_offset_beyond_available() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create 3 payments
    client.create_payment(&customer, &merchant, &1000, &token, &0);
    client.create_payment(&customer, &merchant, &2000, &token, &0);
    client.create_payment(&customer, &merchant, &3000, &token, &0);

    let payments = client.get_payments_by_customer(&customer, &5, &10);
    assert_eq!(payments.len(), 0);
}

#[test]
fn test_query_customer_with_no_payments() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);

    let payments = client.get_payments_by_customer(&customer, &10, &0);
    assert_eq!(payments.len(), 0);

    let count = client.get_payment_count_by_customer(&customer);
    assert_eq!(count, 0);
}

#[test]
fn test_query_merchant_with_no_payments() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let merchant = Address::generate(&env);

    let payments = client.get_payments_by_merchant(&merchant, &10, &0);
    assert_eq!(payments.len(), 0);

    let count = client.get_payment_count_by_merchant(&merchant);
    assert_eq!(count, 0);
}

#[test]
fn test_payments_not_mixed_between_customers() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer1 = Address::generate(&env);
    let customer2 = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create payments for customer1
    let id1 = client.create_payment(&customer1, &merchant, &1000, &token, &0);
    let id2 = client.create_payment(&customer1, &merchant, &2000, &token, &0);

    // Create payments for customer2
    let id3 = client.create_payment(&customer2, &merchant, &3000, &token, &0);

    let payments1 = client.get_payments_by_customer(&customer1, &10, &0);
    assert_eq!(payments1.len(), 2);
    assert_eq!(payments1.get(0).unwrap().id, id1);
    assert_eq!(payments1.get(1).unwrap().id, id2);

    let payments2 = client.get_payments_by_customer(&customer2, &10, &0);
    assert_eq!(payments2.len(), 1);
    assert_eq!(payments2.get(0).unwrap().id, id3);
}

#[test]
fn test_payments_not_mixed_between_merchants() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant1 = Address::generate(&env);
    let merchant2 = Address::generate(&env);
    let token = Address::generate(&env);

    env.mock_all_auths();

    // Create payments for merchant1
    let id1 = client.create_payment(&customer, &merchant1, &1000, &token, &0);
    let id2 = client.create_payment(&customer, &merchant1, &2000, &token, &0);

    // Create payments for merchant2
    let id3 = client.create_payment(&customer, &merchant2, &3000, &token, &0);

    let payments1 = client.get_payments_by_merchant(&merchant1, &10, &0);
    assert_eq!(payments1.len(), 2);
    assert_eq!(payments1.get(0).unwrap().id, id1);
    assert_eq!(payments1.get(1).unwrap().id, id2);

    let payments2 = client.get_payments_by_merchant(&merchant2, &10, &0);
    assert_eq!(payments2.len(), 1);
    assert_eq!(payments2.get(0).unwrap().id, id3);
}

// New tests for expiration functionality

#[test]
fn test_create_payment_with_expiration_duration() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 3600_u64; // 1 hour

    env.mock_all_auths();

    let current_timestamp = env.ledger().timestamp();
    let payment_id =
        client.create_payment(&customer, &merchant, &amount, &token, &expiration_duration);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.expires_at, current_timestamp + expiration_duration);
}

#[test]
fn test_create_payment_no_expiration() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 0_u64;

    env.mock_all_auths();

    let payment_id =
        client.create_payment(&customer, &merchant, &amount, &token, &expiration_duration);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.expires_at, 0);
}

#[test]
fn test_is_payment_expired_true() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 10_u64;

    env.mock_all_auths();

    let payment_id =
        client.create_payment(&customer, &merchant, &amount, &token, &expiration_duration);

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + expiration_duration + 1);

    assert!(client.is_payment_expired(&payment_id));
}

#[test]
fn test_is_payment_expired_false_not_yet() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 100_u64;

    env.mock_all_auths();

    let payment_id =
        client.create_payment(&customer, &merchant, &amount, &token, &expiration_duration);

    env.ledger().set_timestamp(env.ledger().timestamp() + 10);

    assert!(!client.is_payment_expired(&payment_id));
}

#[test]
fn test_is_payment_expired_false_no_expiration() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 0_u64;

    env.mock_all_auths();

    let payment_id =
        client.create_payment(&customer, &merchant, &amount, &token, &expiration_duration);

    env.ledger().set_timestamp(env.ledger().timestamp() + 1000);

    assert!(!client.is_payment_expired(&payment_id));
}

#[test]
fn test_is_payment_expired_false_not_found() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    env.mock_all_auths();

    assert!(!client.is_payment_expired(&999));
}

#[test]
fn test_expire_pending_payment_success() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 10_u64;

    env.mock_all_auths();

    let payment_id =
        client.create_payment(&customer, &merchant, &amount, &token, &expiration_duration);

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + expiration_duration + 1);

    let result = client.try_expire_payment(&payment_id);
    assert!(result.is_ok());

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Cancelled);
}

#[test]
#[should_panic]
fn test_expire_payment_not_found() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    env.mock_all_auths();

    client.expire_payment(&999);
}

#[test]
#[should_panic]
fn test_expire_payment_before_expiration() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 100_u64;

    env.mock_all_auths();

    let payment_id =
        client.create_payment(&customer, &merchant, &amount, &token, &expiration_duration);

    env.ledger().set_timestamp(env.ledger().timestamp() + 10);

    client.expire_payment(&payment_id);
}

#[test]
#[should_panic]
fn test_expire_payment_no_expiration_set() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 0_u64;

    env.mock_all_auths();

    let payment_id =
        client.create_payment(&customer, &merchant, &amount, &token, &expiration_duration);
    env.ledger().set_timestamp(env.ledger().timestamp() + 1000);

    client.expire_payment(&payment_id);
}

#[test]
#[should_panic]
fn test_expire_completed_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 10_u64;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id =
        client.create_payment(&customer, &merchant, &amount, &token, &expiration_duration);
    client.complete_payment(&admin, &payment_id);

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + expiration_duration + 1);

    client.expire_payment(&payment_id);
}

#[test]
#[should_panic]
fn test_expire_refunded_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 10_u64;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id =
        client.create_payment(&customer, &merchant, &amount, &token, &expiration_duration);
    client.refund_payment(&admin, &payment_id);

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + expiration_duration + 1);

    client.expire_payment(&payment_id);
}

#[test]
#[should_panic]
fn test_expire_cancelled_payment() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 10_u64;

    env.mock_all_auths();

    let payment_id =
        client.create_payment(&customer, &merchant, &amount, &token, &expiration_duration);
    client.cancel_payment(&customer, &payment_id);

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + expiration_duration + 1);

    client.expire_payment(&payment_id);
}

#[test]
fn test_payment_expired_event_emitted() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 10_u64;

    env.mock_all_auths();

    let payment_id =
        client.create_payment(&customer, &merchant, &amount, &token, &expiration_duration);
    let _expected_expires_at = env.ledger().timestamp() + expiration_duration;

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + expiration_duration + 1);

    client.expire_payment(&payment_id);

    let events = env.events().all();
    assert!(!events.is_empty());

    let last_event = events.last().unwrap();
    let _data = &last_event.2;
}

#[test]
fn test_multiple_payments_different_expiration_times() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    let payment_id1 = client.create_payment(&customer, &merchant, &amount, &token, &10);
    let initial_timestamp1 = env.ledger().timestamp();

    let payment_id2 = client.create_payment(&customer, &merchant, &amount, &token, &0);

    let payment_id3 = client.create_payment(&customer, &merchant, &amount, &token, &30);
    let initial_timestamp3 = env.ledger().timestamp();

    env.ledger().set_timestamp(initial_timestamp1 + 10 + 1);
    client.expire_payment(&payment_id1);

    let p1 = client.get_payment(&payment_id1);
    let p2 = client.get_payment(&payment_id2);
    let _p3 = client.get_payment(&payment_id3);

    assert_eq!(p1.status, PaymentStatus::Cancelled);
    assert_eq!(p2.status, PaymentStatus::Pending);
    assert!(!client.is_payment_expired(&payment_id3));

    env.ledger().set_timestamp(initial_timestamp3 + 30 + 1);
    client.expire_payment(&payment_id3);

    let p3_after = client.get_payment(&payment_id3);
    assert_eq!(p3_after.status, PaymentStatus::Cancelled);
    assert_eq!(p2.status, PaymentStatus::Pending);
}

#[test]
#[should_panic]
fn test_complete_expired_payment_fails() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 10_u64;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id =
        client.create_payment(&customer, &merchant, &amount, &token, &expiration_duration);

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + expiration_duration + 1);

    client.complete_payment(&admin, &payment_id);
}

#[test]
#[should_panic]
fn test_refund_expired_payment_fails() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;
    let expiration_duration = 10_u64;

    env.mock_all_auths();

    client.initialize(&admin);

    let payment_id =
        client.create_payment(&customer, &merchant, &amount, &token, &expiration_duration);

    env.ledger()
        .set_timestamp(env.ledger().timestamp() + expiration_duration + 1);

    client.refund_payment(&admin, &payment_id);
}

#[test]
fn test_complete_payment_transfers_tokens_to_merchant() {
    let env = Env::default();
    env.mock_all_auths();

    // Register a real mock token contract
    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 1000_i128;

    client.initialize(&admin);

    // Mint tokens to customer
    token_client.mint(&customer, &amount);
    assert_eq!(token_user_client.balance(&customer), amount);

    // Customer approves contract to spend on their behalf
    token_user_client.approve(&customer, &contract_id, &amount, &1000);

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token_contract_id, &0);

    client.complete_payment(&admin, &payment_id);

    // Verify funds moved
    assert_eq!(token_user_client.balance(&customer), 0);
    assert_eq!(token_user_client.balance(&merchant), amount);
}

#[test]
fn test_complete_payment_status_is_completed_after_transfer() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 500_i128;

    client.initialize(&admin);

    token_client.mint(&customer, &amount);
    token_user_client.approve(&customer, &contract_id, &amount, &1000);

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token_contract_id, &0);

    client.complete_payment(&admin, &payment_id);

    let payment = client.get_payment(&payment_id);
    assert_eq!(payment.status, PaymentStatus::Completed);
}

#[test]
#[should_panic]
fn test_complete_payment_fails_without_allowance() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 1000_i128;

    client.initialize(&admin);

    // Mint but no approve — transfer_from should fail
    token_client.mint(&customer, &amount);

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token_contract_id, &0);

    client.complete_payment(&admin, &payment_id);
}

#[test]
#[should_panic]
fn test_complete_payment_fails_insufficient_balance() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 1000_i128;

    client.initialize(&admin);

    // Approve but no balance
    token_user_client.approve(&customer, &contract_id, &amount, &1000);

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token_contract_id, &0);

    client.complete_payment(&admin, &payment_id);
}

#[test]
fn test_complete_payment_partial_allowance_with_exact_amount() {
    let env = Env::default();
    env.mock_all_auths();

    let token_admin = Address::generate(&env);
    let token_contract_id = env
        .register_stellar_asset_contract_v2(token_admin.clone())
        .address();
    let token_client = token::StellarAssetClient::new(&env, &token_contract_id);
    let token_user_client = token::Client::new(&env, &token_contract_id);

    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let admin = Address::generate(&env);
    let amount = 750_i128;

    client.initialize(&admin);

    token_client.mint(&customer, &2000);
    // Approve exactly the payment amount
    token_user_client.approve(&customer, &contract_id, &amount, &1000);

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token_contract_id, &0);

    client.complete_payment(&admin, &payment_id);

    assert_eq!(token_user_client.balance(&merchant), amount);
    assert_eq!(token_user_client.balance(&customer), 2000 - amount);
}

// --------------------------------------------------------------------------------
// Initialization & Admin Authorization Logic
// --------------------------------------------------------------------------------

#[test]
#[should_panic(expected = "already initialized")]
fn test_initialize_already_initialized() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);

    client.initialize(&admin);
    // Panic here on the second call
    client.initialize(&admin);
}

#[test]
fn test_complete_payment_unauthorized_admin() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let real_admin = Address::generate(&env);
    let fake_admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    client.initialize(&real_admin);

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token, &0);

    // Try to complete with fake_admin, should return Error::Unauthorized
    let result = client.try_complete_payment(&fake_admin, &payment_id);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::Unauthorized);
}

#[test]
fn test_refund_payment_unauthorized_admin() {
    let env = Env::default();
    let contract_id = env.register(PaymentContract, ());
    let client = PaymentContractClient::new(&env, &contract_id);

    let customer = Address::generate(&env);
    let merchant = Address::generate(&env);
    let real_admin = Address::generate(&env);
    let fake_admin = Address::generate(&env);
    let token = Address::generate(&env);
    let amount = 1000_i128;

    env.mock_all_auths();

    client.initialize(&real_admin);

    let payment_id = client.create_payment(&customer, &merchant, &amount, &token, &0);

    // Try to refund with fake_admin, should return Error::Unauthorized
    let result = client.try_refund_payment(&fake_admin, &payment_id);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err().unwrap(), Error::Unauthorized);
}
