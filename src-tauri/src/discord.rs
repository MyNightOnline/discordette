use crate::err::ec;
use crate::config::Tag;
use crate::images::Staged;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};

pub const MAX_TITLE: usize = 100;
pub const MAX_CONTENT: usize = 2000;
pub const MAX_TAGS: usize = 5;
pub const IMAGES_PER_MSG: usize = 10;

const HOSTS: [&str; 4] = ["discord.com", "discordapp.com", "ptb.discord.com", "canary.discord.com"];

/// รับเฉพาะ https://{host}/api/webhooks/{id}/{token}
pub fn parse_webhook_url(url: &str) -> Result<(String, String), String> {
    let bad = || ec("err.badWebhook", &[]);
    let rest = url.trim().strip_prefix("https://").ok_or_else(bad)?;
    let mut it = rest.splitn(5, '/');
    let (host, api, hooks) = (it.next(), it.next(), it.next());
    let (id, token) = (it.next().ok_or_else(bad)?, it.next().ok_or_else(bad)?);
    let token = token.split(['?', '#']).next().unwrap_or("").trim_end_matches('/');
    let ok_part = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !HOSTS.contains(&host.unwrap_or("")) || api != Some("api") || hooks != Some("webhooks")
        || !id.chars().all(|c| c.is_ascii_digit()) || id.is_empty() || !ok_part(token)
    {
        return Err(bad());
    }
    Ok((id.to_string(), token.to_string()))
}

fn clean_url(url: &str) -> Result<String, String> {
    let (id, token) = parse_webhook_url(url)?;
    Ok(format!("https://discord.com/api/webhooks/{id}/{token}"))
}

/// ตรวจซ้ำฝั่ง Rust (pure) ก่อนยิงจริง
pub fn validate(
    forum: bool, title: &str, content: &str, tag_count: usize, require_tag: bool,
    image_sizes: &[u64], max_bytes: u64,
) -> Result<(), String> {
    let (t, c) = (title.trim().chars().count(), content.chars().count());
    if forum {
        if t == 0 || t > MAX_TITLE { return Err(ec("err.titleLen", &[&MAX_TITLE, &t])); }
        if c > MAX_CONTENT { return Err(ec("err.contentLong", &[&MAX_CONTENT, &c])); }
        if c == 0 && image_sizes.is_empty() { return Err(ec("err.needBody", &[])); }
        if require_tag && tag_count == 0 { return Err(ec("err.needTag", &[])); }
    } else {
        let total = compose_text(title, content).chars().count();
        if total > MAX_CONTENT { return Err(ec("err.totalLong", &[&MAX_CONTENT, &total])); }
        if total == 0 && image_sizes.is_empty() { return Err(ec("err.nothing", &[])); }
    }
    if tag_count > MAX_TAGS { return Err(ec("err.tagsMax", &[&MAX_TAGS])); }
    if let Some(i) = image_sizes.iter().position(|&s| s > max_bytes) {
        return Err(ec("err.imgOver", &[&(i + 1)]));
    }
    Ok(())
}

pub fn compose_text(title: &str, content: &str) -> String {
    match title.trim() {
        "" => content.to_string(),
        t => format!("**{t}**\n{content}"),
    }
}

pub fn build_payload(forum: bool, title: &str, content: &str, tag_ids: &[String]) -> Value {
    if forum {
        let mut v = json!({ "thread_name": title.trim(), "content": content });
        if !tag_ids.is_empty() { v["applied_tags"] = json!(tag_ids); }
        v
    } else {
        json!({ "content": compose_text(title, content) })
    }
}

pub fn map_error(status: u16, body: &Value) -> String {
    let code = body["code"].as_u64().unwrap_or(0);
    let msg = body["message"].as_str().unwrap_or("");
    match (status, code) {
        (401 | 404, _) | (_, 10015) => ec("err.webhookGone", &[]),
        (413, _) | (_, 40005 | 50045) => ec("err.fileTooBig", &[]),
        (_, 40067) => ec("err.needTag", &[]),
        (429, _) => ec("err.rateLimit", &[]),
        _ => ec("err.discordReject", &[&status, &code, &msg]),
    }
}

#[derive(Serialize)]
pub struct WebhookInfo { pub name: String, pub channel_id: String, pub guild_id: Option<String>, pub avatar_url: Option<String> }

pub async fn webhook_info(http: &reqwest::Client, url: &str) -> Result<WebhookInfo, String> {
    let r = http.get(clean_url(url)?).send().await.map_err(|x| ec("err.discordConnect", &[&x]))?;
    let status = r.status().as_u16();
    let v: Value = r.json().await.unwrap_or(Value::Null);
    if status != 200 { return Err(map_error(status, &v)); }
    let s = |k: &str| v[k].as_str().map(String::from);
    let avatar_url = match (s("id"), s("avatar")) {
        (Some(id), Some(a)) => Some(format!("https://cdn.discordapp.com/avatars/{id}/{a}.png?size=64")),
        _ => None,
    };
    Ok(WebhookInfo { name: s("name").unwrap_or_default(), channel_id: s("channel_id").unwrap_or_default(), guild_id: s("guild_id"), avatar_url })
}

/// เข้า Gateway ชั่วคราวให้บอทขึ้นสถานะออนไลน์ (การอ่านแท็กผ่าน REST ไม่ต้องใช้ แค่ให้เห็นว่าบอทกำลังทำงาน)
/// คืน connection; drop = ออฟไลน์ ล้มเหลวคืน None (ไม่กระทบการดึงแท็ก)
pub async fn go_online(token: &str) -> Option<impl Sized + Send + use<>> {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message;
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        let (mut ws, _) = tokio_tungstenite::connect_async("wss://gateway.discord.gg/?v=10&encoding=json").await.ok()?;
        ws.next().await?.ok()?; // op 10 Hello (อยู่ไม่กี่วินาที จึงไม่ต้อง heartbeat)
        let id = serde_json::json!({ "op": 2, "d": { "token": token, "intents": 0,
            "properties": { "os": "windows", "browser": "discordette", "device": "discordette" },
            "presence": { "status": "online", "afk": false, "activities": [] } } });
        ws.send(Message::text(id.to_string())).await.ok()?;
        while let Some(Ok(m)) = ws.next().await {
            if m.to_text().is_ok_and(|t| t.contains("\"t\":\"READY\"")) { return Some(ws); }
        }
        None
    }).await.ok().flatten()
}

#[derive(Serialize)]
pub struct ChannelTags { pub is_forum: bool, pub require_tag: bool, pub tags: Vec<Tag> }

pub async fn fetch_tags(http: &reqwest::Client, bot_token: &str, channel_id: &str) -> Result<ChannelTags, String> {
    let r = http.get(format!("https://discord.com/api/v10/channels/{channel_id}"))
        .header("Authorization", format!("Bot {bot_token}"))
        .send().await.map_err(|x| ec("err.discordConnect", &[&x]))?;
    let status = r.status().as_u16();
    let v: Value = r.json().await.unwrap_or(Value::Null);
    if status == 401 { return Err(ec("err.botToken", &[])); }
    if status == 403 || status == 404 { return Err(ec("err.botBlind", &[])); }
    if status != 200 { return Err(map_error(status, &v)); }
    Ok(parse_channel(&v))
}

/// สร้างแท็กใหม่ในห้องฟอรั่ม: Discord ให้ส่งรายการแท็กทั้งหมดกลับไปทั้งชุด (ต้องรวมของเดิมด้วย ไม่งั้นหาย) · ต้องมีสิทธิ์ Manage Channels
pub async fn create_tag(http: &reqwest::Client, bot_token: &str, channel_id: &str, name: &str) -> Result<ChannelTags, String> {
    let url = format!("https://discord.com/api/v10/channels/{channel_id}");
    let auth = format!("Bot {bot_token}");
    let cur: Value = http.get(&url).header("Authorization", &auth).send().await
        .map_err(|x| ec("err.discordConnect", &[&x]))?.json().await.unwrap_or(Value::Null);
    let mut tags = cur["available_tags"].as_array().cloned().unwrap_or_default();
    if tags.len() >= 20 { return Err(ec("err.tags20", &[])); }
    tags.push(serde_json::json!({ "name": name }));
    let r = http.patch(&url).header("Authorization", &auth).json(&serde_json::json!({ "available_tags": tags }))
        .send().await.map_err(|x| ec("err.discordConnect", &[&x]))?;
    let status = r.status().as_u16();
    let v: Value = r.json().await.unwrap_or(Value::Null);
    if status == 403 { return Err(ec("err.botNoManage", &[])); }
    if status != 200 { return Err(map_error(status, &v)); }
    Ok(parse_channel(&v))
}

pub fn parse_channel(v: &Value) -> ChannelTags {
    let tags = v["available_tags"].as_array().map(|a| {
        a.iter().filter_map(|t| Some(Tag {
            id: t["id"].as_str()?.to_string(),
            name: t["name"].as_str()?.to_string(),
            emoji: t["emoji_name"].as_str().map(String::from),
        })).collect()
    }).unwrap_or_default();
    ChannelTags { is_forum: v["type"].as_u64() == Some(15), require_tag: v["flags"].as_u64().unwrap_or(0) & 16 != 0, tags }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Sent {
    pub message_id: String, pub channel_id: String, pub guild_id: Option<String>, pub link: Option<String>,
    /// จำนวนไฟล์ที่ส่งสำเร็จแล้ว / ถูกยกเลิกกลางทางหรือไม่
    #[serde(default)] pub sent: usize, #[serde(default)] pub cancelled: bool,
}

/// ยิง 1 ข้อความ (ลองใหม่ 1 ครั้งเมื่อโดน rate limit) คืน JSON ของข้อความที่สร้าง
async fn post_once(http: &reqwest::Client, url: &str, payload: &str, images: &[Staged]) -> Result<Value, String> {
    for attempt in 0..2 {
        let mut form = reqwest::multipart::Form::new().text("payload_json", payload.to_string());
        for (i, im) in images.iter().enumerate() {
            let part = reqwest::multipart::Part::bytes(im.bytes.clone()).file_name(im.name.clone())
                .mime_str(&im.mime).map_err(|e| e.to_string())?;
            form = form.part(format!("files[{i}]"), part);
        }
        let r = http.post(url).multipart(form).send().await.map_err(|x| ec("err.sendFail", &[&x]))?;
        let status = r.status().as_u16();
        let v: Value = r.json().await.unwrap_or(Value::Null);
        if status == 429 && attempt == 0 {
            let wait = v["retry_after"].as_f64().unwrap_or(1.0).clamp(0.1, 30.0);
            tokio::time::sleep(std::time::Duration::from_secs_f64(wait)).await;
            continue;
        }
        if !(200..300).contains(&status) { return Err(map_error(status, &v)); }
        return Ok(v);
    }
    Err(map_error(429, &Value::Null))
}

/// รูปเกิน 10 → ชุดแรกไปกับโพสต์ ชุดที่เหลือส่งต่อเป็นข้อความถัดไป (ฟอรั่ม: ในเธรดที่เพิ่งสร้าง)
/// cancel ถูกเช็กก่อนส่งแต่ละชุด (ข้อความที่ส่งไปแล้วไม่ย้อนคืน); progress รับจำนวนไฟล์ที่ส่งแล้วสะสม
pub async fn send_post(
    http: &reqwest::Client, url: &str, forum: bool, title: &str, content: &str, tag_ids: &[String],
    images: &[Staged], room_guild: Option<&str>, cancel: &AtomicBool, progress: impl Fn(usize),
) -> Result<Sent, String> {
    let url = format!("{}?wait=true", clean_url(url)?);
    let mut chunks = images.chunks(IMAGES_PER_MSG);
    let payload = build_payload(forum, title, content, tag_ids).to_string();
    let first = chunks.next().unwrap_or(&[]);
    let v = post_once(http, &url, &payload, first).await?;
    let mut sent = first.len();
    progress(sent);
    let message_id = v["id"].as_str().unwrap_or_default().to_string();
    let channel_id = v["channel_id"].as_str().unwrap_or_default().to_string();
    let guild_id = v["guild_id"].as_str().map(String::from).or(room_guild.map(String::from));
    let link = guild_id.as_ref().map(|g| if forum {
        format!("https://discord.com/channels/{g}/{channel_id}")
    } else {
        format!("https://discord.com/channels/{g}/{channel_id}/{message_id}")
    });
    let rest_url = if forum { format!("{url}&thread_id={channel_id}") } else { url };
    let mut cancelled = false;
    for (n, chunk) in chunks.enumerate() {
        if cancel.load(Ordering::Relaxed) { cancelled = true; break; }
        post_once(http, &rest_url, "{}", chunk).await
            .map_err(|x| ec("err.partFail", &[&(n + 2), &x]))?;
        sent += chunk.len();
        progress(sent);
    }
    Ok(Sent { message_id, channel_id, guild_id, link, sent, cancelled })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_parse() {
        assert!(parse_webhook_url("https://discord.com/api/webhooks/123/abc-DEF_1").is_ok());
        assert!(parse_webhook_url(" https://ptb.discord.com/api/webhooks/123/abc?x=1 ").is_ok());
        for bad in ["http://discord.com/api/webhooks/1/a", "https://evil.com/api/webhooks/1/a",
            "https://discord.com.evil.com/api/webhooks/1/a", "https://discord.com/api/webhooks/x/a",
            "https://discord.com/api/webhooks/1", "https://discord.com/api/webhooks/1/", ""] {
            assert!(parse_webhook_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn validate_edges() {
        let ok = |t: &str, c: &str, tags, req, imgs: &[u64]| validate(true, t, c, tags, req, imgs, 100);
        assert!(ok(&"a".repeat(100), "", 0, false, &[1]).is_ok());
        assert!(ok(&"a".repeat(101), "x", 0, false, &[]).is_err());
        assert!(ok("", "x", 0, false, &[]).is_err());
        assert!(ok("t", &"a".repeat(2000), 0, false, &[]).is_ok());
        assert!(ok("t", &"a".repeat(2001), 0, false, &[]).is_err());
        assert!(ok("t", "x", 5, false, &[]).is_ok());
        assert!(ok("t", "x", 6, false, &[]).is_err());
        assert!(ok("t", "x", 0, true, &[]).is_err());
        assert!(ok("t", "x", 0, false, &[1; 25]).is_ok());
        assert!(ok("t", "x", 0, false, &[101]).is_err());
        assert!(ok("t", "", 0, false, &[]).is_err());
        // ไทยนับเป็นตัวอักษร ไม่ใช่ไบต์
        assert!(ok(&"ก".repeat(100), "x", 0, false, &[]).is_ok());
        // ห้องข้อความ: นับรวมหัวข้อ (**t**\n = 6 ตัว + 1)
        assert!(validate(false, "t", &"a".repeat(1994), 0, false, &[], 1).is_ok());
        assert!(validate(false, "t", &"a".repeat(1995), 0, false, &[], 1).is_err());
        assert!(validate(false, "", "", 0, false, &[], 1).is_err());
    }

    #[test]
    fn payloads() {
        let f = build_payload(true, " หัวข้อ ", "เนื้อหา", &["1".into(), "2".into()]);
        assert_eq!(f, json!({"thread_name":"หัวข้อ","content":"เนื้อหา","applied_tags":["1","2"]}));
        assert!(build_payload(true, "t", "c", &[]).get("applied_tags").is_none());
        assert_eq!(build_payload(false, "t", "c", &[]), json!({"content":"**t**\nc"}));
        assert_eq!(build_payload(false, "", "c", &[]), json!({"content":"c"}));
    }

    #[test]
    fn errors_and_channel() {
        assert!(map_error(404, &Value::Null).starts_with("err.webhookGone"));
        assert!(map_error(400, &json!({"code":40067})).starts_with("err.needTag"));
        assert!(map_error(413, &Value::Null).starts_with("err.fileTooBig"));
        assert!(map_error(400, &json!({"code":50035,"message":"bad"})).contains("bad"));
        let c = parse_channel(&json!({"type":15,"flags":16,"available_tags":[{"id":"9","name":"LINE","emoji_name":"💬"},{"id":"8","name":"x","emoji_name":null}]}));
        assert!(c.is_forum && c.require_tag && c.tags.len() == 2 && c.tags[0].emoji.as_deref() == Some("💬") && c.tags[1].emoji.is_none());
    }
}
