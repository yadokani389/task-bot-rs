use std::{iter, time::Duration};

use anyhow::{Context as _, Error};

use chrono::{NaiveTime, Timelike};
use futures::StreamExt;
use poise::serenity_prelude::*;

use crate::{save, PoiseContext};

#[poise::command(slash_command)]
/// よく使う時間を追加します。
pub async fn add_suggest_time(
    ctx: PoiseContext<'_>,
    #[description = "よく使う時間のラベル(例: 1限開始時刻)"] label: String,
) -> Result<(), Error> {
    const HOUR: &str = "hour";
    const MINUTE: &str = "minute";
    const SUBMIT: &str = "submit";

    let hour_options = CreateSelectMenuKind::String {
        options: (0..24)
            .map(|hour| CreateSelectMenuOption::new(hour.to_string(), hour.to_string()))
            .collect(),
    };
    let minute_options = CreateSelectMenuKind::String {
        options: (0..60)
            .step_by(5)
            .chain(iter::once(59))
            .map(|minute| CreateSelectMenuOption::new(minute.to_string(), minute.to_string()))
            .collect(),
    };

    let message = ctx
        .send(
            poise::CreateReply::default()
                .embed(
                    CreateEmbed::default()
                        .title(format!("よく使う時間({})を追加", label))
                        .color(Color::DARK_BLUE),
                )
                .components(vec![
                    CreateActionRow::SelectMenu(
                        CreateSelectMenu::new(HOUR, hour_options).placeholder("時"),
                    ),
                    CreateActionRow::SelectMenu(
                        CreateSelectMenu::new(MINUTE, minute_options).placeholder("分"),
                    ),
                    CreateActionRow::Buttons(vec![CreateButton::new(SUBMIT).label("送信")]),
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

    let mut time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();

    while let Some(interaction) = interaction_stream.next().await {
        match &interaction.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => {
                match interaction.data.custom_id.as_str() {
                    HOUR => {
                        let hour = values[0].parse::<u32>()?;
                        time = time.with_hour(hour).context("Invalid hour")?;
                    }
                    MINUTE => {
                        let minute = values[0].parse::<u32>()?;
                        time = time.with_minute(minute).context("Invalid minute")?;
                    }
                    _ => {}
                }
                interaction
                    .create_response(ctx, CreateInteractionResponse::Acknowledge)
                    .await?;
            }
            ComponentInteractionDataKind::Button => {
                if interaction.data.custom_id == SUBMIT {
                    ctx.data()
                        .suggest_times
                        .lock()
                        .unwrap()
                        .insert(time, label.clone());
                    save(ctx.data())?;

                    let title = format!("{}({})を追加しました", label, time.format("%H:%M"));
                    let diff = format!(
                        "```diff\n{}\n```",
                        ctx.data()
                            .suggest_times
                            .lock()
                            .unwrap()
                            .iter()
                            .map(|(t, l)| format!(
                                "{}{}: {}",
                                if l == &label { "+ " } else { "" },
                                l,
                                t.format("%H:%M")
                            ))
                            .collect::<Vec<String>>()
                            .join("\n")
                    );

                    let response = CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::default()
                            .embed(
                                CreateEmbed::default()
                                    .title(title)
                                    .description(diff)
                                    .color(Color::DARK_GREEN),
                            )
                            .components(vec![]),
                    );

                    interaction.create_response(ctx, response).await?;
                    break;
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[poise::command(slash_command)]
/// よく使う時間を削除します。
pub async fn remove_suggest_time(ctx: PoiseContext<'_>) -> Result<(), Error> {
    const LABEL: &str = "label";
    const SUBMIT: &str = "submit";

    let suggest_times = ctx.data().suggest_times.lock().unwrap().clone();

    let suggest_time_options = CreateSelectMenuKind::String {
        options: suggest_times
            .iter()
            .map(|(t, l)| {
                CreateSelectMenuOption::new(
                    format!("{} ({})", l, t.format("%H:%M")),
                    serde_json::to_string(t).unwrap(),
                )
            })
            .collect(),
    };

    let message = ctx
        .send(
            poise::CreateReply::default()
                .embed(
                    CreateEmbed::default()
                        .title("よく使う時間を削除")
                        .color(Color::DARK_BLUE),
                )
                .components(vec![
                    CreateActionRow::SelectMenu(CreateSelectMenu::new(LABEL, suggest_time_options)),
                    CreateActionRow::Buttons(vec![CreateButton::new(SUBMIT).label("送信")]),
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

    let mut time = None;

    while let Some(interaction) = interaction_stream.next().await {
        match &interaction.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => {
                if interaction.data.custom_id == LABEL {
                    time = Some(serde_json::from_str(&values[0].clone())?);
                }
                interaction
                    .create_response(ctx, CreateInteractionResponse::Acknowledge)
                    .await?;
            }
            ComponentInteractionDataKind::Button => {
                if interaction.data.custom_id == SUBMIT {
                    if let Some(time) = time {
                        let title = format!(
                            "{}({})を削除しました",
                            suggest_times[&time],
                            time.format("%H:%M"),
                        );
                        let diff = format!(
                            "```diff\n{}\n```",
                            ctx.data()
                                .suggest_times
                                .lock()
                                .unwrap()
                                .iter()
                                .map(|(t, l)| format!(
                                    "{}{}: {}",
                                    if t == &time { "- " } else { "" },
                                    l,
                                    t.format("%H:%M")
                                ))
                                .collect::<Vec<String>>()
                                .join("\n")
                        );

                        let response = CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::default()
                                .embed(
                                    CreateEmbed::default()
                                        .title(title)
                                        .description(diff)
                                        .color(Color::DARK_GREEN),
                                )
                                .components(vec![]),
                        );

                        interaction.create_response(ctx, response).await?;
                        ctx.data().suggest_times.lock().unwrap().remove(&time);
                        save(ctx.data())?;
                    }
                    break;
                }
            }
            _ => {}
        }
    }
    Ok(())
}
