use std::time::Duration;

use anyhow::{anyhow, Error};
use chrono::Local;
use itertools::Itertools;
use poise::serenity_prelude as serenity;
use serenity::{futures::StreamExt, Mentionable};

use crate::{load, save, Context};

const SHOW_TASKS: &str = "show_tasks";
const SHOW_ARCHIVED_TASKS: &str = "show_archived_tasks";

#[poise::command(slash_command)]
/// パネルをデプロイします。
pub async fn deploy_panel(ctx: Context<'_>) -> Result<(), Error> {
    let message = ctx
        .channel_id()
        .send_message(
            ctx,
            serenity::CreateMessage::default()
                .embed(
                    serenity::CreateEmbed::default()
                        .title("タスク確認")
                        .description("ボタンを押すとタスクを確認できます")
                        .color(serenity::Color::BLUE),
                )
                .components(vec![serenity::CreateActionRow::Buttons(vec![
                    serenity::CreateButton::new(SHOW_TASKS)
                        .label("タスク一覧")
                        .style(serenity::ButtonStyle::Success),
                    serenity::CreateButton::new(SHOW_ARCHIVED_TASKS)
                        .label("過去のタスク一覧")
                        .style(serenity::ButtonStyle::Secondary),
                ])]),
        )
        .await?;
    ctx.data()
        .panel_message
        .lock()
        .unwrap()
        .replace(message.clone());
    save(ctx.data())?;
    ctx.data()
        .panel_listener
        .lock()
        .unwrap()
        .as_ref()
        .inspect(|h| h.abort());
    ctx.data()
        .panel_listener
        .lock()
        .unwrap()
        .replace(tokio::spawn(listen_panel_interactions(
            ctx.serenity_context().clone(),
            message,
        )));
    Ok(())
}

pub async fn listen_panel_interactions(
    ctx: serenity::Context,
    msg: serenity::Message,
) -> Result<(), Error> {
    let mut interaction_stream = msg.await_component_interaction(&ctx).stream();
    while let Some(interaction) = interaction_stream.next().await {
        match &interaction.data.custom_id[..] {
            SHOW_TASKS => {
                tokio::spawn(show_tasks(interaction.clone(), ctx.clone()));
            }
            SHOW_ARCHIVED_TASKS => {
                tokio::spawn(show_archived_tasks(interaction.clone(), ctx.clone()));
            }
            _ => {}
        }
    }
    Ok(())
}

async fn log(
    ctx: &serenity::Context,
    user: &serenity::User,
    message: impl Into<String>,
) -> Result<(), Error> {
    let log_channel = load()?.log_channel.lock().unwrap().clone();

    log_channel
        .ok_or(anyhow!("log channel not set"))?
        .send_message(
            &ctx,
            serenity::CreateMessage::default().embed(
                serenity::CreateEmbed::default()
                    .thumbnail(user.avatar_url().unwrap_or_default())
                    .author(
                        serenity::CreateEmbedAuthor::new(user.name.clone())
                            .icon_url(user.avatar_url().unwrap_or_default()),
                    )
                    .title("パネル操作")
                    .timestamp(Local::now())
                    .description(message)
                    .color(serenity::Color::DARK_BLUE),
            ),
        )
        .await?;
    Ok(())
}

async fn show_tasks(
    interaction: serenity::ComponentInteraction,
    ctx: serenity::Context,
) -> Result<(), Error> {
    const PREV: &str = "prev";
    const NEXT: &str = "next";

    let mut page = 0;
    let message = |page: usize| -> Result<_, Error> {
        let tasks = load()?.tasks.lock().unwrap().clone();
        let fields = tasks
            .iter()
            .filter(|e| Local::now() <= e.datetime)
            .sorted_by_key(|e| e.datetime)
            .map(|task| task.to_field())
            .skip(5 * page);

        Ok(serenity::CreateInteractionResponseMessage::new()
            .embed(
                serenity::CreateEmbed::default()
                    .title("タスク一覧")
                    .description(if fields.len() == 0 {
                        "ありません！:tada:"
                    } else {
                        ""
                    })
                    .fields(fields.clone().take(5))
                    .color(serenity::Color::DARK_BLUE),
            )
            .components(vec![serenity::CreateActionRow::Buttons(vec![
                serenity::CreateButton::new(PREV)
                    .label("前のページ")
                    .disabled(page == 0),
                serenity::CreateButton::new(NEXT)
                    .label("次のページ")
                    .disabled(fields.len() <= 5),
            ])])
            .ephemeral(true))
    };

    interaction
        .create_response(
            &ctx,
            serenity::CreateInteractionResponse::Message(message(page)?),
        )
        .await?;

    log(
        &ctx,
        &interaction.user,
        format!(
            "{}さんがタスク一覧を確認しました",
            interaction.user.mention()
        ),
    )
    .await?;

    let mut interaction_stream = interaction
        .get_response(&ctx)
        .await?
        .await_component_interaction(&ctx)
        .timeout(Duration::from_secs(60 * 30))
        .stream();

    while let Some(interaction) = interaction_stream.next().await {
        match &interaction.data.custom_id[..] {
            PREV => {
                page = page.saturating_sub(1);
                interaction
                    .create_response(
                        &ctx,
                        serenity::CreateInteractionResponse::UpdateMessage(message(page)?),
                    )
                    .await?;
            }
            NEXT => {
                page += 1;
                interaction
                    .create_response(
                        &ctx,
                        serenity::CreateInteractionResponse::UpdateMessage(message(page)?),
                    )
                    .await?;
            }
            _ => {}
        }
    }
    Ok(())
}

async fn show_archived_tasks(
    interaction: serenity::ComponentInteraction,
    ctx: serenity::Context,
) -> Result<(), Error> {
    const PREV: &str = "prev";
    const NEXT: &str = "next";

    let mut page = 0;
    let message = |page: usize| -> Result<_, Error> {
        let tasks = load()?.tasks.lock().unwrap().clone();
        let fields = tasks
            .iter()
            .filter(|e| Local::now() > e.datetime)
            .sorted_by_key(|e| e.datetime)
            .map(|task| task.to_field())
            .skip(5 * page);

        Ok(serenity::CreateInteractionResponseMessage::new()
            .embed(
                serenity::CreateEmbed::default()
                    .title("過去のタスク一覧")
                    .description(if fields.len() == 0 {
                        "ありません"
                    } else {
                        ""
                    })
                    .fields(fields.clone().take(5).collect::<Vec<_>>())
                    .color(serenity::Color::DARK_BLUE),
            )
            .components(vec![serenity::CreateActionRow::Buttons(vec![
                serenity::CreateButton::new(PREV)
                    .label("前のページ")
                    .disabled(page == 0),
                serenity::CreateButton::new(NEXT)
                    .label("次のページ")
                    .disabled(fields.len() <= 5),
            ])])
            .ephemeral(true))
    };

    interaction
        .create_response(
            &ctx,
            serenity::CreateInteractionResponse::Message(message(page)?),
        )
        .await?;

    log(
        &ctx,
        &interaction.user,
        format!(
            "{}さんが過去のタスク一覧を確認しました",
            interaction.user.mention()
        ),
    )
    .await?;

    let mut interaction_stream = interaction
        .get_response(&ctx)
        .await?
        .await_component_interaction(&ctx)
        .timeout(Duration::from_secs(60 * 30))
        .stream();

    while let Some(interaction) = interaction_stream.next().await {
        match &interaction.data.custom_id[..] {
            PREV => {
                page = page.saturating_sub(1);
                interaction
                    .create_response(
                        &ctx,
                        serenity::CreateInteractionResponse::UpdateMessage(message(page)?),
                    )
                    .await?;
            }
            NEXT => {
                page += 1;
                interaction
                    .create_response(
                        &ctx,
                        serenity::CreateInteractionResponse::UpdateMessage(message(page)?),
                    )
                    .await?;
            }
            _ => {}
        }
    }
    Ok(())
}
