+++
title = "Storage"
description = ""
date = 2021-05-01T18:20:00+00:00
updated = 2021-05-01T18:20:00+00:00
draft = false
weight = 23
sort_by = "weight"
template = "docs/page.html"

[extra]
lead = ""
toc = true
top = false
flair =[]
+++

By testing file storage in your controller you can follow this example:
```rust
#[tokio::test]
#[serial]
async fn can_register() {
    testing::request::<App, _, _>(|request, ctx| async move {
        let file_content = "loco file upload";
        let file_part = Part::bytes(file_content.as_bytes()).file_name("loco.txt");

        let multipart_form = MultipartForm::new().add_part("file", file_part);

        let response = request.post("/upload/file").multipart(multipart_form).await;

        response.assert_status_ok();

        let res: views::upload::Response = serde_json::from_str(&response.text()).unwrap();

        let stored_file: String = ctx.storage.unwrap().download(&res.path).await.unwrap();

        assert_eq!(stored_file, file_content);
    })
    .await;
}
```

