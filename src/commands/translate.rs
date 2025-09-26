use core::time;
use std::{
    process::Output,
    thread::{Builder, sleep},
};

use crate::{Data, commands::translate};
use ::serenity::{
    all::{Channel, ChannelId, GetMessages},
    futures::{StreamExt, TryFutureExt},
};
use poise::serenity_prelude as serenity;
type Error = Box<dyn std::error::Error + Send + Sync>;
type Context<'a> = poise::Context<'a, Data, Error>;
use crate::api::translate_api::{Language, translate_url};
use serenity::builder::{CreateEmbed, CreateMessage};

fn remove_emojis(s: &str) -> String {
    s.chars()
        .filter(|c| {
            match c {
            '\u{2600}'..='\u{26FF}' | // Miscellaneous Symbols
            '\u{2700}'..='\u{27BF}' | // Dingbats
            '\u{1F600}'..='\u{1F64F}' | // Emoticons
            '\u{1F680}'..='\u{1F6FF}' | // Transport and Map Symbols
            '\u{1F300}'..='\u{1F5FF}' | // Miscellaneous Symbols and Pictographs
            '\u{1F900}'..='\u{1F9FF}' | // Supplemental Symbols and Pictographs
            '\u{1FA70}'..='\u{1FAFF}' | // Symbols and Pictographs Extended-A
            '\u{1F7E0}'..='\u{1F7FF}' => false, // Geometric Shapes Extended
            _ => true,
        }
        })
        .collect()
}

#[poise::command(slash_command, track_edits, prefix_command)]
pub async fn translate(
    ctx: Context<'_>,
    #[description = "Select Input Language"] language_input: String,
    #[description = "Select Output Language"] language_output: String,
    #[description = "Input Text"] text: Option<String>,
) -> Result<(), Error> {
    let source = Language::from(language_input).unwrap();
    let target = Language::from(language_output).unwrap();
    let channel_id = ctx.channel_id();

    let mut messages = channel_id
        .messages(&ctx.http(), GetMessages::new().limit(2))
        .await?;

    let prev_message = &messages.get(0).unwrap().content;

    let data = translate_url(
        source,
        target,
        text.unwrap_or(prev_message.to_string()),
        "http://localhost:5050/".to_string(),
        None,
    )
    .await
    .unwrap();

    let response = data.output.to_string();
    println!("{}", data.output);

    ctx.say(response).await.unwrap();

    Ok(())
}

#[poise::command(slash_command, track_edits, prefix_command)]
pub async fn tickettranslate(
    ctx: Context<'_>,
    #[description = "Select Channel"] channel: Option<Channel>,
    #[description = "Select Input Language"] language_input: String,
    #[description = "Select Output Language"] language_output: String,
) -> Result<(), Error> {
    ctx.defer().await?;

    let source = Language::from(language_input).unwrap();
    let target = Language::from(language_output).unwrap();

    let channel_id = channel.map_or(ctx.channel_id(), |c| c.id());

    let mut messages_to_send = String::new();

    // Use messages_iter() on the resolved channel_id.
    // Use Box::pin to "pin" the stream to the heap, which is required for it to be awaited.
    let stream = Box::pin(channel_id.messages_iter(ctx.http()));

    let mut messages: Vec<_> = stream.collect().await;

    messages.reverse();

    let mut message_count = 0;

    for message_result in messages {
        let message = match message_result {
            Ok(msg) => msg,
            Err(e) => {
                println!("Error fetching message: {}", e);
                continue;
            }
        };

        if message.content.is_empty() {
            continue;
        }

        let cleaned = remove_emojis(&message.content);

        let translate_result = translate_url(
            source,
            target,
            cleaned,
            "http://localhost:5050/".to_string(),
            None,
        )
        .await;

        let translated_text = match translate_result {
            Ok(data) => data.output,
            Err(e) => {
                let error_message = format!("Error translating message: {}", e);
                ctx.say(&error_message).await?;
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    error_message,
                )));
            }
        };

        let formatted_message = format!("{}: {}\n", message.author.name, translated_text);

        if messages_to_send.len() + formatted_message.len() > 1950 {
            ctx.say(format!("```\n{}\n```", messages_to_send)).await?;
            messages_to_send.clear();
        }

        messages_to_send.push_str(&formatted_message);
        message_count += 1;
    }

    if !messages_to_send.is_empty() {
        ctx.say(format!(
            " <#{}> \n``` \n{}\n```",
            channel_id, messages_to_send
        ))
        .await?;
    }
    Ok(())
}
