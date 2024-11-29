use anyhow::{Context as _, Error};
use poise::serenity_prelude::*;

use crate::interactions::{create_task, select_task};
use crate::{save, PartialTask, PoiseContext};

#[poise::command(slash_command)]
/// タスクを追加します。
pub async fn add_task(ctx: PoiseContext<'_>) -> Result<(), Error> {
    let (mut message, task) = create_task(
        ctx,
        None,
        CreateEmbed::default()
            .title("タスクを追加します".to_string())
            .color(Color::DARK_BLUE),
        PartialTask::default(),
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

    let response = CreateInteractionResponse::UpdateMessage(
        CreateInteractionResponseMessage::default()
            .embed(
                CreateEmbed::default()
                    .title("削除しました")
                    .fields(vec![task.to_field()])
                    .color(Color::DARK_RED),
            )
            .components(vec![]),
    );
    last_interaction
        .context("No interaction")?
        .create_response(ctx, response)
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
        Some(last_interaction.context("No interaction")?),
        CreateEmbed::default()
            .title("タスクを編集します".to_string())
            .color(Color::DARK_BLUE),
        task.as_partial(),
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
