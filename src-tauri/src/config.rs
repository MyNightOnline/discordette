use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};
use tauri::Manager;

const SERVICE: &str = "discordette";
pub const DEFAULT_PROMPT: &str = include_str!("../../prompts/showcase-summarizer.md");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Tag { pub id: String, pub name: String, #[serde(default)] pub emoji: Option<String> }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Room {
    pub id: String,
    pub name: String,
    pub kind: String, // "forum" | "text"
    #[serde(default)] pub channel_id: Option<String>,
    #[serde(default)] pub guild_id: Option<String>,
    #[serde(default)] pub tags: Vec<Tag>,
    #[serde(default)] pub require_tag: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(default)] pub rooms: Vec<Room>,
    #[serde(default = "d_mb")] pub max_file_mb: u64,
    #[serde(default = "d_vmb")] pub max_video_mb: u64,
    #[serde(default = "d_theme")] pub theme: String,
    #[serde(default = "d_prompt")] pub prompt_template: String,
    #[serde(default)] pub providers: Vec<crate::ai::Provider>,
}
fn d_mb() -> u64 { 10 }
fn d_vmb() -> u64 { 25 }
fn d_theme() -> String { "system".into() }
fn d_prompt() -> String { DEFAULT_PROMPT.into() }
impl Default for Config {
    fn default() -> Self { Self { rooms: vec![], max_file_mb: d_mb(), max_video_mb: d_vmb(), theme: d_theme(), prompt_template: d_prompt(), providers: vec![] } }
}

pub fn dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let d = app.path().app_config_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d)
}

pub fn load(app: &tauri::AppHandle) -> Result<Config, String> {
    match std::fs::read_to_string(dir(app)?.join("config.json")) {
        Ok(s) => serde_json::from_str(&s).map_err(|e| format!("config.json เสีย: {e}")),
        Err(_) => Ok(Config::default()),
    }
}

pub fn save(app: &tauri::AppHandle, c: &Config) -> Result<(), String> {
    let s = serde_json::to_string_pretty(c).map_err(|e| e.to_string())?;
    std::fs::write(dir(app)?.join("config.json"), s).map_err(|e| e.to_string())
}

fn entry(key: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, key).map_err(|e| e.to_string())
}
pub fn secret_get(key: &str) -> Option<String> { entry(key).ok()?.get_password().ok() }
pub fn secret_set(key: &str, v: &str) -> Result<(), String> {
    entry(key)?.set_password(v).map_err(|e| format!("บันทึกลง keyring ไม่ได้: {e}"))
}
pub fn secret_del(key: &str) { if let Ok(e) = entry(key) { let _ = e.delete_credential(); } }
pub fn webhook_key(room_id: &str) -> String { format!("webhook:{room_id}") }

pub fn new_id() -> String {
    let n = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    format!("r{n:x}")
}

#[derive(Serialize, Deserialize)]
pub struct Export { pub version: u32, pub config: Config, pub webhooks: HashMap<String, String> }

pub fn export(app: &tauri::AppHandle, path: &str) -> Result<(), String> {
    let mut config = load(app)?;
    config.providers.iter_mut().for_each(|p| p.api_key.clear()); // ไม่ส่งออก API key
    let webhooks = config.rooms.iter().filter_map(|r| Some((r.id.clone(), secret_get(&webhook_key(&r.id))?))).collect();
    let s = serde_json::to_string_pretty(&Export { version: 1, config, webhooks }).map_err(|e| e.to_string())?;
    std::fs::write(path, s).map_err(|e| e.to_string())
}

pub fn read_export(path: &str) -> Result<Export, String> {
    let s = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&s).map_err(|_| "ไฟล์นี้ไม่ใช่ไฟล์ตั้งค่าของโปรแกรม".to_string())
}

pub fn conflicts(app: &tauri::AppHandle, path: &str) -> Result<Vec<String>, String> {
    let (cur, ex) = (load(app)?, read_export(path)?);
    Ok(ex.config.rooms.iter().filter(|r| cur.rooms.iter().any(|c| c.name == r.name)).map(|r| r.name.clone()).collect())
}

#[derive(Serialize, Default)]
pub struct ImportSummary { pub added: usize, pub replaced: usize, pub skipped: usize }

/// รวมห้องจากไฟล์ ห้องชื่อซ้ำ: replace=true แทนที่ (คง id เดิม) / false ข้าม
pub fn import(app: &tauri::AppHandle, path: &str, replace: bool) -> Result<ImportSummary, String> {
    let (mut cur, ex) = (load(app)?, read_export(path)?);
    let mut sum = ImportSummary::default();
    for mut room in ex.config.rooms {
        let hook = ex.webhooks.get(&room.id).cloned();
        if let Some(pos) = cur.rooms.iter().position(|c| c.name == room.name) {
            if !replace { sum.skipped += 1; continue; }
            room.id = cur.rooms[pos].id.clone();
            cur.rooms[pos] = room.clone();
            sum.replaced += 1;
        } else {
            if cur.rooms.iter().any(|c| c.id == room.id) { room.id = new_id(); }
            cur.rooms.push(room.clone());
            sum.added += 1;
        }
        if let Some(h) = hook { secret_set(&webhook_key(&room.id), &h)?; }
    }
    save(app, &cur)?;
    Ok(sum)
}
