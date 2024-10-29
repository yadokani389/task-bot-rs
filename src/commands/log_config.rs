use anyhow::Error;

use poise::serenity_prelude as serenity;
use serenity::Mentionable;

use crate::{save, Context};

#[poise::command(slash_command)]
/// 管理者向けログを送るチャンネルを設定します。
pub async fn set_log_channel(ctx: Context<'_>) -> Result<(), Error> {
    ctx.data()
        .log_channel
        .lock()
        .unwrap()
        .replace(ctx.channel_id());
    save(ctx.data())?;
    ctx.send(
        poise::CreateReply::default().embed(
            serenity::CreateEmbed::default()
                .title("ログチャンネルを設定しました")
                .description(format!("{}", ctx.channel_id().mention()))
                .color(serenity::Color::DARK_BLUE),
        ),
    )
    .await?;
    Ok(())
}
