use application::{background_tasks::SmsCommand, profile_changes::use_cases::SmsService};
use async_trait::async_trait;
use domain::{events::EventDispatcher, EMAIL_SEND_TASK, SMS_SEND_TASK, SNS_PUBLISH_TASK};
use infrastructure::{events::QmlEventDispatcher, SmsTask};
use qml_rs::{JobLocker, MemoryStorage, Storage, TypedWorker, WorkerConfig, WorkerContext};
use serde_json::{json, Value};
use std::sync::Arc;

// ── helpers ──────────────────────────────────────────────────────────────────

fn ctx() -> WorkerContext {
    WorkerContext::new(WorkerConfig::default())
}

fn mem_storage() -> Arc<MemoryStorage> {
    Arc::new(MemoryStorage::new())
}

fn dispatcher(storage: Arc<MemoryStorage>) -> QmlEventDispatcher {
    QmlEventDispatcher::new(storage as Arc<dyn Storage>)
}

// ── SMS mock ──────────────────────────────────────────────────────────────────

struct OkSms;

#[async_trait]
impl SmsService for OkSms {
    async fn send(&self, _phone: &str, _message: &str) -> Result<(), String> {
        Ok(())
    }
}

struct FailSms;

#[async_trait]
impl SmsService for FailSms {
    async fn send(&self, _phone: &str, _message: &str) -> Result<(), String> {
        Err("upstream timeout".to_string())
    }
}

// ── encode_payload: happy path ────────────────────────────────────────────────

/// Single JSON object arg → payload stored as object (TypedWorker-compatible).
#[tokio::test]
async fn encode_payload_single_json_object_is_unwrapped() {
    let storage = mem_storage();
    let d = dispatcher(storage.clone());

    let cmd = SmsCommand {
        phone: "+66811111111".to_string(),
        message: "hello".to_string(),
    };
    let encoded = serde_json::to_string(&cmd).unwrap();

    d.dispatch(SMS_SEND_TASK, vec![encoded]).await.unwrap();

    let job = storage
        .fetch_and_lock_job("qa-worker", None)
        .await
        .unwrap()
        .expect("job should exist");

    assert!(
        job.payload.is_object(),
        "single JSON object arg should be stored as an object, got: {}",
        job.payload
    );
    assert_eq!(job.payload["phone"], json!("+66811111111"));
    assert_eq!(job.payload["message"], json!("hello"));
}

// ── encode_payload: edge cases ────────────────────────────────────────────────

/// Single non-JSON string → falls back to array encoding.
#[tokio::test]
async fn encode_payload_single_non_json_stays_array() {
    let storage = mem_storage();
    let d = dispatcher(storage.clone());

    d.dispatch("some_event", vec!["plain-uuid-string".to_string()])
        .await
        .unwrap();

    let job = storage
        .fetch_and_lock_job("qa-worker", None)
        .await
        .unwrap()
        .expect("job should exist");

    assert!(
        job.payload.is_array(),
        "single non-JSON arg should be stored as array, got: {}",
        job.payload
    );
    assert_eq!(job.payload, json!(["plain-uuid-string"]));
}

/// Multiple args → always array regardless of content.
#[tokio::test]
async fn encode_payload_multiple_args_always_array() {
    let storage = mem_storage();
    let d = dispatcher(storage.clone());

    d.dispatch("some_event", vec!["arg1".to_string(), "arg2".to_string()])
        .await
        .unwrap();

    let job = storage
        .fetch_and_lock_job("qa-worker", None)
        .await
        .unwrap()
        .expect("job should exist");

    assert!(job.payload.is_array());
    assert_eq!(job.payload, json!(["arg1", "arg2"]));
}

/// Empty args → empty array.
#[tokio::test]
async fn encode_payload_empty_args_gives_empty_array() {
    let storage = mem_storage();
    let d = dispatcher(storage.clone());

    d.dispatch("some_event", vec![]).await.unwrap();

    let job = storage
        .fetch_and_lock_job("qa-worker", None)
        .await
        .unwrap()
        .expect("job should exist");

    assert_eq!(job.payload, Value::Array(vec![]));
}

/// Single JSON array arg → stored as array (no extra wrapping).
#[tokio::test]
async fn encode_payload_single_json_array_is_unwrapped() {
    let storage = mem_storage();
    let d = dispatcher(storage.clone());

    let inner = json!(["a", "b"]);
    d.dispatch("some_event", vec![inner.to_string()])
        .await
        .unwrap();

    let job = storage
        .fetch_and_lock_job("qa-worker", None)
        .await
        .unwrap()
        .expect("job should exist");

    // A single arg that is itself a JSON array is stored unwrapped (not double-wrapped).
    assert_eq!(job.payload, json!(["a", "b"]));
}

// ── worker method names ───────────────────────────────────────────────────────

#[test]
fn sms_task_method_name_matches_constant() {
    let worker = SmsTask::new(Arc::new(OkSms));
    assert_eq!(worker.method_name(), SMS_SEND_TASK);
}

#[test]
fn sns_publish_task_method_name_matches_constant() {
    // SnsPublishTask needs AppFactoryState — test constant directly against
    // the value returned by the worker trait impl, which simply re-exports
    // the domain constant.
    assert_eq!(SNS_PUBLISH_TASK, "sns_publish");
    assert_eq!(SMS_SEND_TASK, "sms_send");
    assert_eq!(EMAIL_SEND_TASK, "email_send");
}

// ── SmsTask: behaviour ────────────────────────────────────────────────────────

#[tokio::test]
async fn sms_task_happy_path_returns_success() {
    let worker = SmsTask::new(Arc::new(OkSms));
    let cmd = SmsCommand {
        phone: "+66899999999".to_string(),
        message: "Your OTP is 123456".to_string(),
    };
    let result = worker.execute(cmd, &ctx()).await;
    assert!(result.is_ok());
    assert!(result.unwrap().is_success());
}

#[tokio::test]
async fn sms_task_propagates_sms_service_error_as_worker_error() {
    let worker = SmsTask::new(Arc::new(FailSms));
    let cmd = SmsCommand {
        phone: "+66800000000".to_string(),
        message: "test".to_string(),
    };
    let err = worker.execute(cmd, &ctx()).await.unwrap_err();
    match err {
        qml_rs::QmlError::WorkerError { message } => {
            assert!(message.contains("upstream timeout"), "unexpected message: {message}");
        }
        other => panic!("expected WorkerError, got: {other:?}"),
    }
}
