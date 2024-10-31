use anyhow::{anyhow, Error, Ok};
use chrono::{Duration, Local, NaiveTime};
use itertools::Itertools;
use poise::serenity_prelude as serenity;
use serenity::Mentionable;
use tokio::time::{sleep_until, Instant};

use crate::load;

pub async fn run_daily_job(ctx: serenity::Context) {
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
        let sleep_duration = Duration::seconds(target_time.timestamp() - now.timestamp());

        println!("Now: {}", now);
        println!("Next run: {}", target_time);
        println!("Sleeping for {} seconds", sleep_duration.num_seconds());

        sleep_until(Instant::now() + sleep_duration.to_std().unwrap()).await;
        job(ctx.clone()).await.expect("Failed to run daily job");
    }
}

async fn job(ctx: serenity::Context) -> Result<(), Error> {
    let data = load()?;
    let ping_channel =
        (*data.ping_channel.lock().unwrap()).ok_or(anyhow!("Ping channel not set"))?;
    let ping_role = (*data.ping_role.lock().unwrap()).ok_or(anyhow!("Ping role not set"))?;
    let tasks = data.tasks.lock().unwrap().clone();

    println!(
        "Searching tasks: from {} to {}",
        (Local::now() + Duration::days(1))
            .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .unwrap(),
        (Local::now() + Duration::days(2))
            .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            .unwrap()
    );

    let fields = tasks
        .iter()
        .filter(|task| {
            (Local::now() + Duration::days(1))
                .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                .unwrap()
                <= task.datetime
                && task.datetime
                    < (Local::now() + Duration::days(2))
                        .with_time(NaiveTime::from_hms_opt(0, 0, 0).unwrap())
                        .unwrap()
        })
        .sorted_by_key(|task| task.datetime)
        .map(|task| task.to_field());

    if fields.clone().count() == 0 {
        return Ok(());
    }

    ping_channel
        .send_message(
            ctx,
            serenity::CreateMessage::new()
                .content(format!("{}", ping_role.mention()))
                .embed(
                    serenity::CreateEmbed::default()
                        .title("タスク通知")
                        .description("明日のタスクをお知らせします！")
                        .fields(fields)
                        .color(serenity::Color::RED),
                ),
        )
        .await?;
    Ok(())
}
