use regex::Regex;
use serenity::{all::*, async_trait};
use sqlx::sqlite::SqlitePool;
use std::{env, process};
use uuid::Uuid;
use poise;

mod db_sql;
struct Handler {
    re_short: Regex,
    re_long: Regex,
    db: SqlitePool,
    table_name: &'static str,
}

#[poise::command(slash_command)]
async fn shutdown<U: Sync, E>(ctx: poise::Context<'_, U, E>) -> Result<(), E> {

}

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, ctx: Context, msg: Message) {
        if msg.is_own(&ctx) {
            return;
        }
        let mut is_long_url = true;
        let url;
        if let Some(short_result) = self.re_short.captures(&msg.content) {
            is_long_url = false;
            url = match short_result.get(0) {
                Some(x) => x.as_str(),
                None => return,
            };
        } else if let Some(long_result) = self.re_long.captures(&msg.content) {
            url = match long_result.get(0) {
                Some(x) => x.as_str(),
                None => return,
            };
        } else {
            return;
        }
        if url.len() < msg.content.len() {
            return;
        }
        println!("Found a message with a link!");

        match db_sql::fetch_media_link(&self.db, &self.table_name, is_long_url, &url).await {
            Ok(link) => {
                println!("Found this link in my database!");
                if let Err(err) = msg
                    .channel_id
                    .say(&ctx.http, format!("{url}\n{link}"))
                    .await
                {
                    println!("Ran into error when sending message: {err}");
                    return;
                }
            }
            Err(e) => {
                println!("Ran into some error {e}...");
                match e {
                    sqlx::Error::RowNotFound => {
                        println!("Link not found in database. Calling yt-dlp_linux to create...");

                        let filename = format!("download-{}.mp4", Uuid::new_v4());

                        if process::Command::new("yt-dlp_linux")
                            .arg(url)
                            .arg("-o")
                            .arg(&filename)
                            .output()
                            .is_err()
                        {
                            println!("Error with running yt-dlp_linux");
                            return;
                        }

                        match CreateAttachment::path(format!("./{}", &filename)).await {
                            Ok(x) => {
                                println!("Attempting to send file in message...");
                                let builder = CreateMessage::new().content(url).add_file(x);
                                match msg.channel_id.send_message(&ctx.http, builder).await {
                                    Ok(x) => {
                                        if let Some(att) = x.attachments.get(0) {
                                            if let Err(e) = db_sql::insert_url(
                                                &self.db,
                                                &self.table_name,
                                                &url,
                                                is_long_url,
                                                &att.url,
                                            )
                                            .await
                                            {
                                                println!("{e}");
                                                return;
                                            }
                                        }
                                    }
                                    Err(err) => {
                                        println!("Ran into error when sending message: {err}");
                                        return;
                                    }
                                }
                                println!("File sent successfully!");
                            }
                            Err(e) => println!("{e}"),
                        }

                        println!("Deleting file {}...", &filename);
                        if process::Command::new("rm").arg(&filename).output().is_err() {
                            println!("Error deleting file {}", &filename);
                            return;
                        }
                    }
                    _ => (),
                }
            }
        }
        if let Err(err) = msg.delete(&ctx.http).await {
            println!("{err}");
        }
    }

    async fn ready(&self, _ctx: Context, ready: Ready) {
        println!("Connected! {}", ready.user.name);
        // if let Err(e) = db_sql::print_db(&self.db, &self.table_name).await {
        // 	println!("{e}");
        // }
    }
}

#[tokio::main]
async fn main() {
    let token = env!("TOKEN");
    let filename = env!("DB_FILE");
    println!("{}", token);

    let table_name = "url_to_media";
    let create_table_query = format!(
        "
        CREATE TABLE {} (
            short_url varchar(255),
            long_url varchar(255),
            media_link varchar(255) UNIQUE
        )
    ",
        table_name
    );

    let db = db_sql::connect(&filename, &table_name, &create_table_query)
        .await
        .unwrap();

    let mut client = Client::builder(&token, GatewayIntents::all())
        .event_handler(Handler {
            re_short: Regex::new(r"(https://www.tiktok.com/t/\w+/).*").unwrap(),
            re_long: Regex::new(r"(https://www.tiktok.com/@\w+/video/[0-9]+).*").unwrap(),
            db: db,
            table_name: &table_name,
        })
        .await
        .expect("Error with client creation.");

    if let Err(why) = client.start().await {
        println!("Error in starting client: {}", why);
    }
}
