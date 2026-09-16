//! S3 presign round-trip against a real endpoint (MinIO, LocalStack, AWS).
//!
//! ```sh
//! LOCO_TEST_S3_ENDPOINT=http://127.0.0.1:9000 \
//! LOCO_TEST_S3_BUCKET=loco-presign-test \
//! LOCO_TEST_S3_ACCESS_KEY_ID=minio \
//! LOCO_TEST_S3_SECRET_ACCESS_KEY=minio123 \
//! cargo test -p loco-rs presign_s3_roundtrip --features storage_aws_s3 -- --ignored
//! ```

use std::{path::Path, time::Duration};

use bytes::Bytes;
use opendal::{services::S3, Operator};

use super::opendal_adapter::OpendalAdapter;
use super::{PresignPutOptions, StoreDriver};

fn test_s3_store() -> Option<OpendalAdapter> {
    let endpoint = std::env::var("LOCO_TEST_S3_ENDPOINT").ok()?;
    let bucket = std::env::var("LOCO_TEST_S3_BUCKET").ok()?;
    let key_id = std::env::var("LOCO_TEST_S3_ACCESS_KEY_ID").ok()?;
    let secret_key = std::env::var("LOCO_TEST_S3_SECRET_ACCESS_KEY").ok()?;

    let builder = S3::default()
        .bucket(&bucket)
        .endpoint(&endpoint)
        .region("us-east-1")
        .access_key_id(&key_id)
        .secret_access_key(&secret_key);

    let operator = Operator::new(builder).ok()?;
    Some(OpendalAdapter::new(operator))
}

#[tokio::test]
#[ignore = "needs LOCO_TEST_S3_* env vars; see module docs"]
async fn presign_s3_roundtrip_get_and_put() {
    let store = test_s3_store().expect("set LOCO_TEST_S3_* env vars");
    let path = Path::new("loco-presign-probe.txt");
    let body = Bytes::from("loco presign probe");
    let ttl = Duration::from_secs(300);

    store
        .upload(path, &body)
        .await
        .expect("upload probe object");

    let get = store.presign_get(path, ttl).await.expect("presign get");
    assert_eq!(get.method, http::Method::GET);
    let fetched = reqwest::Client::new()
        .get(get.url())
        .send()
        .await
        .expect("fetch presigned get")
        .text()
        .await
        .expect("read presigned get body");
    assert_eq!(fetched, "loco presign probe");

    let put_path = Path::new("loco-presign-put-probe.txt");
    let put = store
        .presign_put(
            put_path,
            ttl,
            PresignPutOptions {
                content_type: Some("text/plain".to_string()),
                ..Default::default()
            },
        )
        .await
        .expect("presign put");
    assert_eq!(put.method, http::Method::PUT);
    let client = reqwest::Client::new();
    let mut put_req = client.request(put.method.clone(), put.url());
    for (name, value) in put.headers.iter() {
        put_req = put_req.header(name, value);
    }
    put_req
        .body("uploaded via presign")
        .send()
        .await
        .expect("presigned put")
        .error_for_status()
        .expect("presigned put status");

    let roundtrip = store.get(put_path).await.expect("get after presign put");
    let bytes = roundtrip.bytes().await.expect("read put object");
    assert_eq!(bytes, Bytes::from("uploaded via presign"));

    let _ = store.delete(path).await;
    let _ = store.delete(put_path).await;
}
