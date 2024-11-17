use std::iter;

use anyhow::{Context as _, Error};
use chrono::{Datelike, Duration, NaiveTime};
use chrono::{Local, NaiveDate};
use futures::StreamExt;
use itertools::Itertools;
use poise::serenity_prelude::*;
use poise::Modal;
use serde::{Deserialize, Serialize};

use crate::{save, Category, PartialTask, PoiseContext, Task};

#[poise::command(slash_command)]
/// タスクを追加します。
pub async fn add_task(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let (mut message, task) = create_task(
        ctx,
        CreateEmbed::default()
            .title("タスクを追加します".to_string())
            .color(Color::DARK_BLUE),
        PartialTask::default(),
        None,
    )
    .await?;

    ctx.data().tasks.lock().unwrap().insert(task.clone());
    save(ctx.data())?;
    message
        .edit(
            ctx,
            EditMessage::default()
                .embed(
                    CreateEmbed::default()
                        .title("タスクを追加しました")
                        .fields(vec![task.to_field()])
                        .color(Color::DARK_GREEN),
                )
                .components(vec![]),
        )
        .await?;
    Ok(())
}

#[poise::command(slash_command)]
/// タスクを削除します。
pub async fn remove_task(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let (last_interaction, task) = select_task(
        ctx,
        CreateEmbed::default()
            .title("削除するタスクを選択")
            .color(Color::DARK_BLUE),
    )
    .await?;

    {
        let mut tasks = ctx.data().tasks.lock().unwrap();
        tasks.remove(&task);
    }
    save(ctx.data())?;

    last_interaction
        .context("No interaction")?
        .create_response(
            ctx,
            CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::default()
                    .embed(
                        CreateEmbed::default()
                            .title("削除しました")
                            .fields(vec![task.to_field()])
                            .color(Color::DARK_RED),
                    )
                    .components(vec![]),
            ),
        )
        .await?;
    Ok(())
}

#[poise::command(slash_command)]
/// タスクを編集します。
pub async fn edit_task(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let (last_interaction, task) = select_task(
        ctx,
        CreateEmbed::default()
            .title("編集するタスクを選択")
            .color(Color::DARK_BLUE),
    )
    .await?;

    let (mut message, modified_task) = create_task(
        ctx,
        CreateEmbed::default()
            .title("タスクを編集します".to_string())
            .color(Color::DARK_BLUE),
        task.as_partial(),
        Some(last_interaction.context("No interaction")?),
    )
    .await?;

    {
        let mut tasks = ctx.data().tasks.lock().unwrap();
        tasks.remove(&task);
        tasks.insert(modified_task.clone());
    }
    save(ctx.data())?;

    message
        .edit(
            ctx,
            EditMessage::default()
                .embed(
                    CreateEmbed::default()
                        .title("タスクを編集しました")
                        .fields(vec![
                            task.to_field(),
                            ("↓".into(), "".into(), false),
                            modified_task.to_field(),
                        ])
                        .color(Color::DARK_GREEN),
                )
                .components(vec![]),
        )
        .await?;
    Ok(())
}

fn to_ja_weekday(date: String) -> String {
    date.replace("Sun", "日")
        .replace("Mon", "月")
        .replace("Tue", "火")
        .replace("Wed", "水")
        .replace("Thu", "木")
        .replace("Fri", "金")
        .replace("Sat", "土")
}

async fn create_task(
    ctx: PoiseContext<'_>,
    embed: CreateEmbed,
    defaults: PartialTask,
    interaction: Option<ComponentInteraction>,
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
                        to_ja_weekday(date.format("%Y/%m/%d (%a)").to_string()),
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
                        serde_json::to_string(t).unwrap(),
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
            CreateActionRow::SelectMenu(CreateSelectMenu::new(DATE, date_options).placeholder(
                task.date.map_or("日付".into(), |x| {
                    to_ja_weekday(x.format("%Y/%m/%d (%a)").to_string())
                }),
            )),
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

    let message = {
        if let Some(interaction) = interaction {
            interaction
                .create_response(
                    ctx,
                    CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::default()
                            .embed(embed)
                            .components(components(&defaults)),
                    ),
                )
                .await?;
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
        }
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
                interaction
                    .create_response(
                        &ctx,
                        CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::default()
                                .components(components(&task)),
                        ),
                    )
                    .await?;
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

    const YEAR: &str = "year";
    const MONTH: &str = "month";
    const DAY: &str = "day";

    #[derive(Serialize, Deserialize, Clone, Copy)]
    struct MonthHalf {
        month: u32,
        is_first_half: bool,
    }

    impl From<MonthHalf> for String {
        fn from(e: MonthHalf) -> Self {
            format!(
                "{}月{}",
                e.month,
                if e.is_first_half {
                    "前半(〜15)"
                } else {
                    "後半(16〜)"
                }
            )
        }
    }

    task.date = match task.clone().date {
        Some(date) => Some(date),
        None => {
            let mut date = Local::now().date_naive();

            let components = |date: NaiveDate| -> Result<_, Error> {
                let month = date.month();
                let is_first_half = date.day() <= 15;

                let year_options = CreateSelectMenuKind::String {
                    options: (Local::now().year()..=Local::now().year() + 2)
                        .map(|i| {
                            CreateSelectMenuOption::new(i.to_string(), i.to_string())
                                .default_selection(i == date.year())
                        })
                        .collect(),
                };
                let month_options = CreateSelectMenuKind::String {
                    options: (1..=12)
                        .flat_map(|i| {
                            [
                                MonthHalf {
                                    month: i,
                                    is_first_half: true,
                                },
                                MonthHalf {
                                    month: i,
                                    is_first_half: false,
                                },
                            ]
                        })
                        .map(|e| {
                            CreateSelectMenuOption::new(
                                String::from(e),
                                serde_json::to_string(&e).unwrap(),
                            )
                            .default_selection(month == e.month && is_first_half == e.is_first_half)
                        })
                        .collect(),
                };
                let day_options = CreateSelectMenuKind::String {
                    options: if is_first_half {
                        1..=15
                    } else {
                        16..=days_in_month(date.year(), month)?
                    }
                    .map(|i| {
                        CreateSelectMenuOption::new(i.to_string(), i.to_string())
                            .default_selection(i == date.day())
                    })
                    .collect(),
                };

                Ok(vec![
                    CreateActionRow::SelectMenu(
                        CreateSelectMenu::new(YEAR, year_options).placeholder("年"),
                    ),
                    CreateActionRow::SelectMenu(
                        CreateSelectMenu::new(MONTH, month_options).placeholder("月"),
                    ),
                    CreateActionRow::SelectMenu(
                        CreateSelectMenu::new(DAY, day_options).placeholder("日"),
                    ),
                    CreateActionRow::Buttons(vec![CreateButton::new(SUBMIT)
                        .style(ButtonStyle::Primary)
                        .label("送信")]),
                ])
            };

            let response = CreateInteractionResponse::UpdateMessage(
                CreateInteractionResponseMessage::default().components(components(date)?),
            );
            last_interaction
                .clone()
                .context("No interaction")?
                .create_response(ctx, response)
                .await?;

            fn days_in_month(year: i32, month: u32) -> Result<u32, Error> {
                // 次の月の1日から1日引くことで、その月の最終日を取得
                let next_month = if month == 12 { 1 } else { month + 1 };
                let next_year = if month == 12 { year + 1 } else { year };

                let last_day = NaiveDate::from_ymd_opt(next_year, next_month, 1)
                    .context("Invalid date")?
                    .pred_opt()
                    .context("Invalid date")?;

                Ok(last_day.day())
            }

            while let Some(interaction) = interaction_stream.next().await {
                match &interaction.data.kind {
                    ComponentInteractionDataKind::StringSelect { values } => {
                        match interaction.data.custom_id.as_str() {
                            YEAR => {
                                date = date
                                    .with_year(values[0].parse().unwrap())
                                    .context("Invalid date")?;
                                interaction
                                    .create_response(ctx, CreateInteractionResponse::Acknowledge)
                                    .await?;
                            }
                            MONTH => {
                                let selected_month: MonthHalf = serde_json::from_str(&values[0])?;
                                date = date
                                    .with_month(selected_month.month)
                                    .and_then(|d| {
                                        d.with_day(if selected_month.is_first_half {
                                            1
                                        } else {
                                            16
                                        })
                                    })
                                    .context("Invalid date")?;

                                let response = CreateInteractionResponse::UpdateMessage(
                                    CreateInteractionResponseMessage::default()
                                        .components(components(date)?),
                                );
                                interaction.create_response(ctx, response).await?;
                            }
                            DAY => {
                                date = date
                                    .with_day(values[0].parse().unwrap())
                                    .context("Invalid date")?;
                                interaction
                                    .create_response(ctx, CreateInteractionResponse::Acknowledge)
                                    .await?;
                            }
                            _ => {}
                        }
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

            Some(date)
        }
    };

    const HOUR: &str = "hour";
    const MINUTE: &str = "minute";

    task.time = match task.clone().time {
        Some(time) => Some(time),
        None => {
            let components = |selected_hour: Option<u32>, selected_minute: Option<u32>| {
                let hour_options = CreateSelectMenuKind::String {
                    options: (0..24)
                        .map(|i| {
                            CreateSelectMenuOption::new(i.to_string(), i.to_string())
                                .default_selection(selected_hour == Some(i))
                        })
                        .collect(),
                };
                let minute_options = CreateSelectMenuKind::String {
                    options: (0..60)
                        .step_by(5)
                        .chain(iter::once(59))
                        .map(|i| {
                            CreateSelectMenuOption::new(i.to_string(), i.to_string())
                                .default_selection(selected_minute == Some(i))
                        })
                        .collect(),
                };

                vec![
                    CreateActionRow::SelectMenu(
                        CreateSelectMenu::new(HOUR, hour_options).placeholder("時"),
                    ),
                    CreateActionRow::SelectMenu(
                        CreateSelectMenu::new(MINUTE, minute_options).placeholder("分"),
                    ),
                    CreateActionRow::Buttons(vec![CreateButton::new(SUBMIT)
                        .style(ButtonStyle::Primary)
                        .label("送信")
                        .disabled(selected_hour.is_none() || selected_minute.is_none())]),
                ]
            };

            last_interaction
                .clone()
                .context("No interaction")?
                .create_response(
                    ctx,
                    CreateInteractionResponse::UpdateMessage(
                        CreateInteractionResponseMessage::default()
                            .components(components(None, None)),
                    ),
                )
                .await?;

            let mut hour = None;
            let mut minute = None;

            while let Some(interaction) = interaction_stream.next().await {
                match &interaction.data.kind {
                    ComponentInteractionDataKind::StringSelect { values } => {
                        match interaction.data.custom_id.as_str() {
                            HOUR => {
                                hour.replace(values[0].parse().unwrap());
                                interaction
                                    .create_response(
                                        ctx,
                                        CreateInteractionResponse::UpdateMessage(
                                            CreateInteractionResponseMessage::default()
                                                .components(components(hour, minute)),
                                        ),
                                    )
                                    .await?;
                            }
                            MINUTE => {
                                minute.replace(values[0].parse().unwrap());
                                interaction
                                    .create_response(
                                        ctx,
                                        CreateInteractionResponse::UpdateMessage(
                                            CreateInteractionResponseMessage::default()
                                                .components(components(hour, minute)),
                                        ),
                                    )
                                    .await?;
                            }
                            _ => {}
                        }
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

            Some(
                NaiveTime::from_hms_opt(
                    hour.context("Hour not selected")?,
                    minute.context("Minute not selected")?,
                    0,
                )
                .context("Invalid datetime")?,
            )
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

async fn select_task(
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
                    .description(to_ja_weekday(
                        task.datetime.format("%Y/%m/%d (%a) %H:%M").to_string(),
                    ))
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
                interaction
                    .create_response(
                        &ctx,
                        CreateInteractionResponse::UpdateMessage(
                            CreateInteractionResponseMessage::default()
                                .components(components(page, &task)),
                        ),
                    )
                    .await?;
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
