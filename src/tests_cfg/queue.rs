use std::path::PathBuf;

#[cfg(any(feature = "bg_pg", feature = "bg_sqlt"))]
use crate::bgworker;

#[cfg(feature = "bg_pg")]
/// # Panics
///
/// This function will panic if it fails to prepare or insert the seed data,
/// causing the tests to fail quickly and preventing further test execution with
/// incomplete setup.
pub async fn postgres_seed_data(pool: &sqlx::PgPool) {
    let yaml_tasks = std::fs::read_to_string(
        PathBuf::from("tests")
            .join("fixtures")
            .join("queue")
            .join("jobs.yaml"),
    )
    .expect("Failed to read YAML file");

    let tasks: Vec<bgworker::pg::Job> =
        serde_yaml::from_str(&yaml_tasks).expect("Failed to parse YAML");
    for task in tasks {
        sqlx::query(
            r"
            INSERT INTO pg_loco_queue (id, name, task_data, status, run_at, interval, created_at, updated_at)
            VALUES ($1, $2, $3, $4, $5, NULL, $6, $7)
            ",
        )
        .bind(task.id)
        .bind(task.name)
        .bind(task.data)
        .bind(task.status.to_string())
        .bind(task.run_at)
        .bind(task.created_at)
        .bind(task.updated_at)
        .execute(pool)
        .await.expect("execute insert query");
    }
}

#[cfg(feature = "bg_sqlt")]
/// # Panics
///
/// This function will panic if it fails to prepare or insert the seed data,
/// causing the tests to fail quickly and preventing further test execution with
/// incomplete setup.
pub async fn sqlite_seed_data(pool: &sqlx::Pool<sqlx::Sqlite>) {
    let yaml_tasks = std::fs::read_to_string(
        PathBuf::from("tests")
            .join("fixtures")
            .join("queue")
            .join("jobs.yaml"),
    )
    .expect("Failed to read YAML file");

    let tasks: Vec<bgworker::sqlt::Job> =
        serde_yaml::from_str(&yaml_tasks).expect("Failed to parse YAML");
    for task in tasks {
        sqlx::query(
            r"
            INSERT INTO sqlt_loco_queue (id, name, task_data, status, run_at, interval, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, NULL, ?, ?)
            "
        )
        .bind(task.id)
        .bind(task.name)
        .bind(task.data.to_string())
        .bind(task.status.to_string())
        .bind(task.run_at)
        .bind(task.created_at)
        .bind(task.updated_at)
        .execute(pool)
        .await.expect("create row");
    }

    sqlx::query(
        r"
                INSERT INTO sqlt_loco_queue_lock (id, is_locked, locked_at)
    VALUES (1, FALSE, NULL)
    ON CONFLICT (id) DO NOTHING;

            ",
    )
    .execute(pool)
    .await
    .expect("execute insert query");
}
