use anyhow::{Context as _, Error};
use chrono::Duration;
use futures::StreamExt;
use itertools::Itertools;
use poise::serenity_prelude::*;

use crate::{utils::format_datetime, PoiseContext, Task};

pub async fn select_task(
    ctx: PoiseContext<'_>,
    embed: CreateEmbed,
) -> Result<(Option<ComponentInteraction>, Task), Error> {
    const TASK: &str = "task";
    const SUBMIT: &str = "submit";
    const PREV: &str = "prev";
    const NEXT: &str = "next";

    let mut page = 0;
    let components = |page: usize, selected_task: &Option<Task>| {
        let options = ctx
            .data()
            .tasks
            .lock()
            .unwrap()
            .iter()
            .enumerate()
            .sorted_by_key(|(_, task)| task.datetime)
            .rev()
            .map(|(idx, task)| {
                CreateSelectMenuOption::new(task.to_field().0, idx.to_string())
                    .description(format_datetime(task.datetime))
                    .default_selection(selected_task.as_ref() == Some(task))
            })
            .skip(25 * page)
            .collect::<Vec<_>>();
        let task_options = CreateSelectMenuKind::String {
            options: options.clone().into_iter().take(25).collect(),
        };

        vec![
            CreateActionRow::SelectMenu(
                CreateSelectMenu::new(TASK, task_options).placeholder("タスク"),
            ),
            CreateActionRow::Buttons(vec![
                CreateButton::new(PREV)
                    .label("前のページ")
                    .style(ButtonStyle::Secondary)
                    .disabled(page == 0),
                CreateButton::new(NEXT)
                    .label("次のページ")
                    .style(ButtonStyle::Secondary)
                    .disabled(options.len() <= 25),
            ]),
            CreateActionRow::Buttons(vec![CreateButton::new(SUBMIT)
                .style(ButtonStyle::Primary)
                .label("送信")
                .disabled(selected_task.is_none())]),
        ]
    };

    let message = ctx
        .send(
            poise::CreateReply::default()
                .embed(embed)
                .components(components(page, &None)),
        )
        .await?;

    let mut interaction_stream = message
        .clone()
        .into_message()
        .await?
        .await_component_interaction(ctx)
        .timeout(Duration::seconds(60 * 30).to_std()?)
        .stream();

    let mut task: Option<Task> = None;
    let mut last_interaction = None;
    while let Some(interaction) = interaction_stream.next().await {
        match &interaction.data.kind {
            ComponentInteractionDataKind::StringSelect { values } => {
                if interaction.data.custom_id == TASK {
                    let tasks = ctx.data().tasks.lock().unwrap().clone();
                    task.replace(
                        tasks
                            .into_iter()
                            .nth(values[0].parse::<usize>().unwrap())
                            .context("Invalid task")?,
                    );
                }
                let response = CreateInteractionResponse::UpdateMessage(
                    CreateInteractionResponseMessage::default().components(components(page, &task)),
                );
                interaction.create_response(&ctx, response).await?;
            }
            ComponentInteractionDataKind::Button => match interaction.data.custom_id.as_str() {
                PREV => {
                    page = page.saturating_sub(1);
                    task = None;
                    let response = CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::default()
                            .components(components(page, &task)),
                    );
                    interaction.create_response(ctx, response).await?;
                }
                NEXT => {
                    page += 1;
                    task = None;
                    let response = CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::default()
                            .components(components(page, &task)),
                    );
                    interaction.create_response(ctx, response).await?;
                }
                SUBMIT => {
                    last_interaction.replace(interaction);
                    break;
                }
                _ => {}
            },
            _ => {}
        }
    }

    Ok((last_interaction, task.context("Task not selected")?))
}
