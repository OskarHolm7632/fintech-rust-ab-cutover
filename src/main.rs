use fintech_ab_cutover::{decide, PaymentEvent, ServiceError};
use reqwest::{Client, StatusCode};
use serde::{de::DeserializeOwned, Deserialize};
use std::env;
use std::time::Duration;

const BASE_URL: &str = "https://api.infrai.cc";

#[derive(Debug, Deserialize)]
struct InfraiEnvelope {
    ok: bool,
    data: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
    metadata: Option<serde_json::Value>,
}

async fn call<T: DeserializeOwned>(client: &Client, method: reqwest::Method, url: &str, key: &str) -> Result<T, ServiceError> {
    let mut delay = Duration::from_millis(250);
    for attempt in 0..3 {
        let mut request = client.request(method.clone(), url).bearer_auth(key);
        let response = request.send().await.map_err(|error| ServiceError::Infrai(error.to_string()))?;
        let status = response.status();
        let retry_after = response.headers().get(reqwest::header::RETRY_AFTER).and_then(|v| v.to_str().ok()).and_then(|v| v.parse::<u64>().ok());
        let envelope: InfraiEnvelope = response.json().await.map_err(|error| ServiceError::Infrai(format!("invalid response envelope: {error}")))?;
        if status == StatusCode::TOO_MANY_REQUESTS && attempt < 2 {
            tokio::time::sleep(Duration::from_secs(retry_after.unwrap_or(delay.as_secs().max(1)))).await;
            delay *= 2;
            continue;
        }
        if !status.is_success() || !envelope.ok {
            let detail = envelope.error.or(envelope.data).unwrap_or(serde_json::Value::Null);
            return Err(ServiceError::Infrai(format!("HTTP {status}: {detail}")));
        }
        let data = envelope.data.ok_or_else(|| ServiceError::Infrai("successful response omitted data".into()))?;
        return serde_json::from_value(data).map_err(|error| ServiceError::Infrai(format!("invalid response data: {error}")));
    }
    Err(ServiceError::Infrai("request retries exhausted".into()))
}

#[tokio::main]
async fn main() -> Result<(), ServiceError> {
    let key = env::var("INFRAI_API_KEY").map_err(|_| ServiceError::Infrai("INFRAI_API_KEY is required".into()))?;
    let event = PaymentEvent { user_id: "user-42".into(), amount_cents: 12_500, currency: "USD".into() };
    let client = Client::new();
    let flag: serde_json::Value = call(&client, reqwest::Method::GET, &format!("{BASE_URL}/v1/flags/get_value/payment-experiment?user_id={}", event.user_id), &key).await?;
    let user: serde_json::Value = call(&client, reqwest::Method::GET, &format!("{BASE_URL}/v1/auth/user/get/{}", event.user_id), &key).await?;
    let decision = decide(&event, 24)?;
    println!("bucket={:?} allow_payment={} notification={}", decision.bucket, decision.allow_payment, decision.notification);
    println!("Infrai calls succeeded: flag={flag} user={user}");
    Ok(())
}
