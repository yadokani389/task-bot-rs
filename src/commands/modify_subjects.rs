use std::time::Duration;

use anyhow::{anyhow, Error};
use poise::serenity_prelude as serenity;
use serenity::futures::StreamExt;

use crate::{save, Context};

#[poise::command(slash_command)]
/// 教科を追加します。
pub async fn add_subjects(
    ctx: Context<'_>,
    #[description = "追加したい教科 / カンマ区切りで複数追加できます"] subjects: String,
) -> Result<(), Error> {
    let subjects = subjects
        .split(',')
        .map(|s| s.trim().to_string())
        .collect::<Vec<_>>();
    ctx.data()
        .subjects
        .lock()
        .unwrap()
        .extend(subjects.clone().into_iter());
    save(ctx.data())?;

    let diff = format!(
        "```diff\n{}\n```",
        ctx.data()
            .subjects
            .lock()
            .unwrap()
            .iter()
            .map(|s| format!("{}{}", if subjects.contains(s) { "+ " } else { "" }, s))
            .collect::<Vec<_>>()
            .join("\n")
    );

    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::default()
                .title("追加しました")
                .description(diff)
                .color(serenity::Color::DARK_GREEN),
        ),
    )
    .await?;
    Ok(())
}

#[poise::command(slash_command)]
/// 教科を削除します。
pub async fn remove_subject(ctx: Context<'_>) -> Result<(), Error> {
    const REMOVE_SUBJECT: &str = "remove_subject";
    const REMOVE_SUBJECT_CONFIRM: &str = "remove_subject_confirm";

    let subjects = ctx.data().subjects.lock().unwrap().clone();
    let subject_options = serenity::CreateSelectMenuKind::String {
        options: subjects
            .iter()
            .map(|s| serenity::CreateSelectMenuOption::new(s, s))
            .collect(),
    };
    let message = ctx
        .send(
            poise::CreateReply::default()
                .embed(
                    serenity::CreateEmbed::default()
                        .title("削除したい教科を選択してください")
                        .color(serenity::Color::DARK_BLUE),
                )
                .components(vec![
                    serenity::CreateActionRow::SelectMenu(serenity::CreateSelectMenu::new(
                        REMOVE_SUBJECT,
                        subject_options,
                    )),
                    serenity::CreateActionRow::Buttons(vec![serenity::CreateButton::new(
                        REMOVE_SUBJECT_CONFIRM,
                    )
                    .label("削除")
                    .style(serenity::ButtonStyle::Danger)]),
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
    while let Some(interaction) = interaction_stream.next().await {
        match &interaction.data.kind {
            serenity::ComponentInteractionDataKind::StringSelect { values, .. } => {
                select.replace(values[0].clone());
                save(ctx.data())?;
                interaction
                    .create_response(&ctx, serenity::CreateInteractionResponse::Acknowledge)
                    .await?;
            }
            serenity::ComponentInteractionDataKind::Button => {
                let subject = select.ok_or(anyhow!("Subject not selected"))?;
                let diff = format!(
                    "```diff\n{}\n```",
                    ctx.data()
                        .subjects
                        .lock()
                        .unwrap()
                        .iter()
                        .map(|s| format!("{}{}", if s == &subject { "- " } else { "" }, s))
                        .collect::<Vec<_>>()
                        .join("\n")
                );

                ctx.data()
                    .subjects
                    .lock()
                    .unwrap()
                    .retain(|s| s != &subject);
                save(ctx.data())?;

                let response = serenity::CreateInteractionResponse::UpdateMessage(
                    serenity::CreateInteractionResponseMessage::new().embed(
                        serenity::CreateEmbed::default()
                            .title("削除しました")
                            .description(diff)
                            .color(serenity::Color::DARK_GREEN),
                    ),
                );

                interaction.create_response(&ctx, response).await?;
                break;
            }
            _ => {}
        }
    }
    Ok(())
}
