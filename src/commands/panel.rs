use std::time::Duration;

use anyhow::{Context as _, Error};
use chrono::Local;
use itertools::Itertools;
use poise::serenity_prelude::*;
use {futures::StreamExt, Mentionable};

use crate::{load, save, PoiseContext};

const SHOW_TASKS: &str = "show_tasks";
const SHOW_ARCHIVED_TASKS: &str = "show_archived_tasks";

#[poise::command(slash_command)]
/// パネルをデプロイします。
pub async fn deploy_panel(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let message = ctx
        .channel_id()
        .send_message(
            ctx,
            CreateMessage::default()
                .embed(
                    CreateEmbed::default()
                        .title("タスク確認")
                        .description("ボタンを押すとタスクを確認できます")
                        .color(Color::BLUE),
                )
                .components(vec![CreateActionRow::Buttons(vec![
                    CreateButton::new(SHOW_TASKS)
                        .label("タスク一覧")
                        .style(ButtonStyle::Success),
                    CreateButton::new(SHOW_ARCHIVED_TASKS)
                        .label("過去のタスク一覧")
                        .style(ButtonStyle::Secondary),
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
    ctx.send(
        poise::CreateReply::default()
            .embed(
                CreateEmbed::default()
                    .title("パネルをデプロイしました")
                    .color(Color::DARK_GREEN),
            )
            .ephemeral(true),
    )
    .await?;
    Ok(())
}

pub async fn listen_panel_interactions(ctx: Context, msg: Message) -> Result<(), Error> {
    let mut interaction_stream = msg.await_component_interaction(&ctx).stream();
    while let Some(interaction) = interaction_stream.next().await {
        match interaction.data.custom_id.as_str() {
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

async fn log(ctx: &Context, user: &User, message: impl Into<String>) -> Result<(), Error> {
    let log_channel = *load()?.log_channel.lock().unwrap();

    log_channel
        .context("log channel not set")?
        .send_message(
            &ctx,
            CreateMessage::default().embed(
                CreateEmbed::default()
                    .thumbnail(user.avatar_url().unwrap_or_default())
                    .author(
                        CreateEmbedAuthor::new(user.name.clone())
                            .icon_url(user.avatar_url().unwrap_or_default()),
                    )
                    .title("パネル操作")
                    .timestamp(Local::now())
                    .description(message)
                    .color(Color::DARK_BLUE),
            ),
        )
        .await?;
    Ok(())
}

async fn show_tasks(interaction: ComponentInteraction, ctx: Context) -> Result<(), Error> {
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

        Ok(CreateInteractionResponseMessage::new()
            .embed(
                CreateEmbed::default()
                    .title("タスク一覧")
                    .description(if fields.len() == 0 {
                        "ありません！:tada:"
                    } else {
                        ""
                    })
                    .fields(fields.clone().take(5))
                    .color(Color::DARK_BLUE),
            )
            .components(vec![CreateActionRow::Buttons(vec![
                CreateButton::new(PREV)
                    .label("前のページ")
                    .style(ButtonStyle::Secondary)
                    .disabled(page == 0),
                CreateButton::new(NEXT)
                    .label("次のページ")
                    .style(ButtonStyle::Secondary)
                    .disabled(fields.len() <= 5),
            ])])
            .ephemeral(true))
    };

    interaction
        .create_response(&ctx, CreateInteractionResponse::Message(message(page)?))
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
        match interaction.data.custom_id.as_str() {
            PREV => {
                page = page.saturating_sub(1);
                interaction
                    .create_response(
                        &ctx,
                        CreateInteractionResponse::UpdateMessage(message(page)?),
                    )
                    .await?;
            }
            NEXT => {
                page += 1;
                interaction
                    .create_response(
                        &ctx,
                        CreateInteractionResponse::UpdateMessage(message(page)?),
                    )
                    .await?;
            }
            _ => {}
        }
    }
    Ok(())
}

async fn show_archived_tasks(interaction: ComponentInteraction, ctx: Context) -> Result<(), Error> {
    const PREV: &str = "prev";
    const NEXT: &str = "next";

    let mut page = 0;
    let message = |page: usize| -> Result<_, Error> {
        let tasks = load()?.tasks.lock().unwrap().clone();
        let fields = tasks
            .iter()
            .filter(|e| Local::now() > e.datetime)
            .sorted_by_key(|e| e.datetime)
            .rev()
            .map(|task| task.to_field())
            .skip(5 * page);

        Ok(CreateInteractionResponseMessage::new()
            .embed(
                CreateEmbed::default()
                    .title("過去のタスク一覧")
                    .description(if fields.len() == 0 {
                        "ありません"
                    } else {
                        ""
                    })
                    .fields(fields.clone().take(5).collect::<Vec<_>>())
                    .color(Color::DARK_BLUE),
            )
            .components(vec![CreateActionRow::Buttons(vec![
                CreateButton::new(PREV)
                    .label("前のページ")
                    .style(ButtonStyle::Secondary)
                    .disabled(page == 0),
                CreateButton::new(NEXT)
                    .label("次のページ")
                    .style(ButtonStyle::Secondary)
                    .disabled(fields.len() <= 5),
            ])])
            .ephemeral(true))
    };

    interaction
        .create_response(&ctx, CreateInteractionResponse::Message(message(page)?))
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
        match interaction.data.custom_id.as_str() {
            PREV => {
                page = page.saturating_sub(1);
                interaction
                    .create_response(
                        &ctx,
                        CreateInteractionResponse::UpdateMessage(message(page)?),
                    )
                    .await?;
            }
            NEXT => {
                page += 1;
                interaction
                    .create_response(
                        &ctx,
                        CreateInteractionResponse::UpdateMessage(message(page)?),
                    )
                    .await?;
            }
            _ => {}
        }
    }
    Ok(())
}
