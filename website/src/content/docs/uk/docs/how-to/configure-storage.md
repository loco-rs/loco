---
title: Налаштування файлового сховища
description: Підключіть Storage API поверх локального диска, пам'яті чи хмари (S3/Azure/GCS), оберіть стратегію дзеркалення/резервного копіювання та стрімте великі файли.
sidebar:
  order: 30
---

Мета: дати вашому застосунку місце для завантажених файлів — на диску, у пам'яті (для тестів) або в хмарному бакеті — через один послідовний `Storage` API, без написання власного клієнта OpenDAL.

Шар сховища Loco — це тонка абстракція над [Apache OpenDAL](https://opendal.apache.org/). Кожен драйвер зрештою реалізує той самий трейт `StoreDriver`, тож ваш код контролера не змінюється, коли ви замінюєте локальний диск на S3.

## Передумови

Локальне, в пам'яті та null-сховище працюють без додаткових Cargo-фіч. Хмарним драйверам потрібна одна з:

```toml
loco-rs = { version = "...", features = ["storage_aws_s3"] } # або storage_azure, storage_gcp, all_storage
```

Повну матрицю дивіться у [довіднику feature-прапорців](/uk/docs/reference/feature-flags/).

## 1. Підключіть один драйвер

Сховище не налаштовується в YAML — воно підключається в коді, у хуці `after_context` (`src/app.rs`), і потрапляє в `ctx.storage: Arc<Storage>`.

```rust
use loco_rs::storage::{self, drivers};

async fn after_context(ctx: AppContext) -> Result<AppContext> {
    Ok(ctx
        .into_builder()
        .storage(storage::Storage::single(drivers::local::new()).into())
        .build())
}
```

`AppContext` є `#[non_exhaustive]`, тож `AppContext { storage, ..ctx }` не скомпілюється у вашому застосунку — саме це захищає збірку від зламу, коли в майбутньому релізі Loco з'явиться нове поле. `into_builder()` — це заміна: він переносить усе, що послідовність завантаження вже приєднала (мейлер, чергу, кеш, спільне сховище), а ви перевизначаєте лише те, що вас цікавить. Побудова через `AppContext::builder(..)` замість цього скомпілювалася б, але мовчки відкинула б решту.

Якщо ви взагалі не перевизначаєте `after_context`, Loco за замовчуванням використовує драйвер **`Null`** — кожна операція сховища повертає `StorageError::Any("Operation not supported by null storage")`. Це навмисна поведінка fail-fast за замовчуванням, а не баг: вона означає «ви ще не підключили сховище».

## 2. Оберіть драйвер

Кожен драйвер будується звичайною функцією-конструктором у `loco_rs::storage::drivers::*` — жодних маніпуляцій з trait-об'єктами не потрібно.

| Драйвер | Фіча | Конструктор | Примітки |
|---|---|---|---|
| Локальна файлова система | немає | `drivers::local::new()` — коренем є поточна робоча тека<br>`drivers::local::new_with_prefix(prefix) -> StorageResult<Box<dyn StoreDriver>>` | `new_with_prefix` повертає помилку, якщо шлях префікса не існує |
| У пам'яті | немає | `drivers::mem::new()` | Добре для тестів; дані не переживають завершення процесу |
| Null | немає | `drivers::null::new()` | Стандартний для фреймворку; кожна операція помиляється |
| AWS S3 | `storage_aws_s3` | `drivers::aws::new(bucket, region) -> StorageResult<...>`<br>`drivers::aws::with_credentials(bucket, region, cred) -> StorageResult<...>`<br>`drivers::aws::with_credentials_and_endpoint(bucket, region, endpoint, cred) -> StorageResult<...>` | `Credential { key_id, secret_key, token: Option<String> }` |
| Azure Blob | `storage_azure` | `drivers::azure::new(container, account_name, access_key, endpoint) -> StorageResult<...>` | |
| Google Cloud Storage | `storage_gcp` | `drivers::gcp::new(bucket, credential_path) -> StorageResult<...>` | `credential_path` вказує на JSON-файл ключа service-акаунта |

Усі хмарні конструктори повертають `StorageResult<Box<dyn StoreDriver>>` (вони можуть не зуміти побудувати нижчий оператор OpenDAL), тож поширюйте помилку через `?`:

```rust
use loco_rs::storage::{self, drivers};

async fn after_context(ctx: AppContext) -> Result<AppContext> {
    let store = drivers::aws::new("my-app-uploads", "us-east-1")?;
    Ok(ctx
        .into_builder()
        .storage(storage::Storage::single(store).into())
        .build())
}
```

Для облікових даних, яких немає в середовищі/профілі інстансу, передайте їх явно:

```rust
use loco_rs::storage::drivers::aws::{self, Credential};

let credential = Credential {
    key_id: std::env::var("AWS_ACCESS_KEY_ID")?,
    secret_key: std::env::var("AWS_SECRET_ACCESS_KEY")?,
    token: None,
};
let store = aws::with_credentials("my-app-uploads", "us-east-1", credential)?;
```

> Трейт драйвера сховища називається `StoreDriver` (не `StorageDriver`) — ви побачите його в повідомленнях про помилки та якщо реалізуватимете власний драйвер.

## 3. Використовуйте кілька драйверів зі стратегією (опційно)

Для резервування між провайдерами налаштуйте кілька іменованих сховищ і `StorageStrategy`, яка вирішує, як операції розподіляються між ними.

**Дзеркало (Mirror)** — реплікує завантаження/видалення/перейменування/копіювання до кожного сховища; читання намагається основне сховище, а в разі невдачі переходить до другорядних. Це `ReplicatedStrategy::mirror`.

```rust
use std::collections::BTreeMap;
use loco_rs::storage::{
    drivers, Storage,
    strategies::{replicated::{ReplicatedStrategy, FailurePolicy}, StorageStrategy},
};

let primary = drivers::aws::new("bucket-primary", "us-east-1")?;
let mirror = drivers::azure::new("container", "account", "access-key", "https://account.blob.core.windows.net")?;

let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::mirror(
    "primary",
    Some(vec!["mirror".to_string()]),
    FailurePolicy::FailIfAny, // або AllowAll
));

let storage = Storage::new(
    BTreeMap::from([
        ("primary".to_string(), primary),
        ("mirror".to_string(), mirror),
    ]),
    strategy,
);
```

`FailurePolicy::FailIfAny` вимагає успіху кожного другорядного сховища (помилки спливають угору як `StorageError::Multi`); `AllowAll` ковтає збої другорядних.

**Резервне копіювання (Backup)** — основне сховище має завжди успішно виконувати запис; збої другорядних керуються окремою політикою відмов, а читання *завжди* надходять лише з основного сховища. Це `ReplicatedStrategy::backup`.

```rust
use loco_rs::storage::strategies::replicated::{ReplicatedStrategy, FailurePolicy};

let strategy: Box<dyn StorageStrategy> = Box::new(ReplicatedStrategy::backup(
    "primary",
    Some(vec!["backup_store".to_string()]),
    FailurePolicy::AllowAll, // також: FailIfAny, AllowSingleFailure, FailAtFailures(n)
));
```

Дзеркало та резервне копіювання — обидва `ReplicatedStrategy`, що відрізняються лише використаним конструктором (`mirror` vs `backup`) і обраним `FailurePolicy`. Кожен метод `Storage` має варіант `_with_policy`/`_with_strategy` (`upload_with_strategy`, `download_with_policy`, ...), якщо потрібно перевизначити стратегію для одного виклику.

## 4. Завантаження та читання у контролері

```rust
use loco_rs::prelude::*;
use std::path::PathBuf;

async fn upload_file(
    State(ctx): State<AppContext>,
    mut multipart: Multipart,
) -> Result<Response> {
    while let Some(field) = multipart.next_field().await.map_err(|_| {
        Error::BadRequest("could not read multipart".into())
    })? {
        let file_name = field
            .file_name()
            .map(str::to_string)
            .ok_or_else(|| Error::BadRequest("file name not found".into()))?;

        let content = field
            .bytes()
            .await
            .map_err(|_| Error::BadRequest("could not read bytes".into()))?;

        let path = PathBuf::from("uploads").join(file_name);
        ctx.storage.as_ref().upload(&path, &content).await?;

        return format::json(serde_json::json!({ "path": path }));
    }
    not_found()
}
```

(Потребує фічі `multipart` у крейті `axum`.)

## 5. Стрімте великі файли замість буферизації

Для файлів, завеликих, щоб комфортно тримати їх у пам'яті, використайте streaming API — `download_stream`/`upload_stream` повертають/приймають `BytesStream`, який конвертується напряму з/у axum `Body`. У попередніх релізах Loco це не було задокументовано, але це стабільна, повністю публічна фіча.

Стрімінг читання прямо в HTTP-відповідь, без жодної додаткової буферизації:

```rust
use axum::response::IntoResponse;
use std::path::Path;

async fn download_video(State(ctx): State<AppContext>) -> Result<impl IntoResponse> {
    let stream = ctx.storage.download_stream(Path::new("videos/demo.mp4")).await?;
    Ok(stream.into_body())
}
```

Стрімінг завантаження з тіла вхідного запиту (потік `Body` в axum видає `axum::Error`, тож спершу відобразіть її в `std::io::Error` — саме цей тип помилки очікує `BytesStream`):

```rust
use loco_rs::storage::stream::BytesStream;
use futures_util::StreamExt;
use std::path::Path;

async fn upload_video(State(ctx): State<AppContext>, body: axum::body::Body) -> Result<Response> {
    let mapped = body
        .into_data_stream()
        .map(|chunk| chunk.map_err(std::io::Error::other));
    let stream = BytesStream::from_body_stream(mapped);
    ctx.storage.upload_stream(Path::new("videos/demo.mp4"), stream).await?;
    format::empty()
}
```

Якщо вам усе одно потрібен увесь payload як один буфер `Bytes`, `BytesStream::collect()` це дасть — але в цей момент ви вже відмовилися від переваги стрімінгу в пам'яті.

**Застереження щодо стратегій:** стрімінг не є рівномірно «справжнім стрімінгом», коли залучена стратегія, відмінна від `SingleStrategy`. `ReplicatedStrategy` уніфікує колишню поведінку дзеркала/резерву: читання (як буферизований `download`, так і `download_stream`) переходять до другорядних, коли встановлено `read_from_secondaries` — тобто конструкція через `ReplicatedStrategy::mirror` — і обслуговуються лише з основного сховища, коли конструкція через `ReplicatedStrategy::backup`. В обох випадках `upload_stream` буферизує весь payload один раз через `collect()`, а потім конкурентно розподіляє до другорядних. Якщо вам потрібен гарантований стрімінг без буферизації до одного сховища, залишайтеся на `SingleStrategy` (стандартний).

## 6. Перевірка існування, перелік і статистика

`Storage` також надає `exists`, `list` і `stat`, кожен із яких проходить через обрану стратегію так само, як `upload`/`download` (із братніми `exists_with_policy`/`list_with_policy`/`stat_with_policy` для перевизначення стратегії на виклик).

```rust
use std::path::Path;

// Чи існує ключ?
let found = ctx.storage.exists(Path::new("uploads/report.pdf")).await?;

// Перелічити все під префіксом, рекурсивно.
let all_entries = ctx.storage.list(Path::new("uploads"), true).await?;

// Перелічити на один рівень углиб — дочірні префікси повертаються як записи директорій.
let top_level = ctx.storage.list(Path::new("uploads"), false).await?;

// Метадані одного ключа, без читання його вмісту.
let meta = ctx.storage.stat(Path::new("uploads/report.pdf")).await?;
println!("{} bytes, is_dir={}", meta.content_length.unwrap_or(0), meta.is_dir);
```

Кожен запис, що повертається `list`/`stat`, — це `storage::drivers::ListEntry` (надавайте перевагу `ListEntry::new(...)` над структурними літералами).

На `ReplicatedStrategy` (дзеркало / `read_from_secondaries`):

- `stat` переходить до другорядних у разі помилки основного сховища (те саме, що `download`)
- `exists` переходить, коли основне сховище повідомляє `false` **або** помиляється (промах сам по собі не є помилкою)
- `list` переходить, коли основне сховище помиляється **або** повертає порожній перелік; порожні другорядні пропускаються, тож пізніше другорядне сховище з даними все одно перемагає

Режим резерву (`read_from_secondaries: false`) залишає всі три лише на основному сховищі.

## 7. Presign для прямих завантажень і читань

`Storage` надає `presign_get` і `presign_put` для передачі клієнтам URL з обмеженим часом дії, який звертається до бекенд-сховища напряму (S3, Azure Blob, GCS) без проксіювання байтів через ваш застосунок.

```rust
use std::{path::Path, time::Duration};
use loco_rs::storage::drivers::PresignPutOptions;

let ttl = Duration::from_secs(300);

let download = ctx.storage.presign_get(Path::new("exports/checkpoint.bin"), ttl).await?;
println!("GET {}", download.url());

let upload = ctx
    .storage
    .presign_put(
        Path::new("uploads/report.pdf"),
        ttl,
        PresignPutOptions {
            content_type: Some("application/pdf".to_string()),
            ..Default::default()
        },
    )
    .await?;
println!("{} {}", upload.method, upload.url());
```

Кожен виклик повертає `storage::drivers::PresignedRequest` з `method`, `uri`, `headers` і хелпером `url()`. Клієнти мають надсилати підписані заголовки точно так, як вони повернуті.

На `ReplicatedStrategy` presign працює **лише з основним сховищем** — presign-URL прив'язані до облікових даних одного бекенду й ніколи не переходять до другорядних.

Щоб перевірити presign на реальному S3-сумісному ендпоінті в тестах (ігнорується у звичайному CI; встановіть змінні середовища та передайте `-- --ignored`):

```sh
LOCO_TEST_S3_ENDPOINT=http://127.0.0.1:9000 \
LOCO_TEST_S3_BUCKET=loco-presign-test \
LOCO_TEST_S3_ACCESS_KEY_ID=minio \
LOCO_TEST_S3_SECRET_ACCESS_KEY=minio123 \
cargo test -p loco-rs presign_s3_roundtrip --features storage_aws_s3 -- --ignored
```

## 8. Перевірте

```rust
use axum_test::multipart::{MultipartForm, Part}; // не реекспортується testing prelude
use loco_rs::testing::prelude::*;

#[tokio::test]
#[serial]
async fn can_upload_and_download() {
    request::<App, _, _>(|request, ctx| async move {
        let file_content = "loco file upload";
        let file_part = Part::bytes(file_content.as_bytes()).file_name("loco.txt");
        let multipart_form = MultipartForm::new().add_part("file", file_part);

        let response = request.post("/upload/file").multipart(multipart_form).await;
        response.assert_status_ok();

        let res: serde_json::Value = serde_json::from_str(&response.text()).unwrap();
        let path = res["path"].as_str().unwrap();

        let stored: String = ctx.storage.as_ref().download(&std::path::Path::new(path)).await.unwrap();
        assert_eq!(stored, file_content);
    })
    .await;
}
```

## Довідник

- Feature-прапорці `storage_aws_s3` / `storage_azure` / `storage_gcp` / `all_storage`: [довідник feature-прапорців](/uk/docs/reference/feature-flags/)
- Сховище не має поверхні YAML-конфігурації — усе наведене вище є повною історією конфігурації; ключа `storage:` для пошуку у [довіднику конфігурації](/uk/docs/reference/configuration/) немає
