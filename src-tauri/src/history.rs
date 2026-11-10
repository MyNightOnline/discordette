use serde::{Deserialize, Serialize};

const MAX: usize = 1000;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Entry {
    pub at: String, // RFC3339-ish UTC "YYYY-MM-DDTHH:MM:SSZ"
    pub room_id: String,
    pub room_name: String,
    pub title: String,
    #[serde(default)] pub content: String,
    #[serde(default)] pub source_url: String,
    #[serde(default)] pub tags: Vec<String>,
    #[serde(default)] pub link: Option<String>,
}

fn norm(s: &str) -> String { s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase() }

/// หัวข้อซ้ำนับเฉพาะในห้องเดียวกัน ส่วนลิงก์ต้นทางซ้ำนับทุกห้อง
pub fn find_duplicate<'a>(list: &'a [Entry], room_id: &str, title: &str, source_url: &str) -> Option<&'a Entry> {
    let (t, u) = (norm(title), norm(source_url));
    list.iter().find(|e| (!t.is_empty() && e.room_id == room_id && norm(&e.title) == t) || (!u.is_empty() && norm(&e.source_url) == u))
}

pub fn load(app: &tauri::AppHandle) -> Vec<Entry> {
    crate::config::dir(app).ok()
        .and_then(|d| std::fs::read_to_string(d.join("history.json")).ok())
        .and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

/// ใหม่สุดอยู่หน้า สูงสุด 1000 รายการ
pub fn push(app: &tauri::AppHandle, e: Entry) -> Result<(), String> {
    let mut l = load(app);
    l.insert(0, e);
    l.truncate(MAX);
    let s = serde_json::to_string(&l).map_err(|e| e.to_string())?;
    std::fs::write(crate::config::dir(app)?.join("history.json"), s).map_err(|e| e.to_string())
}

pub fn now_utc() -> String {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as i64;
    let (days, rem) = (secs.div_euclid(86400), secs.rem_euclid(86400));
    // civil-from-days (Howard Hinnant)
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem % 3600 / 60, rem % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(title: &str, url: &str) -> Entry {
        Entry { at: String::new(), room_id: "r".into(), room_name: "n".into(), title: title.into(), content: String::new(), source_url: url.into(), tags: vec![], link: None }
    }

    #[test]
    fn duplicates() {
        let l = vec![e("Hello World", "https://x.com/a")];
        assert!(find_duplicate(&l, "r", "  hello   world ", "").is_some());
        assert!(find_duplicate(&l, "other-room", "hello world", "").is_none());
        assert!(find_duplicate(&l, "other-room", "x", " HTTPS://X.com/a ").is_some());
        assert!(find_duplicate(&l, "r", "other", "").is_none());
        assert!(find_duplicate(&l, "r", "", "").is_none());
    }

    #[test]
    fn clock_format() {
        let s = now_utc();
        assert_eq!(s.len(), 20);
        assert!(s.starts_with("20") && s.ends_with('Z'));
    }
}
