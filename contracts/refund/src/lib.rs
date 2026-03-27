#![no_std]
use soroban_sdk::{
    contract,
    contracterror,
    contractevent,
    contractimpl,
    contracttype,
    Address,
    Env,
    String,
    Vec,
};

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Admin,
    Refund(u64),
    RefundCounter,
    RefundsByStatus(RefundStatus, u64),
    RefundStatusCount(RefundStatus),
    RefundStatusIndex(u64),
    MerchantRefunds(Address, u64),
    MerchantRefundCount(Address),
    CustomerRefunds(Address, u64),
    CustomerRefundCount(Address),
    PaymentRefunds(u64, u64),
    PaymentRefundCount(u64),
    RefundPolicy(Address),
    DefaultRefundPolicy,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum RefundStatus {
    Requested,
    Approved,
    Rejected,
    Processed,
}

#[contracterror]
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    InvalidAmount = 1,
    RefundNotFound = 2,
    Unauthorized = 3,
    InvalidPaymentId = 4,
    TransferFailed = 5,
    NotApproved = 6,
    InvalidStatus = 7,
    AlreadyProcessed = 8,
    RefundExceedsPayment = 9,
    TotalRefundsExceedPayment = 10,
    RefundWindowExpired = 11,
    RefundExceedsPolicy = 12,
    PolicyNotFound = 13,
    PolicyInactive = 14,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundRequested {
    pub refund_id: u64,
    pub payment_id: u64,
    pub merchant: Address,
    pub customer: Address,
    pub amount: i128,
    pub token: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundProcessed {
    pub refund_id: u64,
    pub processed_by: Address,
    pub customer: Address,
    pub amount: i128,
    pub token: Address,
    pub processed_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundApproved {
    pub refund_id: u64,
    pub approved_by: Address,
    pub approved_at: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundRejected {
    pub refund_id: u64,
    pub rejected_by: Address,
    pub rejected_at: u64,
    pub rejection_reason: String,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundPolicySet {
    pub merchant: Address,
    pub refund_window: u64,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefundPolicyDeactivated {
    pub merchant: Address,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PolicyOverrideApplied {
    pub refund_id: u64,
    pub admin: Address,
    pub reason: String,
}

#[contractevent]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoApproved {
    pub refund_id: u64,
    pub amount: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct Refund {
    pub id: u64,
    pub payment_id: u64,
    pub merchant: Address,
    pub customer: Address,
    pub amount: i128,
    pub original_payment_amount: i128,
    pub token: Address,
    pub status: RefundStatus,
    pub requested_at: u64,
    pub reason: String,
}

#[derive(Clone)]
#[contracttype]
pub struct RefundPolicy {
    pub merchant: Address,
    pub refund_window: u64, // seconds from payment creation
    pub max_refund_percentage: u32, // basis points (10000 = 100%)
    pub requires_admin_approval: bool,
    pub auto_approve_below: i128, // auto-approve refunds below this amount
    pub active: bool,
}

#[contract]
pub struct RefundContract;

#[contractimpl]
impl RefundContract {
    pub fn initialize(env: Env, admin: Address) {
        if env.storage().instance().has(&DataKey::Admin) {
            panic!("Already initialized");
        }
        env.storage().instance().set(&DataKey::Admin, &admin);

        // Set default refund policy (30 days, 100% refund, requires approval, no auto-approve)
        let default_policy = RefundPolicy {
            merchant: admin.clone(), // Placeholder, will be overridden per merchant
            refund_window: 30 * 24 * 60 * 60, // 30 days in seconds
            max_refund_percentage: 10000, // 100%
            requires_admin_approval: true,
            auto_approve_below: 0, // No auto-approve by default
            active: true,
        };
        env.storage().instance().set(&DataKey::DefaultRefundPolicy, &default_policy);
    }

    pub fn request_refund(
        env: Env,
        merchant: Address,
        payment_id: u64,
        customer: Address,
        amount: i128,
        original_payment_amount: i128,
        token: Address,
        reason: String,
        payment_created_at: u64
    ) -> Result<u64, Error> {
        // Require merchant authentication
        merchant.require_auth();

        // Validate amount is positive
        if amount <= 0 {
            return Err(Error::InvalidAmount);
        }

        if amount > original_payment_amount {
            return Err(Error::RefundExceedsPayment);
        }

        // Validate payment_id is valid (greater than 0)
        if payment_id == 0 {
            return Err(Error::InvalidPaymentId);
        }

        Self::can_refund_payment(&env, payment_id, amount, original_payment_amount)?;

        // Validate against refund policy
        Self::validate_against_policy(
            &env,
            &merchant,
            amount,
            original_payment_amount,
            payment_created_at
        )?;

        // Get and increment refund counter
        let counter: u64 = env.storage().instance().get(&DataKey::RefundCounter).unwrap_or(0);
        let refund_id = counter + 1;

        // Determine initial status based on policy
        let initial_status = if let Some(policy) = Self::get_refund_policy(&env, merchant.clone()) {
            if !policy.requires_admin_approval && amount <= policy.auto_approve_below {
                RefundStatus::Approved
            } else {
                RefundStatus::Requested
            }
        } else {
            RefundStatus::Requested
        };

        // Create Refund struct
        let refund = Refund {
            id: refund_id,
            payment_id,
            merchant: merchant.clone(),
            customer: customer.clone(),
            amount,
            original_payment_amount,
            token: token.clone(),
            status: initial_status.clone(),
            requested_at: env.ledger().timestamp(),
            reason,
        };

        // Store refund in contract storage
        env.storage().instance().set(&DataKey::Refund(refund_id), &refund);
        env.storage().instance().set(&DataKey::RefundCounter, &refund_id);
        Self::add_to_status_index(&env, initial_status.clone(), refund_id);

        // Index refund by merchant
        let merchant_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::MerchantRefundCount(merchant.clone()))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::MerchantRefunds(merchant.clone(), merchant_count), &refund_id);
        env.storage()
            .instance()
            .set(&DataKey::MerchantRefundCount(merchant.clone()), &(merchant_count + 1));

        // Index refund by customer
        let customer_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CustomerRefundCount(customer.clone()))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::CustomerRefunds(customer.clone(), customer_count), &refund_id);
        env.storage()
            .instance()
            .set(&DataKey::CustomerRefundCount(customer.clone()), &(customer_count + 1));

        // Index refund by payment
        let payment_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::PaymentRefundCount(payment_id))
            .unwrap_or(0);
        env.storage()
            .instance()
            .set(&DataKey::PaymentRefunds(payment_id, payment_count), &refund_id);
        env.storage()
            .instance()
            .set(&DataKey::PaymentRefundCount(payment_id), &(payment_count + 1));

        // Emit RefundRequested event
        (RefundRequested {
            refund_id,
            payment_id,
            merchant,
            customer,
            amount,
            token,
        }).publish(&env);

        // Emit AutoApproved event if applicable
        if initial_status == RefundStatus::Approved {
            (AutoApproved {
                refund_id,
                amount,
            }).publish(&env);
        }

        // Return the new refund ID
        Ok(refund_id)
    }

    pub fn get_refund(env: &Env, refund_id: u64) -> Result<Refund, Error> {
        // Retrieve refund from storage by ID
        env.storage().instance().get(&DataKey::Refund(refund_id)).ok_or(Error::RefundNotFound)
    }

    pub fn approve_refund(env: Env, admin: Address, refund_id: u64) -> Result<(), Error> {
        // Require admin authentication
        admin.require_auth();

        // Retrieve refund from storage
        let mut refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)?;

        // Check refund status is Requested
        if refund.status != RefundStatus::Requested {
            return Err(Error::InvalidStatus);
        }

        Self::remove_from_status_index(&env, RefundStatus::Requested, refund_id)?;

        // Update refund status to Approved
        refund.status = RefundStatus::Approved;

        // Store updated refund back to storage
        env.storage().instance().set(&DataKey::Refund(refund_id), &refund);
        Self::add_to_status_index(&env, RefundStatus::Approved, refund_id);

        // Emit RefundApproved event
        (RefundApproved {
            refund_id,
            approved_by: admin,
            approved_at: env.ledger().timestamp(),
        }).publish(&env);

        Ok(())
    }

    pub fn reject_refund(
        env: Env,
        admin: Address,
        refund_id: u64,
        rejection_reason: String
    ) -> Result<(), Error> {
        // Require admin authentication
        admin.require_auth();

        // Retrieve refund from storage
        let mut refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)?;

        // Check refund status is Requested
        if refund.status != RefundStatus::Requested {
            return Err(Error::InvalidStatus);
        }

        Self::remove_from_status_index(&env, RefundStatus::Requested, refund_id)?;

        // Update refund status to Rejected
        refund.status = RefundStatus::Rejected;

        // Store updated refund back to storage
        env.storage().instance().set(&DataKey::Refund(refund_id), &refund);
        Self::add_to_status_index(&env, RefundStatus::Rejected, refund_id);

        // Emit RefundRejected event
        (RefundRejected {
            refund_id,
            rejected_by: admin,
            rejected_at: env.ledger().timestamp(),
            rejection_reason,
        }).publish(&env);

        Ok(())
    }

    pub fn process_refund(env: Env, admin: Address, refund_id: u64) -> Result<(), Error> {
        admin.require_auth();

        let mut refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)?;

        if refund.status != RefundStatus::Approved {
            return Err(Error::InvalidStatus);
        }

        Self::can_refund_payment(
            &env,
            refund.payment_id,
            refund.amount,
            refund.original_payment_amount
        )?;

        Self::remove_from_status_index(&env, RefundStatus::Approved, refund_id)?;
        refund.status = RefundStatus::Processed;

        env.storage().instance().set(&DataKey::Refund(refund_id), &refund);
        Self::add_to_status_index(&env, RefundStatus::Processed, refund_id);

        Ok(())
    }

    pub fn get_refunds_by_status(
        env: &Env,
        status: RefundStatus,
        limit: u64,
        offset: u64
    ) -> Vec<Refund> {
        let mut results: Vec<Refund> = Vec::new(env);
        let total = Self::get_refund_count_by_status(env, status.clone());

        if limit == 0 || offset >= total {
            return results;
        }

        let end = core::cmp::min(total, offset.saturating_add(limit));
        let mut index = offset;
        while index < end {
            if
                let Some(refund_id) = env
                    .storage()
                    .instance()
                    .get::<_, u64>(&DataKey::RefundsByStatus(status.clone(), index))
            {
                if
                    let Some(refund) = env
                        .storage()
                        .instance()
                        .get::<_, Refund>(&DataKey::Refund(refund_id))
                {
                    results.push_back(refund);
                }
            }
            index += 1;
        }

        results
    }

    pub fn get_refund_count_by_status(env: &Env, status: RefundStatus) -> u64 {
        env.storage().instance().get(&DataKey::RefundStatusCount(status)).unwrap_or(0)
    }

    pub fn get_total_refunded_amount(env: &Env, payment_id: u64) -> i128 {
        let total_refunds: u64 = env.storage().instance().get(&DataKey::RefundCounter).unwrap_or(0);
        let mut total: i128 = 0;

        let mut id: u64 = 1;
        while id <= total_refunds {
            if let Some(refund) = env.storage().instance().get::<_, Refund>(&DataKey::Refund(id)) {
                if refund.payment_id == payment_id && refund.status == RefundStatus::Processed {
                    total += refund.amount;
                }
            }
            id += 1;
        }

        total
    }

    pub fn can_refund_payment(
        env: &Env,
        payment_id: u64,
        requested_amount: i128,
        original_amount: i128
    ) -> Result<bool, Error> {
        let total_refunded = Self::get_total_refunded_amount(env, payment_id);
        if requested_amount.saturating_add(total_refunded) > original_amount {
            return Err(Error::TotalRefundsExceedPayment);
        }

        Ok(true)
    }

    pub fn set_refund_policy(
        env: Env,
        merchant: Address,
        refund_window: u64,
        max_refund_percentage: u32,
        requires_admin_approval: bool,
        auto_approve_below: i128
    ) -> Result<(), Error> {
        // Require merchant authentication
        merchant.require_auth();

        // Validate max_refund_percentage is within bounds (0-10000 basis points)
        if max_refund_percentage > 10000 {
            return Err(Error::RefundExceedsPolicy);
        }

        let policy = RefundPolicy {
            merchant: merchant.clone(),
            refund_window,
            max_refund_percentage,
            requires_admin_approval,
            auto_approve_below,
            active: true,
        };

        env.storage().instance().set(&DataKey::RefundPolicy(merchant.clone()), &policy);

        // Emit RefundPolicySet event
        (RefundPolicySet {
            merchant,
            refund_window,
        }).publish(&env);

        Ok(())
    }

    pub fn get_refund_policy(env: &Env, merchant: Address) -> Option<RefundPolicy> {
        env.storage().instance().get(&DataKey::RefundPolicy(merchant))
    }

    pub fn deactivate_refund_policy(env: Env, merchant: Address) -> Result<(), Error> {
        // Require merchant authentication
        merchant.require_auth();

        let mut policy: RefundPolicy = env
            .storage()
            .instance()
            .get(&DataKey::RefundPolicy(merchant.clone()))
            .ok_or(Error::PolicyNotFound)?;

        if !policy.active {
            return Err(Error::PolicyInactive);
        }

        policy.active = false;
        env.storage().instance().set(&DataKey::RefundPolicy(merchant.clone()), &policy);

        // Emit RefundPolicyDeactivated event
        (RefundPolicyDeactivated { merchant }).publish(&env);

        Ok(())
    }

    pub fn admin_override_policy(
        env: Env,
        admin: Address,
        refund_id: u64,
        reason: String
    ) -> Result<(), Error> {
        // Require admin authentication
        admin.require_auth();

        let admin_address: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;

        if admin != admin_address {
            return Err(Error::Unauthorized);
        }

        // Verify refund exists
        let _refund: Refund = env
            .storage()
            .instance()
            .get(&DataKey::Refund(refund_id))
            .ok_or(Error::RefundNotFound)?;

        // Emit PolicyOverrideApplied event
        (PolicyOverrideApplied {
            refund_id,
            admin,
            reason,
        }).publish(&env);

        Ok(())
    }

    fn validate_against_policy(
        env: &Env,
        merchant: &Address,
        amount: i128,
        original_amount: i128,
        payment_created_at: u64
    ) -> Result<(), Error> {
        // Get merchant-specific policy or default
        let policy: RefundPolicy = if
            let Some(merchant_policy) = Self::get_refund_policy(env, merchant.clone())
        {
            merchant_policy
        } else {
            env.storage()
                .instance()
                .get(&DataKey::DefaultRefundPolicy)
                .unwrap_or_else(|| RefundPolicy {
                    merchant: merchant.clone(),
                    refund_window: 30 * 24 * 60 * 60, // 30 days
                    max_refund_percentage: 10000, // 100%
                    requires_admin_approval: true,
                    auto_approve_below: 0,
                    active: true,
                })
        };

        // Check if policy is active
        if !policy.active {
            return Err(Error::PolicyInactive);
        }

        // Check refund window
        let current_time = env.ledger().timestamp();
        if current_time > payment_created_at.saturating_add(policy.refund_window) {
            return Err(Error::RefundWindowExpired);
        }

        // Check refund percentage using overflow-safe math
        let refund_percentage_bps = amount
            .checked_mul(10000)
            .unwrap_or(i128::MAX)
            .checked_div(original_amount)
            .unwrap_or(u32::MAX as i128) as u32;

        if refund_percentage_bps > policy.max_refund_percentage {
            return Err(Error::RefundExceedsPolicy);
        }

        Ok(())
    }

    fn add_to_status_index(env: &Env, status: RefundStatus, refund_id: u64) {
        let count = Self::get_refund_count_by_status(env, status.clone());
        env.storage().instance().set(&DataKey::RefundsByStatus(status.clone(), count), &refund_id);
        env.storage()
            .instance()
            .set(&DataKey::RefundStatusCount(status.clone()), &(count + 1));
        env.storage().instance().set(&DataKey::RefundStatusIndex(refund_id), &count);
    }

    fn remove_from_status_index(
        env: &Env,
        status: RefundStatus,
        refund_id: u64
    ) -> Result<(), Error> {
        let count = Self::get_refund_count_by_status(env, status.clone());
        if count == 0 {
            return Err(Error::InvalidStatus);
        }

        let index: u64 = env
            .storage()
            .instance()
            .get(&DataKey::RefundStatusIndex(refund_id))
            .ok_or(Error::InvalidStatus)?;
        let last_index = count - 1;

        if index != last_index {
            let last_refund_id: u64 = env
                .storage()
                .instance()
                .get(&DataKey::RefundsByStatus(status.clone(), last_index))
                .ok_or(Error::InvalidStatus)?;
            env.storage()
                .instance()
                .set(&DataKey::RefundsByStatus(status.clone(), index), &last_refund_id);
            env.storage().instance().set(&DataKey::RefundStatusIndex(last_refund_id), &index);
        }

        env.storage().instance().remove(&DataKey::RefundsByStatus(status.clone(), last_index));
        env.storage().instance().remove(&DataKey::RefundStatusIndex(refund_id));
        env.storage().instance().set(&DataKey::RefundStatusCount(status), &last_index);

        Ok(())
    }
}

mod test;
mod test_process;
mod test_policy;
