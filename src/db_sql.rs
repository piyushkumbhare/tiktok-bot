use sqlx::{
    sqlite::{SqliteConnectOptions, SqlitePool},
    Error, Row,
};

pub async fn insert_url(
    db: &SqlitePool,
    table: &str,
    url: &str,
    is_long_url: bool,
    media_link: &str,
) -> Result<i64, Error> {
    let mut conn = db.acquire().await?;

    let url_type: &str = match is_long_url {
        true => "long_url",
        false => "short_url",
    };

    let update_query = format!(
        "
        UPDATE {table}
        SET {url_type}='{url}'
        WHERE media_link='{media_link}'
    "
    );

    // Attempt to update the existing record with the new information
    let row = sqlx::query(&update_query).execute(&mut *conn).await?;

    if row.rows_affected() == 0 {
        // No entry exists for said media link. Create one...
        let insert_query = format!(
            "
            INSERT INTO {table} ({url_type}, media_link)
            VALUES ('{url}', '{media_link}')
        "
        );

        let insert = sqlx::query(&insert_query)
            .execute(&mut *conn)
            .await?
            .last_insert_rowid();
        return Ok(insert);
    }
    Ok(row.last_insert_rowid())
}

pub async fn fetch_media_link(
    db: &SqlitePool,
    table: &str,
    is_long_url: bool,
    url: &str,
) -> Result<String, Error> {
    let mut conn = db.acquire().await?;

    let url_type: &str = match is_long_url {
        true => "long_url",
        false => "short_url",
    };

    let fetch_query = format!(
        "
        SELECT media_link
        FROM {table}
        WHERE {url_type}='{url}'
    "
    );

    let link: String = sqlx::query(&fetch_query)
        .fetch_one(&mut *conn)
        .await?
        .get(0);

    Ok(link)
}

#[allow(dead_code)]

pub async fn print_db(db: &SqlitePool, table: &str) -> Result<(), Error> {
    let mut conn = db.acquire().await?;

    let print_query = format!(
        "
        SELECT *
        FROM {table}
    "
    );

    let _res = sqlx::query(&print_query)
        .fetch_all(&mut *conn)
        .await?
        .iter()
        .for_each(|row| {
            println!("======================================");
            for i in 0..row.len() {
                let t: &str = match i {
                    0 => "short_url",
                    1 => "long_url",
                    2 => "media_link",
                    _ => "",
                };
                let s: &str = row.try_get(i).unwrap_or("");
                println!("{t}:\n{s}\n");
            }
        });

    Ok(())
}

pub async fn connect(filename: &str, table: &str, create_table: &str) -> Result<SqlitePool, Error> {
    // Create the options and connect to the database, create if missing.
    let options = SqliteConnectOptions::new()
        .filename(filename)
        .create_if_missing(true);

    let db = SqlitePool::connect_with(options).await?;

    let mut conn = db.acquire().await?;

    // Check if the table provided exists
    let table_exists: i32 = sqlx::query_scalar(
        "
        SELECT COUNT(*)
        FROM sqlite_master
        WHERE type='table' AND name=?
    ",
    )
    .bind(table)
    .fetch_one(&mut *conn)
    .await?;

    if table_exists == 1 {
        println!("Found table '{table}'!");
        return Ok(db);
    } else {
        // Create said table
        println!("Table '{table}', not found, attempting to create...");
        let _create_table = sqlx::query(create_table).execute(&mut *conn).await?;
        println!("Successfully created table '{table}'!");
        return Ok(db);
    }
}
