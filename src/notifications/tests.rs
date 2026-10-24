use std::sync::{Arc, Mutex};

use axum::{
    body::Bytes,
    http::{HeaderMap, StatusCode, Uri},
};
use serde::Serialize;

use super::*;
use crate::config::{Notifications, SlackNotifications, WebhookNotifications, WorkerMode};

#[derive(Debug, Clone)]
struct Received {
    path: String,
    headers: HeaderMap,
    body: Vec<u8>,
}

impl Received {
    fn json(&self) -> Value {
        serde_json::from_slice(&self.body).expect("request body is JSON")
    }

    fn header(&self, name: &str) -> Option<String> {
        self.headers
            .get(name)
            .map(|v| v.to_str().unwrap().to_string())
    }
}

type Log = Arc<Mutex<Vec<Received>>>;

/// A local HTTP server that records every request and answers `status`.
async fn mock_server(status: StatusCode) -> (String, Log) {
    let log: Log = Arc::default();
    let recorder = log.clone();
    let app = axum::Router::new().fallback(move |uri: Uri, headers: HeaderMap, body: Bytes| {
        let recorder = recorder.clone();
        async move {
            recorder.lock().unwrap().push(Received {
                path: uri.path().to_string(),
                headers,
                body: body.to_vec(),
            });
            status
        }
    });
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://{addr}"), log)
}

fn requests(log: &Log) -> Vec<Received> {
    log.lock().unwrap().clone()
}

#[derive(Serialize)]
struct QueuePaused {
    by: String,
    reason: String,
    #[serde(skip)]
    to: Vec<Channel>,
}

impl QueuePaused {
    fn to(to: Vec<Channel>) -> Self {
        Self {
            by: "ana".to_string(),
            reason: "deploy freeze".to_string(),
            to,
        }
    }
}

impl Notifier for QueuePaused {
    fn message(&self) -> Message {
        Message::new(format!("Queue paused by {}", self.by))
            .body(&self.reason)
            .field("repo", "acme/web")
            .url("https://example.com/queue?a=1&b=2")
    }

    fn channels(&self, _ctx: &AppContext) -> Vec<Channel> {
        self.to.clone()
    }
}

async fn context(notifications: Option<Notifications>) -> AppContext {
    let mut ctx = crate::tests_cfg::app::get_app_context().await;
    // The test config runs jobs inline, so a delivery completes before
    // `deliver_later` returns and the mock server has seen it.
    assert_eq!(ctx.config.workers.mode, WorkerMode::ForegroundBlocking);
    ctx.config.notifications = notifications;
    ctx
}

#[test]
fn event_defaults_to_snake_case_type_name() {
    assert_eq!(QueuePaused::event(), "queue_paused");
}

#[test]
fn slack_payload_escapes_mrkdwn_control_characters() {
    let payload = slack::payload(&Message::new("<!channel> & co").body("a <b>"));
    assert_eq!(payload["text"], "*&lt;!channel&gt; &amp; co*\na &lt;b&gt;");
}

#[tokio::test]
async fn slack_posts_a_text_message_to_an_explicit_url() {
    let (base, log) = mock_server(StatusCode::OK).await;
    let ctx = context(None).await;

    let report = QueuePaused::to(vec![Channel::slack_url(format!("{base}/hooks/T1"))])
        .deliver_later(&ctx)
        .await;

    assert_eq!(
        report,
        Report {
            accepted: 1,
            skipped: 0,
            failed: 0
        }
    );
    let reqs = requests(&log);
    assert_eq!(reqs.len(), 1);
    assert_eq!(reqs[0].path, "/hooks/T1");
    assert_eq!(
        reqs[0].json(),
        json!({"text": "*Queue paused by ana*\ndeploy freeze\n*repo:* acme/web\n<https://example.com/queue?a=1&b=2|Open>"})
    );
}

#[tokio::test]
async fn slack_default_channel_uses_the_configured_url() {
    let (base, log) = mock_server(StatusCode::OK).await;
    let ctx = context(Some(Notifications {
        slack: Some(SlackNotifications {
            webhook_url: format!("{base}/configured"),
        }),
        ..Notifications::default()
    }))
    .await;

    let report = QueuePaused::to(vec![Channel::slack()])
        .deliver_later(&ctx)
        .await;

    assert_eq!(report.accepted, 1);
    assert_eq!(requests(&log)[0].path, "/configured");
}

#[tokio::test]
async fn unconfigured_channels_are_skipped_not_failed() {
    let ctx = context(Some(Notifications {
        slack: Some(SlackNotifications {
            // What `get_env(name="SLACK_WEBHOOK_URL", default="")` renders to.
            webhook_url: String::new(),
        }),
        ..Notifications::default()
    }))
    .await;

    let report = QueuePaused::to(vec![
        Channel::slack(),
        Channel::webhook(),
        Channel::slack_url(""),
    ])
    .deliver_later(&ctx)
    .await;

    assert_eq!(
        report,
        Report {
            accepted: 0,
            skipped: 3,
            failed: 0
        }
    );
}

#[tokio::test]
async fn webhook_posts_the_event_envelope_unsigned() {
    let (base, log) = mock_server(StatusCode::NO_CONTENT).await;
    let ctx = context(None).await;

    let report = QueuePaused::to(vec![
        Channel::webhook_url(format!("{base}/in")).header("Authorization", "Bearer t0ken")
    ])
    .deliver_later(&ctx)
    .await;

    assert_eq!(report.accepted, 1);
    let req = &requests(&log)[0];
    let body = req.json();
    assert_eq!(body["event"], "queue_paused");
    assert_eq!(body["title"], "Queue paused by ana");
    assert_eq!(body["body"], "deploy freeze");
    assert_eq!(
        body["fields"],
        json!([{"name": "repo", "value": "acme/web"}])
    );
    assert_eq!(
        body["data"],
        json!({"by": "ana", "reason": "deploy freeze"})
    );
    assert_eq!(req.header("x-loco-event").as_deref(), Some("queue_paused"));
    assert_eq!(
        req.header("x-loco-delivery"),
        body["id"].as_str().map(String::from)
    );
    assert_eq!(req.header("authorization").as_deref(), Some("Bearer t0ken"));
    assert_eq!(
        req.header("content-type").as_deref(),
        Some("application/json")
    );
    assert!(req.header("x-loco-timestamp").is_some());
    assert!(req.header("x-loco-signature").is_none());
}

#[tokio::test]
async fn signed_webhook_carries_a_verifiable_signature() {
    let (base, log) = mock_server(StatusCode::OK).await;
    let ctx = context(None).await;

    QueuePaused::to(vec![
        Channel::webhook_url(format!("{base}/in")).signed("whsec")
    ])
    .deliver_later(&ctx)
    .await;

    let req = &requests(&log)[0];
    let timestamp = req.header("x-loco-timestamp").unwrap();
    let signature = req.header("x-loco-signature").unwrap();
    assert!(webhook::verify_signature(
        "whsec", &timestamp, &req.body, &signature
    ));
    assert!(!webhook::verify_signature(
        "wrong", &timestamp, &req.body, &signature
    ));
}

#[tokio::test]
async fn default_webhook_uses_config_secret_and_headers_without_queueing_them() {
    let (base, log) = mock_server(StatusCode::OK).await;
    let ctx = context(Some(Notifications {
        webhook: Some(WebhookNotifications {
            url: format!("{base}/configured"),
            secret: Some("config-secret".to_string()),
            headers: BTreeMap::from([("X-Team".to_string(), "core".to_string())]),
        }),
        ..Notifications::default()
    }))
    .await;
    let notifier = QueuePaused::to(vec![Channel::webhook()]);

    // The job payload names no URL or secret: both are read from config when
    // the job runs, so they never land in the queue table.
    let (deliveries, _) = notifier.deliveries(&ctx).unwrap();
    let job = serde_json::to_string(&deliveries).unwrap();
    assert!(!job.contains("config-secret"), "{job}");
    assert!(!job.contains("/configured"), "{job}");
    assert_eq!(
        serde_json::from_str::<Vec<Delivery>>(&job).unwrap(),
        deliveries
    );

    notifier.deliver_later(&ctx).await;
    let req = &requests(&log)[0];
    assert_eq!(req.path, "/configured");
    assert_eq!(req.header("x-team").as_deref(), Some("core"));
    assert!(webhook::verify_signature(
        "config-secret",
        &req.header("x-loco-timestamp").unwrap(),
        &req.body,
        &req.header("x-loco-signature").unwrap()
    ));
}

#[tokio::test]
async fn explicit_webhook_url_never_gets_the_configured_secret() {
    let (base, log) = mock_server(StatusCode::OK).await;
    let ctx = context(Some(Notifications {
        webhook: Some(WebhookNotifications {
            url: "http://127.0.0.1:1/unused".to_string(),
            secret: Some("config-secret".to_string()),
            headers: BTreeMap::new(),
        }),
        ..Notifications::default()
    }))
    .await;

    QueuePaused::to(vec![Channel::webhook_url(format!("{base}/third-party"))])
        .deliver_later(&ctx)
        .await;

    assert!(requests(&log)[0].header("x-loco-signature").is_none());
}

#[cfg(feature = "mailer")]
#[tokio::test]
async fn email_goes_through_the_app_mailer() {
    let mut ctx = context(Some(Notifications {
        email: Some(crate::config::EmailNotifications {
            to: vec!["ops@example.com".to_string()],
            from: Some("Alerts <alerts@example.com>".to_string()),
        }),
        ..Notifications::default()
    }))
    .await;
    ctx.mailer = Some(crate::mailer::EmailSender::stub());

    let report = QueuePaused::to(vec![
        Channel::email(),
        Channel::email_to("dev@example.com").and_to("lead@example.com"),
    ])
    .deliver_later(&ctx)
    .await;

    assert_eq!(report.accepted, 2);
    let deliveries = ctx.mailer.as_ref().unwrap().deliveries();
    assert_eq!(deliveries.count, 2);
    let configured = &deliveries.messages[0];
    assert!(configured.contains("To: ops@example.com"), "{configured}");
    assert!(
        configured.contains("From: Alerts <alerts@example.com>"),
        "{configured}"
    );
    assert!(
        configured.contains("Subject: Queue paused by ana"),
        "{configured}"
    );
    assert!(configured.contains("repo: acme/web"), "{configured}");
    assert!(deliveries.messages[1].contains("lead@example.com"));
}

#[cfg(feature = "mailer")]
#[tokio::test]
async fn email_without_a_mailer_fails_that_channel_only() {
    let (base, log) = mock_server(StatusCode::OK).await;
    let ctx = context(None).await;
    assert!(ctx.mailer.is_none());

    let report = QueuePaused::to(vec![
        Channel::email_to("ops@example.com"),
        Channel::slack_url(format!("{base}/s")),
    ])
    .deliver_later(&ctx)
    .await;

    assert_eq!((report.accepted, report.failed), (1, 1));
    assert_eq!(requests(&log).len(), 1);
}

#[tokio::test]
async fn a_failing_channel_does_not_fail_the_caller_or_the_other_channels() {
    let (broken, broken_log) = mock_server(StatusCode::INTERNAL_SERVER_ERROR).await;
    let (healthy, healthy_log) = mock_server(StatusCode::OK).await;
    let ctx = context(None).await;

    let report = QueuePaused::to(vec![
        Channel::slack_url(format!("{broken}/s")),
        // Nothing listens on port 1: a connection error, not an HTTP one.
        Channel::webhook_url("http://127.0.0.1:1/down"),
        Channel::webhook_url(format!("{healthy}/w")),
    ])
    .deliver_later(&ctx)
    .await;

    assert_eq!(
        report,
        Report {
            accepted: 1,
            skipped: 0,
            failed: 2
        }
    );
    assert_eq!(requests(&broken_log).len(), 1);
    assert_eq!(requests(&healthy_log).len(), 1);
}

#[tokio::test]
async fn enqueue_failure_is_reported_not_returned() {
    let mut ctx = context(None).await;
    // Queue mode with no queue provider: every enqueue errors.
    ctx.config.workers.mode = WorkerMode::BackgroundQueue;
    assert!(ctx.queue_provider.is_none());

    let report = QueuePaused::to(vec![Channel::slack_url("http://127.0.0.1:1/s")])
        .deliver_later(&ctx)
        .await;

    assert_eq!(report.failed, 1);
}

#[tokio::test]
async fn deliver_now_returns_failures_without_leaking_the_url() {
    let (base, log) = mock_server(StatusCode::OK).await;
    let ctx = context(None).await;

    let err = QueuePaused::to(vec![
        Channel::slack_url("http://127.0.0.1:1/services/SECRET-TOKEN"),
        Channel::slack_url(format!("{base}/ok")),
    ])
    .deliver_now(&ctx)
    .await
    .expect_err("one channel is down");

    let msg = err.to_string();
    assert!(msg.contains("slack"), "{msg}");
    assert!(!msg.contains("SECRET-TOKEN"), "{msg}");
    // The healthy channel was still tried.
    assert_eq!(requests(&log).len(), 1);
}

#[tokio::test]
async fn non_2xx_slack_answer_is_an_error_so_the_queue_retries() {
    let (base, _log) = mock_server(StatusCode::TOO_MANY_REQUESTS).await;
    let ctx = context(None).await;
    let worker = NotificationWorker::build(&ctx);

    let err = worker
        .perform(Delivery {
            id: "d1".to_string(),
            event: "queue_paused".to_string(),
            target: Target::Slack {
                webhook_url: Some(format!("{base}/s")),
                payload: json!({"text": "hi"}),
            },
        })
        .await
        .expect_err("429 must fail the job");
    assert!(err.to_string().contains("429"), "{err}");
}

#[cfg(all(feature = "worker", feature = "db-sqlite"))]
#[tokio::test]
async fn deliveries_go_through_a_real_queue_as_one_job_per_channel() {
    use crate::bgworker::{sqlt, JobStatus};

    let tree = tree_fs::TreeBuilder::default()
        .drop(true)
        .create()
        .expect("temp dir");
    let queue = sqlt::create_provider(&crate::config::SqliteQueueConfig {
        uri: format!(
            "sqlite://{}?mode=rwc",
            tree.root.join("queue.sqlite").display()
        ),
        dangerously_flush: false,
        enable_logging: false,
        max_connections: 1,
        min_connections: 1,
        connect_timeout: 500,
        idle_timeout: 500,
        poll_interval_sec: 1,
        num_workers: 1,
        reaper: None,
    })
    .await
    .expect("sqlite queue");
    queue.setup().await.expect("queue setup");

    let mut ctx = context(Some(Notifications::default())).await;
    ctx.config.workers.mode = WorkerMode::BackgroundQueue;
    // Registers with the configured retry policy, as boot does.
    register_worker(&ctx, &queue).await.expect("register");
    ctx.queue_provider = Some(Arc::new(queue));

    let report = QueuePaused::to(vec![
        Channel::slack_url("https://hooks.slack.invalid/T1"),
        Channel::webhook_url("https://example.invalid/in"),
    ])
    .deliver_later(&ctx)
    .await;
    assert_eq!(report.accepted, 2);

    let jobs = ctx
        .queue_provider
        .as_ref()
        .unwrap()
        .get_jobs(Some(&vec![JobStatus::Queued]), None)
        .await
        .expect("list jobs");
    assert_eq!(jobs.len(), 2);
    for job in jobs {
        assert_eq!(job.name, "NotificationWorker");
        let delivery: Delivery = serde_json::from_value(job.data).expect("job is a Delivery");
        assert_eq!(delivery.event, "queue_paused");
    }
}
