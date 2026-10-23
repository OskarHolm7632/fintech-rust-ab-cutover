use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    MissingUser,
    InvalidAmount,
    Infrai(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingUser => write!(f, "user_id is required"),
            Self::InvalidAmount => write!(f, "amount must be positive"),
            Self::Infrai(message) => write!(f, "Infrai request failed: {message}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaymentEvent {
    pub user_id: String,
    pub amount_cents: u64,
    pub currency: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bucket {
    Control,
    Treatment,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub bucket: Bucket,
    pub allow_payment: bool,
    pub notification: String,
}

pub fn decide(event: &PaymentEvent, risk_score: u8) -> Result<Decision, ServiceError> {
    if event.user_id.trim().is_empty() {
        return Err(ServiceError::MissingUser);
    }
    if event.amount_cents == 0 {
        return Err(ServiceError::InvalidAmount);
    }
    let bucket = stable_bucket(&event.user_id);
    let allow_payment = risk_score < 70 && event.amount_cents <= 100_000;
    let notification = match (&bucket, allow_payment) {
        (Bucket::Treatment, true) => "payment approved; treatment receipt queued",
        (Bucket::Control, true) => "payment approved; control receipt queued",
        (_, false) => "payment held for review; audit notification queued",
    };
    Ok(Decision { bucket, allow_payment, notification: notification.to_string() })
}

pub fn stable_bucket(user_id: &str) -> Bucket {
    let hash = user_id.bytes().fold(0u8, |state, byte| state.wrapping_add(byte));
    if hash % 2 == 0 { Bucket::Control } else { Bucket::Treatment }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_user_keeps_assignment_and_risky_payment_is_held() {
        let event = PaymentEvent { user_id: "user-42".into(), amount_cents: 12_500, currency: "USD".into() };
        let first = decide(&event, 72).expect("valid payment");
        let second = decide(&event, 72).expect("valid payment");
        assert_eq!(first.bucket, second.bucket);
        assert!(!first.allow_payment);
        assert_eq!(first.notification, "payment held for review; audit notification queued");
    }
}
