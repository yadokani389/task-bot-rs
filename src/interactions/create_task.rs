use std::iter;

use anyhow::{Context as _, Error};
use chrono::{Duration, Local, NaiveDate, NaiveTime};
use futures::StreamExt;
use poise::serenity_prelude::*;
use poise::Modal;

use crate::interactions::select_date;
use crate::interactions::select_time;
use crate::utils::format_date;
use crate::{Category, PartialTask, PoiseContext, Task};

pub async fn create_task(
    ctx: PoiseContext<'_>,
    interaction: Option<ComponentInteraction>,
    embed: CreateEmbed,
    defaults: PartialTask,
) -> Result<(Message, Task), Error> {
    const CATEGORY: &str = "category";
    const SUBJECT: &str = "subject";
    const DATE: &str = "date";
    const TIME: &str = "time";
    const SUBMIT: &str = "submit";

    let subjects = ctx.data().subjects.lock().unwrap().clone();
    let suggest_times = ctx.data().suggest_times.lock().unwrap().clone();

    let components = |task: &PartialTask| {
        let category_options = CreateSelectMenuKind::String {
            options: Category::VALUES
                .iter()
                .map(|&c| {
                    CreateSelectMenuOption::new(c, c).default_selection(task.category == Some(c))
                })
                .collect(),
        };
        let subject_options = CreateSelectMenuKind::String {
            options: subjects
                .iter()
                .map(|s| {
                    CreateSelectMenuOption::new(s, s)
                        .default_selection(task.subject == Some(s.to_string()))
                })
                .collect(),
        };
        let date_options = CreateSelectMenuKind::String {
            options: (0..24)
                .map(|i| {
                    let date = Local::now().date_naive() + Duration::days(i);
                    CreateSelectMenuOption::new(
                        format_date(date),
                        serde_json::to_string(&Some(date)).unwrap(),
                    )
                    .default_selection(task.date == Some(date))
                })
                .chain(iter::once(
                    CreateSelectMenuOption::new(
                        "その他の日付",
                        serde_json::to_string(&None::<NaiveDate>).unwrap(),
                    )
                    .default_selection(task.date.is_none()),
                ))
                .collect(),
        };
        let time_options = CreateSelectMenuKind::String {
            options: suggest_times
                .iter()
                .map(|(t, l)| {
                    CreateSelectMenuOption::new(
                        format!("{} ({})", l, t.format("%H:%M")),
                        serde_json::to_string(&Some(t)).unwrap(),
                    )
                    .default_selection(task.time == Some(*t))
                })
                .chain(iter::once(
                    CreateSelectMenuOption::new(
                        "その他の時刻",
                        serde_json::to_string(&None::<NaiveTime>).unwrap(),
                    )
                    .default_selection(task.time.is_none()),
                ))
                .collect::<Vec<_>>(),
        };

        vec![
            CreateActionRow::SelectMenu(
                CreateSelectMenu::new(CATEGORY, category_options).placeholder("カテゴリー"),
            ),
            CreateActionRow::SelectMenu(
                CreateSelectMenu::new(SUBJECT, subject_options).placeholder("教科"),
            ),
            CreateActionRow::SelectMenu(
                CreateSelectMenu::new(DATE, date_options)
                    .placeholder(task.date.map_or("日付".into(), format_date)),
            ),
            CreateActionRow::SelectMenu(
                CreateSelectMenu::new(TIME, time_options).placeholder(
                    task.time
                        .map_or("時間".into(), |x| x.format("%H:%M").to_string()),
                ),
            ),
            CreateActionRow::Buttons(vec![CreateButton::new(SUBMIT)
                .style(ButtonStyle::Primary)
                .label("送信")
                .disabled(task.category.is_none() || task.subject.is_none())]),
        ]
    };

    let message = if let Some(interaction) = interaction {
        let response = CreateInteractionResponse::UpdateMessage(
            CreateInteractionResponseMessage::default()
                .embed(embed)
                .components(components(&defaults)),
        );
        interaction.create_response(ctx, response).await?;
        interaction.get_response(ctx).await?
    } else {
        ctx.send(
            poise::CreateReply::default()
                .embed(embed)
                .components(components(&defaults)),
        )
        .await?
        .into_message()
        .await?
    };

    let mut interaction_stream = message
        .clone()
        .await_component_interaction(ctx)
        .timeout(Duration::seconds(60 * 30).to_std()?)
        .stream();

    let mut task = defaults.clone();

    let mut last_interaction = None;
    while let Some(interaction) = interaction_stream.next().await {
        match &interaction.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => {
                match interaction.data.custom_id.as_str() {
                    CATEGORY => {
                        task.category.replace(values[0].clone().into());
                    }
                    SUBJECT => {
                        task.subject.replace(values[0].clone());
                    }
                    DATE => {
                        task.date = serde_json::from_str(&values[0])?;
                    }
                    TIME => {
                        task.time = serde_json::from_str(&values[0])?;
                    }
                    _ => {}
                }
                let response = CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::default().components(components(&task)),
                );
                interaction.create_response(&ctx, response).await?;
            }
            ComponentInteractionDataKind::Button => {
                if interaction.data.custom_id == SUBMIT {
                    last_interaction.replace(interaction);
                    break;
                }
            }
            _ => {}
        }
    }

    task.date = match task.clone().date {
        Some(date) => Some(date),
        None => {
            let (interaction, date) = select_date(
                ctx,
                message.clone(),
                last_interaction.clone().context("No interaction")?,
            )
            .await?;
            last_interaction.replace(interaction.context("No interaction")?);
            Some(date)
        }
    };

    task.time = match task.clone().time {
        Some(time) => Some(time),
        None => {
            let (interaction, time) = select_time(
                ctx,
                Some(last_interaction.clone().context("No interaction")?),
                None,
            )
            .await?;
            last_interaction.replace(interaction.context("No interaction")?);
            Some(time)
        }
    };

    #[derive(Modal)]
    #[name = "詳細入力"]
    struct DetailsModal {
        #[name = "詳細を入力してください"]
        #[placeholder = "詳細"]
        details: String,
    }

    let DetailsModal { details } = poise::execute_modal_on_component_interaction::<DetailsModal>(
        ctx,
        last_interaction.context("No interaction")?,
        defaults
            .clone()
            .details
            .map(|x| DetailsModal { details: x }),
        Some(Duration::seconds(60 * 30).to_std()?),
    )
    .await?
    .context("No interaction")?;
    task.details.replace(details);

    let task = task.to_task()?;

    Ok((message, task))
}
