use axum::extract::Multipart;
use loco_rs::prelude::*;
use std::path::PathBuf;

/// File upload example
///
///  curl -H "Content-Type: multipart/form-data" -F "file=@./test-2.json" 127.0.0.1:3000/upload/file
async fn upload_file(State(ctx): State<AppContext>, mut multipart: Multipart) -> Result<()> {
    while let Some(field) = multipart.next_field().await.map_err(|err| {
        tracing::error!(error = ?err,"could not readd multipart");
        Error::BadRequest("could not readd multipart".into())
    })? {
        let file_name = match field.file_name() {
            Some(file_name) => file_name.to_string(),
            _ => return Err(Error::BadRequest("file name not found".into())),
        };

        let bytes = field.bytes().await.map_err(|err| {
            tracing::error!(error = ?err,"could not readd bytes");
            Error::BadRequest("could not readd bytes".into())
        })?;
        ctx.storage
            .as_ref()
            .unwrap()
            .primary
            .write(PathBuf::from("users").join(file_name).as_path(), bytes)
            .await?;
    }

    format::empty()
}

pub fn routes() -> Routes {
    Routes::new()
        .prefix("upload")
        .add("/file", post(upload_file))
}
