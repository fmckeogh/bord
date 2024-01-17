use std::time::Duration;
use tokio::time::MissedTickBehavior;
use {
    bollard::Docker,
    color_eyre::Result,
    serde::Serialize,
    sqlx::{Pool, Postgres},
    tracing::error,
};

const CHECK_INTERVAL: Duration = Duration::from_secs(5);

/// Waits for new submissions then benchmarks them
pub async fn submission_runner(db: Pool<Postgres>, docker: Docker) {
    let mut interval = tokio::time::interval(CHECK_INTERVAL);
    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    loop {
        interval.tick().await;

        if let Err(e) = process_submission(&db, &docker).await {
            error!("encountered error while processing submission: {:?}", e);
        }
    }
}

async fn process_submission(db: &Pool<Postgres>, _docker: &Docker) -> Result<()> {
    // get contents of oldest submission
    let Some(entry) = get_oldest_submission(db).await? else {
        return Ok(());
    };

    // create image, put contents in designated folder

    // run it
    // retrieve results

    insert_leaderboard_entry(
        db,
        LeaderboardEntry {
            submitted_at: entry.submitted_at,
            score: 1_000_000,
            username: entry.username,
            file_name: entry.file_name,
            file_size: i32::try_from(entry.file_contents.len())?,
        },
    )
    .await?;

    Ok(())
}

#[derive(Debug, Clone)]
pub struct SubmissionEntry {
    submitted_at: i64,
    username: String,
    file_name: String,
    file_contents: Vec<u8>,
}

async fn get_oldest_submission(db: &Pool<Postgres>) -> Result<Option<SubmissionEntry>> {
    Ok(sqlx::query_as!(
        SubmissionEntry,
        r#"
            SELECT
                submissions.submitted_at,
                username,
                file_name,
                file_contents
            FROM submissions
            ORDER BY submitted_at
            LIMIT 1
        "#
    )
    .fetch_optional(db)
    .await?)
}

#[derive(Debug, Clone, Serialize)]
pub struct LeaderboardEntry {
    submitted_at: i64,
    score: i32,
    username: String,
    file_name: String,
    file_size: i32,
}

/// Inserts a new leaderboard entry and removes the corresponding submission
/// from the queue
async fn insert_leaderboard_entry(db: &Pool<Postgres>, entry: LeaderboardEntry) -> Result<()> {
    // start transaction
    let mut tx = db.begin().await?;

    // delete submission
    sqlx::query!(
        r#"
            DELETE FROM submissions
            WHERE username = $1 AND submitted_at = $2
            "#,
        &entry.username,
        entry.submitted_at
    )
    .execute(&mut *tx)
    .await?;

    // insert new leaderboard entry
    sqlx::query!(
        r#"
                INSERT INTO leaderboard (username, submitted_at, file_name, file_size, score)
                VALUES ($1, $2, $3, $4, $5)
                ON CONFLICT (username) DO UPDATE
                SET
                    submitted_at = EXCLUDED.submitted_at,
                    file_name = EXCLUDED.file_name,
                    file_size = EXCLUDED.file_size,
                    score = EXCLUDED.score

            "#,
        &entry.username,
        entry.submitted_at,
        &entry.file_name,
        &entry.file_size,
        &entry.score
    )
    .execute(&mut *tx)
    .await?;

    Ok(tx.commit().await?)
}
