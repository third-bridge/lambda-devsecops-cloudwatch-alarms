use std::env;

use aws_lambda_events::event::sns::SnsEvent;
use lambda_runtime::{tracing, Error, LambdaEvent};
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

static RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\d+\.\d+(?:[eE][+-]?\d+)?|\d{2}/\d{2}/\d{2} \d{2}:\d{2}:\d{2}|\d+").unwrap()
});

static HTTP_CLIENT: Lazy<Client> = Lazy::new(Client::new);

// ! Custom structs to workaround trend based alarms without threshold, aws_lambda_events may update it in the future
#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct MyCloudWatchAlarmPayload {
    pub alarm_name: String,
    pub new_state_value: String,
    pub new_state_reason: String,
    pub alarm_arn: String,
    pub alarm_description: String,
    pub trigger: MyCloudWatchAlarmTrigger,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct MyCloudWatchAlarmTrigger {
    pub threshold: Option<f64>,
}

/// Converts an SNS event into a list of Slack payloads (serde_json::Value)
pub(crate) fn sns_event_to_slack_payload_list(sns_event: &SnsEvent) -> Result<Vec<Value>, Error> {
    sns_event.records.iter().map(|record| {
        tracing::debug!("Record JSON: {}", serde_json::to_string(&record)?);
        let payload: MyCloudWatchAlarmPayload = serde_json::from_str(&record.sns.message)?;

        let color = match payload.new_state_value.as_ref() {
            "OK" => "36a64f",
            "ALARM" => "cb2431",
            "INSUFFICIENT_DATA" => "cb2431",
            _ => "cb2431",
        };

        let alarm_url = format!(
            "https://{region}.console.aws.amazon.com/cloudwatch/home?region={region}#alarmsV2:alarm/{alarm_name}",
            region = payload.alarm_arn.split(':').collect::<Vec<&str>>().get(3).unwrap_or(&"eu-west-1"),
            alarm_name = payload.alarm_name
        );
        tracing::debug!("Alarm URL: {}", alarm_url);

        let title = format!(
            "*{}: <{}|{}>*",
            payload.new_state_value, alarm_url, payload.alarm_name
        );

        let slack_payload = json!({
            "text": title,
            "attachments": [{
                "fallback": title,
                "footer": payload.alarm_description,
                "color": color,
                "fields": [{
                    "value": quote_numbers_and_dates(&payload.new_state_reason),
                    "short": false
                }]
            }]
        });

        Ok(slack_payload)
    }).collect()
}

pub(crate) async fn function_handler(event: LambdaEvent<SnsEvent>) -> Result<(), Error> {
    tracing::info!("Lambda version: {}", env!("CARGO_PKG_VERSION"));

    let sns_event = event.payload;
    tracing::debug!("SNS event: {}", serde_json::to_string(&sns_event)?);

    let slack_payloads = sns_event_to_slack_payload_list(&sns_event)?;
    let slack_webhook_url =
        env::var("SLACK_WEBHOOK_URL").expect("SLACK_WEBHOOK_URL environment variable not set");

    for slack_payload in slack_payloads {
        tracing::debug!("Slack payload: {}", serde_json::to_string(&slack_payload)?);

        let res = HTTP_CLIENT
            .post(&slack_webhook_url)
            .json(&slack_payload)
            .send()
            .await?;

        if res.status().is_success() {
            tracing::info!("Slack notification sent successfully.");
        } else {
            tracing::error!("Failed to send Slack notification: {:?}", res.status());
            tracing::error!("Failed to send Slack notification: {:?}", res.text().await?);
        }
    }

    Ok(())
}

/// Quotes numbers and dates in a string for Slack formatting.
fn quote_numbers_and_dates(text: &str) -> String {
    // Match numbers (including decimals) and date/time patterns
    let re = &*RE;

    static FLOAT_RE: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"^\d+\.\d{4,}(?:[eE][+-]?\d+)?$").unwrap());

    re.replace_all(text, |caps: &regex::Captures| {
        let matched = &caps[0];
        // Only keep 4 decimal places for floats
        if FLOAT_RE.is_match(matched) {
            // Parse as f64 and format to 4 decimal places
            match matched.parse::<f64>() {
                Ok(num) => format!("`{:.4}`", num),
                Err(_) => format!("`{}`", matched),
            }
        } else {
            format!("`{}`", matched)
        }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_lambda_events::event::sns::{
        CloudWatchAlarmPayload, SnsEventObj, SnsMessageObj, SnsRecordObj,
    };

    #[tokio::test]
    async fn test_event_handler() {
        // Build a strongly-typed CloudWatchAlarmPayload
        let payload = CloudWatchAlarmPayload {
            alarm_name: "TestAlarm".to_string(),
            alarm_description: "Test alarm description".to_string(),
            aws_account_id: "123456789012".to_string(),
            new_state_value: "ALARM".to_string(),
            new_state_reason: "Threshold Crossed".to_string(),
            state_change_time: "2023-01-01T00:00:00.000+0000".to_string(),
            region: "us-east-1".to_string(),
            ..Default::default()
        };

        // Wrap it in SnsMessageObj
        let sns_message = SnsMessageObj {
            sns_message_type: "Notification".to_string(),
            message_id: "test-message-id".to_string(),
            topic_arn: "arn:aws:sns:us-east-1:123456789012:TestTopic".to_string(),
            signature_version: "1".to_string(),
            signature: "test-signature".to_string(),
            signing_cert_url: "https://sns.us-east-1.amazonaws.com/SimpleNotificationService.pem"
                .to_string(),
            unsubscribe_url: "https://sns.us-east-1.amazonaws.com/?Action=Unsubscribe".to_string(),
            message: payload,
            message_attributes: std::collections::HashMap::new(),
            subject: Some("Test Subject".to_string()),
            timestamp: "2023-01-01T00:00:00.000Z"
                .parse::<chrono::DateTime<chrono::Utc>>()
                .unwrap(),
        };

        // Wrap in SnsRecordObj
        let record = SnsRecordObj {
            event_source: "aws:sns".to_string(),
            event_version: "1.0".to_string(),
            event_subscription_arn:
                "arn:aws:sns:us-east-1:123456789012:TestTopic:1234abcd-12ab-34cd-56ef-1234567890ab"
                    .to_string(),
            sns: sns_message,
        };

        // Wrap in SnsEventObj
        let sns_event = SnsEventObj {
            records: vec![record],
        };

        // Convert SnsEventObj<CloudWatchAlarmPayload> to SnsEvent (with JSON message)
        let sns_event_json = serde_json::to_string(&sns_event).unwrap();
        let sns_event: aws_lambda_events::event::sns::SnsEvent =
            serde_json::from_str(&sns_event_json).expect("Failed to parse SNS event");

        // Test the new function directly
        let slack_payloads = sns_event_to_slack_payload_list(&sns_event).unwrap();
        assert_eq!(slack_payloads.len(), 1);
        assert_eq!(
            slack_payloads[0]["attachments"][0]["fields"][0]["value"],
            "Threshold Crossed"
        );
    }
}
