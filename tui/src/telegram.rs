//! Minimal Telegram Bot API client for `kontu watch` new-listing alerts. Only
//! the two calls we need: send a message, and discover the chat id from a message
//! the user sent the bot (so setup is "make a bot, message it, done").

use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};

const API: &str = "https://api.telegram.org";

fn http() -> Result<Client> {
    Ok(Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()?)
}

/// Send an HTML-formatted message. Link previews are left on so a listing URL
/// renders its cover photo inline in the chat.
pub async fn send_message(token: &str, chat_id: &str, html: &str) -> Result<()> {
    let resp: Value = http()?
        .post(format!("{API}/bot{token}/sendMessage"))
        .json(&json!({
            "chat_id": chat_id,
            "text": html,
            "parse_mode": "HTML",
            "disable_web_page_preview": false,
        }))
        .send()
        .await
        .context_telegram()?
        .json()
        .await?;
    if resp.get("ok").and_then(Value::as_bool) == Some(true) {
        Ok(())
    } else {
        Err(anyhow!(
            "telegram sendMessage failed: {}",
            resp.get("description").and_then(Value::as_str).unwrap_or("unknown error")
        ))
    }
}

/// Send a photo card with one inline "open" button — the best UX for a new-listing
/// alert: the cover image is the body, the caption carries the facts, and the
/// button deep-links into the web app. Falls back to a text+link message if
/// Telegram cannot fetch the photo (CDN hiccup, missing cover).
pub async fn send_photo_with_button(
    token: &str,
    chat_id: &str,
    photo_url: &str,
    caption_html: &str,
    button_text: &str,
    button_url: &str,
) -> Result<()> {
    let markup = json!({ "inline_keyboard": [[{ "text": button_text, "url": button_url }]] });
    let resp: Value = http()?
        .post(format!("{API}/bot{token}/sendPhoto"))
        .json(&json!({
            "chat_id": chat_id,
            "photo": photo_url,
            "caption": caption_html,
            "parse_mode": "HTML",
            "reply_markup": markup,
        }))
        .send()
        .await
        .context_telegram()?
        .json()
        .await?;
    if resp.get("ok").and_then(Value::as_bool) == Some(true) {
        return Ok(());
    }
    send_message_with_button(token, chat_id, caption_html, button_text, button_url).await
}

/// Text alert with one inline "open" button (the photo-less fallback / no-cover path).
pub async fn send_message_with_button(
    token: &str,
    chat_id: &str,
    html: &str,
    button_text: &str,
    button_url: &str,
) -> Result<()> {
    let markup = json!({ "inline_keyboard": [[{ "text": button_text, "url": button_url }]] });
    let resp: Value = http()?
        .post(format!("{API}/bot{token}/sendMessage"))
        .json(&json!({
            "chat_id": chat_id,
            "text": html,
            "parse_mode": "HTML",
            "disable_web_page_preview": false,
            "reply_markup": markup,
        }))
        .send()
        .await
        .context_telegram()?
        .json()
        .await?;
    if resp.get("ok").and_then(Value::as_bool) == Some(true) {
        Ok(())
    } else {
        Err(anyhow!(
            "telegram send failed: {}",
            resp.get("description").and_then(Value::as_str).unwrap_or("unknown error")
        ))
    }
}

/// Resolve the chat id from the most recent update sent to the bot. The user
/// messages the bot once, then this picks up the chat to deliver alerts to.
pub async fn detect_chat_id(token: &str) -> Result<String> {
    let resp: Value = http()?
        .get(format!("{API}/bot{token}/getUpdates"))
        .send()
        .await
        .context_telegram()?
        .json()
        .await?;
    if resp.get("ok").and_then(Value::as_bool) != Some(true) {
        let code = resp.get("error_code").and_then(Value::as_i64).unwrap_or(0);
        let desc = resp.get("description").and_then(Value::as_str).unwrap_or("");
        if code == 409 || desc.contains("webhook") {
            return Err(anyhow!(
                "this bot has a webhook set — remove it (deleteWebhook) or pass --chat-id explicitly"
            ));
        }
        return Err(anyhow!("telegram getUpdates rejected the token ({desc}) — double-check it"));
    }
    let updates = resp.get("result").and_then(Value::as_array).cloned().unwrap_or_default();
    updates
        .iter()
        .rev()
        .find_map(|u| {
            u.pointer("/message/chat/id")
                .or_else(|| u.pointer("/channel_post/chat/id"))
                .and_then(Value::as_i64)
        })
        .map(|id| id.to_string())
        .ok_or_else(|| {
            anyhow!("no message found — open Telegram, send your bot any message, then retry")
        })
}

/// Escape the five HTML-significant characters for Telegram's HTML parse mode.
pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

trait TelegramErr<T> {
    fn context_telegram(self) -> Result<T>;
}

impl<T> TelegramErr<T> for reqwest::Result<T> {
    fn context_telegram(self) -> Result<T> {
        self.map_err(|e| anyhow!("telegram request failed (network/proxy?): {e}"))
    }
}
