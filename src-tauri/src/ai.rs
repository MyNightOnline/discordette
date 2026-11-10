use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

fn t() -> bool { true }

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Model {
    pub id: String,
    #[serde(default)] pub ctx: Option<String>,
    #[serde(default)] pub vision: bool,
    #[serde(default = "t")] pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Provider {
    pub id: String,
    pub name: String,
    #[serde(default)] pub preset: Option<String>,
    pub base_url: String,
    /// "anthropic" (/v1/messages) | "chat" (/chat/completions)
    pub format: String,
    #[serde(default)] pub api_key: String,
    #[serde(default = "t")] pub enabled: bool,
    #[serde(default)] pub models: Vec<Model>,
}

pub const SYSTEM: &str = "คุณช่วยสรุปข้อความที่ผู้ใช้ให้มาเป็นโพสต์สำหรับ Discord \
ตอบเป็น JSON อย่างเดียว ไม่มีข้อความอื่น รูปแบบ {\"title\": \"...\", \"content\": \"...\"} \
title สั้นกระชับ ไม่เกิน 100 ตัวอักษร · content ไม่เกิน 2000 ตัวอักษร ใช้ Discord markdown ได้ (**ตัวหนา**, bullet) \
ใช้ภาษาเดียวกับต้นฉบับ ห้ามแต่งข้อมูลที่ไม่มีในต้นฉบับ \
สรุปเฉพาะประเด็นสำคัญ ตัดคำโปรย/คำเชิญชวนติดตาม/แฮชแท็ก (#...) ท้ายโพสต์ทิ้ง \
ถ้ามีลิงก์ ให้ใส่เป็น URL เต็มที่ขึ้นต้นด้วย https:// เสมอ (เช่น https://github.com/owner/repo) เพื่อให้กดได้ใน Discord ห้ามเขียนแค่ github.com/...";

/// prompt ระบบ: ถ้าห้องมีแท็ก ให้ AI เลือกจากรายการ (tags) และเสนอแท็กใหม่ได้เมื่อไม่มีอันไหนเหมาะ (new_tags)
pub fn system(tags: &[String]) -> String {
    if tags.is_empty() { return SYSTEM.into(); }
    format!("{SYSTEM} นอกจากนี้ให้เลือกแท็ก: ใส่ใน \"tags\" (array) เฉพาะชื่อที่มีในรายการนี้เท่านั้น ใช้ชื่อให้ตรงตัวอักษร ไม่เกิน 5 รายการ: {} ถ้าไม่มีแท็กไหนในรายการเหมาะกับเนื้อหาเลย ให้เสนอแท็กใหม่ใน \"new_tags\" (array, ไม่เกิน 2, ชื่อสั้นไม่เกิน 20 ตัวอักษร) ถ้ามีแท็กเหมาะอยู่แล้วให้ใส่ [] รูปแบบทั้งหมด {{\"title\": \"...\", \"content\": \"...\", \"tags\": [], \"new_tags\": []}}", tags.join(", "))
}

/// ยิงข้อความเดียวไปหา provider แล้วคืนข้อความตอบ (รวมเฉพาะส่วน text)
pub async fn call(http: &reqwest::Client, p: &Provider, model: &str, system: &str, user: &str, max_tokens: u32) -> Result<String, String> {
    let base = p.base_url.trim().trim_end_matches('/');
    if base.is_empty() { return Err("ยังไม่ได้ใส่ Base URL".into()); }
    if model.trim().is_empty() { return Err("ยังไม่ได้เลือกโมเดล".into()); }
    let anthropic = p.format == "anthropic";
    let req = if anthropic {
        let base = base.strip_suffix("/v1").unwrap_or(base);
        http.post(format!("{base}/v1/messages")).header("x-api-key", p.api_key.trim()).header("anthropic-version", "2023-06-01")
            .json(&json!({ "model": model, "max_tokens": max_tokens, "system": system, "messages": [{ "role": "user", "content": user }] }))
    } else {
        let r = http.post(format!("{base}/chat/completions"));
        let r = if p.api_key.trim().is_empty() { r } else { r.bearer_auth(p.api_key.trim()) };
        r.json(&json!({ "model": model, "max_tokens": max_tokens, "messages": [{ "role": "system", "content": system }, { "role": "user", "content": user }] }))
    };
    let res = req.send().await.map_err(|e| format!("เชื่อมต่อ AI ไม่ได้: {e}"))?;
    let status = res.status();
    let body = res.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() { return Err(format!("AI ตอบกลับ {status}: {}", body.chars().take(300).collect::<String>())); }
    let v: Value = serde_json::from_str(&body).map_err(|_| "อ่านคำตอบของ AI ไม่ได้ (ตรวจ Base URL และ API format)".to_string())?;
    Ok(if anthropic {
        v["content"].as_array().map(|a| a.iter().filter(|b| b["type"] == "text").filter_map(|b| b["text"].as_str()).collect::<Vec<_>>().join("")).unwrap_or_default()
    } else {
        v["choices"][0]["message"]["content"].as_str().unwrap_or_default().to_string()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_defaults() {
        let p: Provider = serde_json::from_str(r#"{"id":"a","name":"x","base_url":"u","format":"chat"}"#).unwrap();
        assert!(p.enabled && p.models.is_empty() && p.api_key.is_empty());
    }
}
