use anyhow::{Context as _, Error, Ok};
use chrono::{Duration, Local, NaiveTime};
use itertools::Itertools;
use poise::serenity_prelude::*;
use tokio::{
    fs::File,
    time::{sleep_until, Instant},
};

use crate::{data, utils::format_datetime};

pub async fn wait(ctx: Context) {
    loop {
        let now = Local::now();
        let target_time = {
            let time = Local::now()
                .with_time(NaiveTime::from_hms_opt(12, 0, 0).unwrap())
                .unwrap();
            if time < now {
                time + Duration::days(1)
            } else {
                time
            }
        };
        let sleep_duration = target_time - now;

        println!("Now: {}", now);
        println!("Next run: {}", target_time);
        println!("Sleeping for {} seconds", sleep_duration.num_seconds());

        sleep_until(Instant::now() + sleep_duration.to_std().unwrap()).await;
        notify(ctx.clone()).await.expect("Failed to notify");
        backup(ctx.clone()).await.expect("Failed to backup");
    }
}

async fn notify(ctx: Context) -> Result<(), Error> {
    let data = data::load()?;
    let ping_channel = (*data.ping_channel.lock().unwrap()).context("Ping channel not set")?;
    let ping_role = (*data.ping_role.lock().unwrap()).context("Ping role not set")?;
    let tasks = data.tasks.lock().unwrap().clone();

    let from = (Local::now() + Duration::days(1))
        .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        .unwrap();
    let to = (Local::now() + Duration::days(2))
        .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        .unwrap();

    println!("Searching tasks: from {} to {}", from, to);

    let fields = tasks
        .iter()
        .filter(|task| from < task.datetime && task.datetime <= to)
        .sorted_by_key(|task| task.datetime)
        .map(|task| task.to_field());

    if fields.len() > 0 {
        ping_channel
            .send_message(
                ctx,
                CreateMessage::new()
                    .content(format!("{}", ping_role.mention()))
                    .embed(
                        CreateEmbed::default()
                            .title("タスク通知")
                            .description("明日のタスクをお知らせします！")
                            .fields(fields)
                            .color(Color::RED),
                    ),
            )
            .await?;
    }
    Ok(())
}

async fn backup(ctx: Context) -> Result<(), Error> {
    let data = data::load()?;
    let log_channel = (*data.log_channel.lock().unwrap()).context("Log channel not set")?;

    log_channel
        .send_files(
            ctx,
            vec![
                CreateAttachment::file(
                    &File::open(data::FILE_PATH).await?,
                    format!("{}.json", Local::now().timestamp()),
                )
                .await?,
            ],
            CreateMessage::default().embed(CreateEmbed::default().title(format!(
                "データのバックアップ ({})",
                format_datetime(Local::now())
            ))),
        )
        .await?;

    Ok(())
}
