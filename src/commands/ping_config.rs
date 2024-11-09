use std::time::Duration;

use anyhow::{anyhow, Error};
use poise::serenity_prelude as serenity;
use serenity::{futures::StreamExt, Mentionable};

use crate::{save, Context};

#[poise::command(slash_command)]
/// タスク通知を送るチャンネルを設定します。
pub async fn set_ping_channel(ctx: Context<'_>) -> Result<(), Error> {
    ctx.data()
        .ping_channel
        .lock()
        .unwrap()
        .replace(ctx.channel_id());
    save(ctx.data())?;
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::default()
                .title("通知チャンネルを設定しました")
                .description(format!("{}", ctx.channel_id().mention()))
                .color(serenity::Color::DARK_BLUE),
        ),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
/// タスク通知を送るロールを設定します。
pub async fn set_ping_role(ctx: Context<'_>) -> Result<(), Error> {
    const SET_PING_ROLE: &str = "set_ping_role";

    let message = ctx
        .send(
            poise::CreateReply::default()
                .embed(
                    serenity::CreateEmbed::default()
                        .title("ロールを設定してください")
                        .color(serenity::Color::DARK_BLUE),
                )
                .components(vec![
                    serenity::CreateActionRow::SelectMenu(
                        serenity::CreateSelectMenu::new(
                            SET_PING_ROLE,
                            serenity::CreateSelectMenuKind::Role {
                                default_roles: None,
                            },
                        )
                        .placeholder("ロールを選択してください"),
                    ),
                    serenity::CreateActionRow::Buttons(vec![serenity::CreateButton::new("submit")
                        .label("送信")
                        .style(serenity::ButtonStyle::Primary)]),
                ]),
        )
        .await?;
    let mut interaction_stream = message
        .clone()
        .into_message()
        .await?
        .await_component_interaction(ctx)
        .timeout(Duration::from_secs(60 * 30))
        .stream();

    let mut select = None;
    let mut last_interaction = None;
    while let Some(interaction) = interaction_stream.next().await {
        match &interaction.data.kind {
            serenity::ComponentInteractionDataKind::RoleSelect { values } => {
                select.replace(values[0]);
                interaction
                    .create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
                    .await?;
            }
            serenity::ComponentInteractionDataKind::Button => {
                last_interaction.replace(interaction.clone());
                break;
            }
            _ => {}
        }
    }
    ctx.data()
        .ping_role
        .lock()
        .unwrap()
        .replace(select.ok_or(anyhow!("No role selected"))?);
    save(ctx.data())?;
    last_interaction
        .ok_or(anyhow!("No interaction"))?
        .create_response(
            &ctx,
            serenity::CreateInteractionResponse::UpdateMessage(
                serenity::CreateInteractionResponseMessage::default()
                    .embed(
                        serenity::CreateEmbed::default()
                            .title("ロールを設定しました")
                            .description(format!("{}", select.unwrap().mention()))
                            .color(serenity::Color::DARK_BLUE),
                    )
                    .components(vec![]),
            ),
        )
        .await?;
    Ok(())
}
