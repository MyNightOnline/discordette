mod ai;
mod config;
mod err;
mod discord;
mod history;
mod images;

use config::{Config, Room};
use err::ec;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{ipc::{InvokeBody, Request}, AppHandle, Emitter, State};

struct AppState { http: reqwest::Client, images: images::Store, cancel: AtomicBool }

type R<T> = Result<T, String>;

#[tauri::command]
fn get_config(app: AppHandle) -> R<Config> { config::load(&app) }

#[tauri::command]
fn save_settings(app: AppHandle, max_file_mb: u64, max_video_mb: u64, theme: String, prompt_template: String) -> R<Config> {
    let mut c = config::load(&app)?;
    c.max_file_mb = max_file_mb.clamp(1, 500);
    c.max_video_mb = max_video_mb.clamp(1, 500);
    c.theme = theme;
    c.prompt_template = prompt_template;
    config::save(&app, &c)?;
    Ok(c)
}

#[tauri::command]
fn save_providers(app: AppHandle, providers: Vec<ai::Provider>) -> R<Config> {
    let mut c = config::load(&app)?;
    c.providers = providers;
    config::save(&app, &c)?;
    Ok(c)
}

/// ให้ AI สรุปข้อความ: คืนข้อความดิบที่ AI ตอบ (หน้าเว็บแกะ JSON เอง)
#[tauri::command]
async fn ai_summarize(app: AppHandle, state: State<'_, AppState>, provider_id: String, model: String, text: String, extra: String, tags: Vec<String>) -> R<String> {
    let c = config::load(&app)?;
    let p = c.providers.iter().find(|p| p.id == provider_id && p.enabled).ok_or_else(|| ec("err.noProvider", &[]))?;
    let user = if extra.trim().is_empty() { text } else { format!("{text}

---
คำสั่งเพิ่มเติม: {}", extra.trim()) };
    let out = ai::call(&state.http, p, &model, &ai::system(&tags), &user, 4096).await?;
    if out.trim().is_empty() { return Err(ec("err.aiEmpty", &[])); }
    Ok(out)
}

/// ทดสอบการเชื่อมต่อด้วยค่าที่กำลังแก้ (ยังไม่ต้องบันทึก)
#[tauri::command]
async fn ai_test(state: State<'_, AppState>, provider: ai::Provider, model: String) -> R<()> {
    ai::call(&state.http, &provider, &model, "ตอบสั้นๆ", "ตอบว่า ok", 256).await.map(|_| ())
}

/// เพิ่ม/แก้ห้อง: webhook_url ว่าง = คงของเดิม (ใช้ตอนแก้)
#[tauri::command]
fn save_room(app: AppHandle, mut room: Room, webhook_url: Option<String>) -> R<Config> {
    let mut c = config::load(&app)?;
    if room.name.trim().is_empty() { return Err(ec("err.roomName", &[])); }
    if c.rooms.iter().any(|r| r.name == room.name && r.id != room.id) { return Err(ec("err.roomDup", &[])); }
    if room.id.is_empty() { room.id = config::new_id(); }
    match webhook_url.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        Some(u) => { discord::parse_webhook_url(u)?; config::secret_set(&config::webhook_key(&room.id), u)?; }
        None if config::secret_get(&config::webhook_key(&room.id)).is_none() => return Err(ec("err.webhookReq", &[])),
        None => {}
    }
    match c.rooms.iter_mut().find(|r| r.id == room.id) { Some(r) => *r = room, None => c.rooms.push(room) }
    config::save(&app, &c)?;
    Ok(c)
}

#[tauri::command]
fn delete_room(app: AppHandle, id: String) -> R<Config> {
    let mut c = config::load(&app)?;
    c.rooms.retain(|r| r.id != id);
    config::secret_del(&config::webhook_key(&id));
    config::save(&app, &c)?;
    Ok(c)
}

/// จัดลำดับห้องตาม ids (ห้องที่ไม่อยู่ในรายการต่อท้ายตามเดิม)
#[tauri::command]
fn reorder_rooms(app: AppHandle, ids: Vec<String>) -> R<Config> {
    let mut c = config::load(&app)?;
    c.rooms.sort_by_key(|r| ids.iter().position(|i| *i == r.id).unwrap_or(usize::MAX));
    config::save(&app, &c)?;
    Ok(c)
}

#[tauri::command]
fn get_webhook_url(room_id: String) -> Option<String> { config::secret_get(&config::webhook_key(&room_id)) }

#[tauri::command]
async fn webhook_info(state: State<'_, AppState>, url: String) -> R<discord::WebhookInfo> {
    discord::webhook_info(&state.http, &url).await
}

#[tauri::command]
async fn test_room(state: State<'_, AppState>, room_id: String) -> R<discord::WebhookInfo> {
    let url = config::secret_get(&config::webhook_key(&room_id)).ok_or_else(|| ec("err.noWebhook", &[]))?;
    discord::webhook_info(&state.http, &url).await
}

#[tauri::command]
fn set_bot_token(token: String) -> R<()> {
    if token.trim().is_empty() { config::secret_del("bot"); Ok(()) } else { config::secret_set("bot", token.trim()) }
}

#[tauri::command]
fn has_bot_token() -> bool { config::secret_get("bot").is_some() }

#[tauri::command]
fn get_bot_token() -> Option<String> { config::secret_get("bot") }

#[tauri::command]
async fn fetch_tags(app: AppHandle, state: State<'_, AppState>, room_id: String) -> R<Config> {
    let token = config::secret_get("bot").ok_or_else(|| ec("err.noBotToken", &[]))?;
    let mut c = config::load(&app)?;
    let room = c.rooms.iter_mut().find(|r| r.id == room_id).ok_or_else(|| ec("err.noRoom", &[]))?;
    let ch = room.channel_id.clone().filter(|s| !s.is_empty()).ok_or_else(|| ec("err.noChannel", &[]))?;
    let gw = discord::go_online(&token).await;
    let info = discord::fetch_tags(&state.http, &token, &ch).await;
    // ค้างออนไลน์อีกสักพักให้เห็นสถานะ แล้วค่อยตัดการเชื่อมต่อ (ไม่ต้องรอตรงนี้)
    tokio::spawn(async move { tokio::time::sleep(std::time::Duration::from_secs(8)).await; drop(gw); });
    let info = info?;
    if info.is_forum { room.kind = "forum".into(); room.tags = info.tags; room.require_tag = info.require_tag; }
    else { room.kind = "text".into(); room.tags.clear(); room.require_tag = false; }
    config::save(&app, &c)?;
    Ok(c)
}

#[tauri::command]
async fn create_tag(app: AppHandle, state: State<'_, AppState>, room_id: String, name: String) -> R<Config> {
    let name: String = name.trim().chars().take(20).collect();
    if name.is_empty() { return Err(ec("err.tagName", &[])); }
    let token = config::secret_get("bot").ok_or_else(|| ec("err.noBotToken", &[]))?;
    let mut c = config::load(&app)?;
    let room = c.rooms.iter_mut().find(|r| r.id == room_id).ok_or_else(|| ec("err.noRoom", &[]))?;
    let ch = room.channel_id.clone().filter(|s| !s.is_empty()).ok_or_else(|| ec("err.noChannel", &[]))?;
    let info = discord::create_tag(&state.http, &token, &ch, &name).await?;
    room.tags = info.tags;
    config::save(&app, &c)?;
    Ok(c)
}

/// รับไฟล์รูปเป็น raw body; header: name (URL-encoded), mime
/// (async) = รันบน thread pool ไม่ใช่ main thread: ถอดรหัส/ย่อรูปหนักๆ จะไม่ทำให้ UI ค้าง
#[tauri::command(async)]
fn stage_image(app: AppHandle, state: State<'_, AppState>, request: Request<'_>) -> R<images::StagedInfo> {
    let InvokeBody::Raw(raw) = request.body() else { return Err(ec("err.noFile", &[])) };
    let h = |k: &str| request.headers().get(k).and_then(|v| v.to_str().ok()).unwrap_or("").to_string();
    let name = percent_decode(&h("name"));
    let mime = match h("mime").as_str() { "" | "application/octet-stream" => "image/png".to_string(), m => m.to_string() };
    let cfg = config::load(&app)?;
    let max = if images::is_video(&name, &mime) { cfg.max_video_mb } else { cfg.max_file_mb } * 1_000_000;
    images::stage(&state.images, raw.clone(), &name, &mime, max)
}

#[derive(Serialize)]
struct FolderFile { path: String, name: String, video: bool }
#[derive(Serialize)]
struct FolderScan { files: Vec<FolderFile>, skipped: usize }

/// สแกนโฟลเดอร์ (รวมโฟลเดอร์ย่อย) หารูป/คลิปตามชนิดที่ติ๊ก; ไฟล์อื่นนับเป็น skipped
#[tauri::command(async)]
fn list_folder(path: String, image: bool, video: bool) -> R<FolderScan> {
    let root = std::path::PathBuf::from(&path);
    let mut scan = FolderScan { files: vec![], skipped: 0 };
    let mut stack = vec![root.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            if dir == root { return Err(ec("err.openDir", &[])); }
            continue;
        };
        for e in rd.flatten() {
            let Ok(t) = e.file_type() else { continue };
            if t.is_dir() { stack.push(e.path()); continue; }
            if !t.is_file() { continue; }
            let name = e.file_name().to_string_lossy().into_owned();
            match images::ext_mime(&name) {
                Some((_, v)) if (v && video) || (!v && image) => scan.files.push(FolderFile { path: e.path().to_string_lossy().into_owned(), name, video: v }),
                _ => scan.skipped += 1,
            }
        }
    }
    Ok(scan)
}

/// อ่านไฟล์จากพาธ (เฉพาะนามสกุลรูป/คลิปที่รองรับ) แล้ว stage เหมือน stage_image
#[tauri::command(async)]
fn stage_path(app: AppHandle, state: State<'_, AppState>, path: String) -> R<images::StagedInfo> {
    let name = std::path::Path::new(&path).file_name().and_then(|n| n.to_str()).unwrap_or("file").to_string();
    let (mime, video) = images::ext_mime(&name).ok_or_else(|| ec("err.badType", &[]))?;
    let cfg = config::load(&app)?;
    let max = if video { cfg.max_video_mb } else { cfg.max_file_mb } * 1_000_000;
    let len = std::fs::metadata(&path).map_err(|x| ec("err.readFile", &[&x]))?.len();
    if video && len > max { return Err(ec("err.videoBig", &[&(max / 1_000_000)])); }
    let raw = std::fs::read(&path).map_err(|x| ec("err.readFile", &[&x]))?;
    images::stage(&state.images, raw, &name, &mime, max)
}

/// ส่งไบต์ของไฟล์ที่ stage แล้วกลับไปให้หน้าเว็บทำ thumbnail (รูปที่ถูกย่อจะเล็กลงด้วย)
#[tauri::command]
fn get_staged(state: State<'_, AppState>, id: u64) -> R<tauri::ipc::Response> {
    let m = state.images.map.lock().unwrap();
    m.get(&id).map(|s| tauri::ipc::Response::new(s.bytes.clone())).ok_or_else(|| ec("err.noStaged", &[]))
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let (mut out, mut i) = (Vec::new(), 0);
    while i < b.len() {
        if let Some(Ok(v)) = (b[i] == b'%').then(|| s.get(i + 1..i + 3).map(|h| u8::from_str_radix(h, 16))).flatten() { out.push(v); i += 3; }
        else { out.push(b[i]); i += 1; }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[tauri::command]
fn remove_image(state: State<'_, AppState>, id: u64) { state.images.map.lock().unwrap().remove(&id); }

#[tauri::command]
fn clear_images(state: State<'_, AppState>) { state.images.map.lock().unwrap().clear(); }

#[derive(Serialize)]
struct PostResult { link: Option<String>, sent: usize, cancelled: bool }

/// ขอยกเลิกการส่ง: ข้อความที่ส่งไปแล้วคงอยู่ ชุดที่ยังไม่ได้ส่งจะถูกข้าม
#[tauri::command]
fn cancel_send(state: State<'_, AppState>) { state.cancel.store(true, Ordering::Relaxed); }

#[tauri::command]
async fn send_post(
    app: AppHandle, state: State<'_, AppState>, room_id: String, title: String, content: String,
    tag_ids: Vec<String>, image_ids: Vec<u64>, source_url: String,
) -> R<PostResult> {
    let c = config::load(&app)?;
    let room = c.rooms.iter().find(|r| r.id == room_id).ok_or_else(|| ec("err.noRoom", &[]))?;
    let url = config::secret_get(&config::webhook_key(&room.id)).ok_or_else(|| ec("err.noWebhook", &[]))?;
    let imgs: Vec<images::Staged> = {
        let m = state.images.map.lock().unwrap();
        image_ids.iter().map(|id| m.get(id).map(|s| images::Staged { name: s.name.clone(), mime: s.mime.clone(), bytes: s.bytes.clone() }).ok_or_else(|| ec("err.imgLost", &[])))
            .collect::<Result<_, _>>()?
    };
    let forum = room.kind == "forum";
    let sizes: Vec<u64> = imgs.iter().map(|i| i.bytes.len() as u64).collect();
    discord::validate(forum, &title, &content, tag_ids.len(), forum && room.require_tag, &sizes, u64::MAX)?;
    // เพดานแยกรูป/คลิป (เผื่อผู้ใช้ลดเพดานในตั้งค่าหลังแนบไฟล์ไปแล้ว)
    for (i, im) in imgs.iter().enumerate() {
        let (video, mb) = if images::is_video(&im.name, &im.mime) { (true, c.max_video_mb) } else { (false, c.max_file_mb) };
        if im.bytes.len() as u64 > mb * 1_000_000 { return Err(ec(if video { "err.overVideo" } else { "err.overImage" }, &[&(i + 1), &im.name, &mb])); }
    }
    state.cancel.store(false, Ordering::Relaxed);
    let total = imgs.len();
    let sent = discord::send_post(&state.http, &url, forum, &title, &content, &tag_ids, &imgs, room.guild_id.as_deref(), &state.cancel,
        |n| { let _ = app.emit("send-progress", serde_json::json!({ "sent": n, "total": total })); }).await?;
    let tag_names = tag_ids.iter().filter_map(|id| room.tags.iter().find(|t| &t.id == id).map(|t| t.name.clone())).collect();
    // ส่งสำเร็จแล้ว ประวัติพลาดไม่ควรทำให้ผู้ใช้เห็นว่าโพสต์ล้มเหลว
    let _ = history::push(&app, history::Entry {
        at: history::now_utc(), room_id: room.id.clone(), room_name: room.name.clone(), title, content,
        source_url, tags: tag_names, link: sent.link.clone(),
    });
    Ok(PostResult { link: sent.link, sent: sent.sent, cancelled: sent.cancelled })
}

#[tauri::command]
fn find_duplicate(app: AppHandle, room_id: String, title: String, source_url: String) -> Option<history::Entry> {
    history::find_duplicate(&history::load(&app), &room_id, &title, &source_url).cloned()
}

#[tauri::command]
fn export_config(app: AppHandle, path: String) -> R<()> { config::export(&app, &path) }

#[tauri::command]
fn import_conflicts(app: AppHandle, path: String) -> R<Vec<String>> { config::conflicts(&app, &path) }

#[tauri::command]
fn import_config(app: AppHandle, path: String, replace: bool) -> R<config::ImportSummary> { config::import(&app, &path, replace) }

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState { http: reqwest::Client::builder().user_agent("discordette/0.1")
            .connect_timeout(std::time::Duration::from_secs(10)).timeout(std::time::Duration::from_secs(180)).build().expect("http client"), images: Default::default(), cancel: AtomicBool::new(false) })
        .invoke_handler(tauri::generate_handler![
            get_config, save_settings, save_room, delete_room, reorder_rooms, get_webhook_url, webhook_info, test_room,
            set_bot_token, has_bot_token, get_bot_token, fetch_tags, create_tag, stage_image, list_folder, stage_path, get_staged, remove_image, clear_images, send_post, cancel_send,
            find_duplicate, export_config, import_conflicts, import_config, save_providers, ai_summarize, ai_test
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
