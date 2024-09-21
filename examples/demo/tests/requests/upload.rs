use axum_test::multipart::{MultipartForm, Part};
use demo_app::{app::App, views};
use loco_rs::{app::AppContext, testing};
use serial_test::serial;

#[tokio::test]
#[serial]
async fn can_upload_file() {
    testing::request::<AppContext, App, _, _>(|request, ctx| async move {
        let file_content = "loco file upload";
        let file_part = Part::bytes(file_content.as_bytes()).file_name("loco.txt");

        let multipart_form = MultipartForm::new().add_part("file", file_part);

        let response = request.post("/upload/file").multipart(multipart_form).await;

        response.assert_status_ok();

        let res: views::upload::Response = serde_json::from_str(&response.text()).unwrap();

        let stored_file: String = ctx.storage.download(&res.path).await.unwrap();

        assert_eq!(stored_file, file_content);
    })
    .await;
}
