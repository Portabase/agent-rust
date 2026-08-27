use crate::core::context::Context;
use crate::services::api::ApiClient;
use crate::services::backup::logger::JobLogger;
use crate::services::restore::RestoreService;
use crate::tests::init_tracing_for_test;
use crate::utils::edge_key::EdgeKey;

use std::sync::Arc;
use tempfile::TempDir;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn a_failing_download_is_retried_until_it_succeeds() {
    init_tracing_for_test();
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/backups/archive.tar.gz"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(2)
        .with_priority(1)
        .expect(2)
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/backups/archive.tar.gz"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"portabase-archive".to_vec()))
        .with_priority(2)
        .expect(1)
        .mount(&server)
        .await;

    let ctx = Context {
        edge_key: EdgeKey {
            server_url: server.uri(),
            agent_id: "agent-1".to_string(),
            master_key_b64: String::new(),
        },
        api: ApiClient::new(server.uri()),
    };

    let service = RestoreService::new(Arc::new(ctx));

    let temp_dir = TempDir::new().unwrap();
    let logger = Arc::new(JobLogger::new());
    let url = format!("{}/backups/archive.tar.gz", server.uri());

    let downloaded = service
        .download_backup(&url, temp_dir.path(), Arc::clone(&logger), None)
        .await
        .unwrap();

    assert_eq!(std::fs::read(&downloaded).unwrap(), b"portabase-archive");

    let entries = Arc::try_unwrap(logger).unwrap().into_entries();
    assert_eq!(entries.iter().filter(|e| e.level == "warn").count(), 2);
    assert!(
        entries
            .iter()
            .any(|e| e.message == "Backup download succeeded on attempt 3/3")
    );
}
