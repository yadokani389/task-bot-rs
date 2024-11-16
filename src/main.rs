use anyhow::Error;
use dotenvy::dotenv;
use poise::serenity_prelude::*;

mod data;
use data::*;
mod commands;
use commands::*;
mod daily_job;

pub type PoiseContext<'a> = poise::Context<'a, Data, Error>;

async fn event_handler(
    ctx: &Context,
    event: &FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    if let FullEvent::Ready { data_about_bot } = event {
        println!("Logged in as {}", data_about_bot.user.name);
        match load() {
            Ok(restore) => {
                *data.tasks.lock().unwrap() = restore.tasks.lock().unwrap().clone();
                *data.subjects.lock().unwrap() = restore.subjects.lock().unwrap().clone();
                *data.suggest_times.lock().unwrap() = restore.suggest_times.lock().unwrap().clone();
                *data.panel_message.lock().unwrap() = restore.panel_message.lock().unwrap().clone();
                *data.ping_channel.lock().unwrap() = *restore.ping_channel.lock().unwrap();
                *data.ping_role.lock().unwrap() = *restore.ping_role.lock().unwrap();
                *data.log_channel.lock().unwrap() = *restore.log_channel.lock().unwrap();
                println!("Config restored:");
                println!("{:#?}", data);
            }
            Err(_) => {
                println!("Note: {} not found, using default data", DATA_FILE);
                save(data)?;
            }
        }
        tokio::spawn(daily_job::run_daily_job(ctx.clone()));
        if let Some(panel_message) = &*data.panel_message.lock().unwrap() {
            data.panel_listener.lock().unwrap().replace(tokio::spawn(
                commands::panel::listen_panel_interactions(ctx.clone(), panel_message.clone()),
            ));
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().expect(".env file not found");

    let token = std::env::var("DISCORD_TOKEN").expect("Missing DISCORD_TOKEN");
    let intents = GatewayIntents::non_privileged();

    let framework = poise::Framework::builder()
        .options(poise::FrameworkOptions {
            commands: vec![
                modify_tasks::add_task(),
                modify_tasks::remove_task(),
                modify_tasks::edit_task(),
                modify_subjects::add_subjects(),
                modify_subjects::remove_subject(),
                modify_suggest_times::add_suggest_time(),
                modify_suggest_times::remove_suggest_time(),
                panel::deploy_panel(),
                ping_config::set_ping_channel(),
                ping_config::set_ping_role(),
                log_config::set_log_channel(),
            ],
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data::default())
            })
        })
        .build();

    let client = ClientBuilder::new(token, intents)
        .framework(framework)
        .await;
    client
        .expect("Failed to create client")
        .start()
        .await
        .expect("Failed to start client");
}
