use std::time::Duration;

use anyhow::{anyhow, Error};

use chrono::{NaiveTime, Timelike};
use poise::serenity_prelude as serenity;
use serenity::futures::StreamExt;

use crate::{save, Context};

#[poise::command(slash_command)]
/// よく使う時間を追加します。
pub async fn add_suggest_time(
    ctx: Context<'_>,
    #[description = "よく使う時間のラベル(例: 1限開始時刻)"] label: String,
) -> Result<(), Error> {
    const HOUR: &str = "hour";
    const MINUTE: &str = "minute";
    const SUBMIT: &str = "submit";

    let hour_options = serenity::CreateSelectMenuKind::String {
        options: (0..24)
            .map(|hour| serenity::CreateSelectMenuOption::new(hour.to_string(), hour.to_string()))
            .collect(),
    };
    let minute_options = serenity::CreateSelectMenuKind::String {
        options: (0..60)
            .step_by(5)
            .map(|minute| {
                serenity::CreateSelectMenuOption::new(minute.to_string(), minute.to_string())
            })
            .collect(),
    };

    let message = ctx
        .send(
            poise::CreateReply::default()
                .embed(
                    serenity::CreateEmbed::default()
                        .title("よく使う時間を追加")
                        .color(serenity::Color::DARK_BLUE),
                )
                .components(vec![
                    serenity::CreateActionRow::SelectMenu(
                        serenity::CreateSelectMenu::new(HOUR, hour_options).placeholder("時"),
                    ),
                    serenity::CreateActionRow::SelectMenu(
                        serenity::CreateSelectMenu::new(MINUTE, minute_options).placeholder("分"),
                    ),
                    serenity::CreateActionRow::Buttons(vec![
                        serenity::CreateButton::new(SUBMIT).label("送信")
                    ]),
                ]),
        )
        .await?;

    let mut interaction_stream = message
        .clone()
        .into_message()
        .await?
        .await_component_interaction(ctx)
        .timeout(Duration::from_secs(60))
        .stream();

    let mut time = NaiveTime::from_hms_opt(0, 0, 0).unwrap();

    while let Some(interaction) = interaction_stream.next().await {
        match &interaction.data.kind {
            serenity::ComponentInteractionDataKind::StringSelect { values } => {
                match &interaction.data.custom_id[..] {
                    HOUR => {
                        let hour = values[0].parse::<u32>()?;
                        time = time.with_hour(hour).ok_or(anyhow!("Invalid hour"))?;
                    }
                    MINUTE => {
                        let minute = values[0].parse::<u32>()?;
                        time = time.with_minute(minute).ok_or(anyhow!("Invalid minute"))?;
                    }
                    _ => {}
                }
                interaction
                    .create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
                    .await?;
            }
            serenity::ComponentInteractionDataKind::Button => {
                if interaction.data.custom_id == SUBMIT {
                    ctx.data()
                        .suggest_times
                        .lock()
                        .unwrap()
                        .insert(label.clone(), time);
                    save(ctx.data())?;

                    let title = format!("{}({})を追加しました", label, time.format("%H:%M"));
                    let diff = format!(
                        "```diff\n{}\n```",
                        ctx.data()
                            .suggest_times
                            .lock()
                            .unwrap()
                            .iter()
                            .map(|(k, v)| format!(
                                "{}{}: {}",
                                if k == &label { "+ " } else { "" },
                                k,
                                v.format("%H:%M")
                            ))
                            .collect::<Vec<String>>()
                            .join("\n")
                    );

                    let response = serenity::CreateInteractionResponse::UpdateMessage(
                        serenity::CreateInteractionResponseMessage::default()
                            .embed(
                                serenity::CreateEmbed::default()
                                    .title(title)
                                    .description(diff)
                                    .color(serenity::Color::DARK_GREEN),
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
pub async fn remove_suggest_time(ctx: Context<'_>) -> Result<(), Error> {
    const LABEL: &str = "label";
    const SUBMIT: &str = "submit";

    let suggest_times = ctx.data().suggest_times.lock().unwrap().clone();

    let suggest_time_options = serenity::CreateSelectMenuKind::String {
        options: suggest_times
            .keys()
            .map(|label| serenity::CreateSelectMenuOption::new(label, label))
            .collect(),
    };

    let message = ctx
        .send(
            poise::CreateReply::default()
                .embed(
                    serenity::CreateEmbed::default()
                        .title("よく使う時間を削除")
                        .color(serenity::Color::DARK_BLUE),
                )
                .components(vec![
                    serenity::CreateActionRow::SelectMenu(serenity::CreateSelectMenu::new(
                        LABEL,
                        suggest_time_options,
                    )),
                    serenity::CreateActionRow::Buttons(vec![
                        serenity::CreateButton::new(SUBMIT).label("送信")
                    ]),
                ]),
        )
        .await?;

    let mut interaction_stream = message
        .clone()
        .into_message()
        .await?
        .await_component_interaction(ctx)
        .timeout(Duration::from_secs(60))
        .stream();

    let mut label = None;

    while let Some(interaction) = interaction_stream.next().await {
        match &interaction.data.kind {
            serenity::ComponentInteractionDataKind::StringSelect { values } => {
                if interaction.data.custom_id == LABEL {
                    label = Some(values[0].clone());
                }
                interaction
                    .create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
                    .await?;
            }
            serenity::ComponentInteractionDataKind::Button => {
                if interaction.data.custom_id == SUBMIT {
                    if let Some(label) = label {
                        let title = format!(
                            "{}({})を削除しました",
                            label,
                            suggest_times[&label].format("%H:%M")
                        );
                        let diff = format!(
                            "```diff\n{}\n```",
                            ctx.data()
                                .suggest_times
                                .lock()
                                .unwrap()
                                .iter()
                                .map(|(k, v)| format!(
                                    "{}{}: {}",
                                    if k == &label { "- " } else { "" },
                                    k,
                                    v.format("%H:%M")
                                ))
                                .collect::<Vec<String>>()
                                .join("\n")
                        );

                        let response = serenity::CreateInteractionResponse::UpdateMessage(
                            serenity::CreateInteractionResponseMessage::default()
                                .embed(
                                    serenity::CreateEmbed::default()
                                        .title(title)
                                        .description(diff)
                                        .color(serenity::Color::DARK_GREEN),
                                )
                                .components(vec![]),
                        );

                        interaction.create_response(ctx, response).await?;
                        ctx.data().suggest_times.lock().unwrap().remove(&label);
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
