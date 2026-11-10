use crate::err::ec;
use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, DynamicImage, ImageReader};
use serde::Serialize;
use std::{collections::HashMap, io::Cursor, sync::Mutex};

const MAX_SIDE: u32 = 4096;

pub struct Staged { pub name: String, pub mime: String, pub bytes: Vec<u8> }

#[derive(Default)]
pub struct Store { pub map: Mutex<HashMap<u64, Staged>>, pub next: Mutex<u64> }

#[derive(Serialize)]
pub struct StagedInfo { pub id: u64, pub size: u64, pub w: u32, pub h: u32, pub shrunk: bool, pub original_size: u64, pub name: String, pub video: bool, pub mime: String }

fn encode_jpeg(img: &DynamicImage, q: u8) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    JpegEncoder::new_with_quality(&mut out, q).encode_image(&img.to_rgb8()).map_err(|e| e.to_string())?;
    Ok(out)
}

/// ย่อรูปจนไม่เกิน max (ไบต์): ด้านยาว ≤4096 → JPEG q90→60 → ลดขนาดภาพลง 20% ทีละรอบ
pub fn shrink(bytes: &[u8], max: u64) -> Result<(Vec<u8>, u32, u32), String> {
    let mut img = image::load_from_memory(bytes).map_err(|x| ec("err.readImage", &[&x]))?;
    if img.width().max(img.height()) > MAX_SIDE { img = img.resize(MAX_SIDE, MAX_SIDE, FilterType::Triangle); }
    loop {
        for q in [90u8, 80, 70, 60] {
            let out = encode_jpeg(&img, q)?;
            if out.len() as u64 <= max { return Ok((out, img.width(), img.height())); }
        }
        let (w, h) = (img.width() * 4 / 5, img.height() * 4 / 5);
        if w < 16 || h < 16 { return Err(ec("err.shrinkFail", &[])); }
        img = img.resize_exact(w, h, FilterType::Triangle);
    }
}

const VIDEO_EXT: [(&str, &str); 5] = [("mp4", "video/mp4"), ("mov", "video/quicktime"), ("webm", "video/webm"), ("mkv", "video/x-matroska"), ("m4v", "video/x-m4v")];

/// ชนิดไฟล์ตามนามสกุล → (mime, เป็นคลิปไหม); None = ไม่ใช่รูป/คลิปที่รองรับ
pub fn ext_mime(name: &str) -> Option<(String, bool)> {
    let ext = name.rsplit_once('.')?.1.to_ascii_lowercase();
    let img = match ext.as_str() { "png" => "image/png", "jpg" | "jpeg" => "image/jpeg", "webp" => "image/webp", "gif" => "image/gif", _ => "" };
    if !img.is_empty() { return Some((img.into(), false)); }
    VIDEO_EXT.iter().find(|(e, _)| *e == ext).map(|(_, m)| (m.to_string(), true))
}

pub fn is_video(name: &str, mime: &str) -> bool { video_mime(name, mime).is_some() }

fn video_mime(name: &str, mime: &str) -> Option<String> {
    if mime.starts_with("video/") { return Some(mime.into()); }
    let ext = name.rsplit_once('.')?.1.to_ascii_lowercase();
    VIDEO_EXT.iter().find(|(e, _)| *e == ext).map(|(_, m)| m.to_string())
}

fn put(store: &Store, name: String, mime: String, bytes: Vec<u8>) -> u64 {
    let id = { let mut n = store.next.lock().unwrap(); *n += 1; *n };
    store.map.lock().unwrap().insert(id, Staged { name, mime, bytes });
    id
}

pub fn stage(store: &Store, raw: Vec<u8>, name: &str, mime: &str, max: u64) -> Result<StagedInfo, String> {
    let original_size = raw.len() as u64;
    // คลิป: ส่งตามต้นฉบับ ย่อไม่ได้ จึงเช็กเพดานอย่างเดียว
    if let Some(vm) = video_mime(name, mime) {
        if original_size > max { return Err(ec("err.videoBig", &[&(max / 1_000_000)])); }
        let id = put(store, name.to_string(), vm.clone(), raw);
        return Ok(StagedInfo { id, size: original_size, w: 0, h: 0, shrunk: false, original_size, name: name.to_string(), video: true, mime: vm });
    }
    let (w, h) = ImageReader::new(Cursor::new(&raw)).with_guessed_format().ok()
        .and_then(|r| r.into_dimensions().ok()).ok_or_else(|| ec("err.notImage", &[]))?;
    let (mut bytes, mut name, mut mime, mut shrunk, mut w, mut h) = (raw, name.to_string(), mime.to_string(), false, w, h);
    if original_size > max {
        if mime == "image/gif" { return Err(ec("err.gifBig", &[&(max / 1_000_000)])); }
        let (b, nw, nh) = shrink(&bytes, max)?;
        let stem = name.rsplit_once('.').map_or(name.as_str(), |(s, _)| s).to_string();
        (bytes, name, mime, shrunk, w, h) = (b, format!("{stem}.jpg"), "image/jpeg".into(), true, nw, nh);
    }
    let size = bytes.len() as u64;
    let id = put(store, name.clone(), mime.clone(), bytes);
    Ok(StagedInfo { id, size, w, h, shrunk, original_size, name, video: false, mime })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn noisy_png(w: u32, h: u32) -> Vec<u8> {
        let mut s = 12345u32;
        let img = image::RgbImage::from_fn(w, h, |_, _| {
            s = s.wrapping_mul(1664525).wrapping_add(1013904223);
            image::Rgb([(s >> 24) as u8, (s >> 16) as u8, (s >> 8) as u8])
        });
        let mut out = Vec::new();
        DynamicImage::ImageRgb8(img).write_to(&mut Cursor::new(&mut out), image::ImageFormat::Png).unwrap();
        out
    }

    #[test]
    fn small_passes_through_and_large_shrinks() {
        let store = Store::default();
        let png = noisy_png(400, 400);
        let a = stage(&store, png.clone(), "a.png", "image/png", u64::MAX).unwrap();
        assert!(!a.shrunk && a.size == png.len() as u64 && (a.w, a.h) == (400, 400));
        let cap = 30_000;
        assert!(png.len() as u64 > cap);
        let b = stage(&store, png, "b.png", "image/png", cap).unwrap();
        assert!(b.shrunk && b.size <= cap && b.name == "b.jpg");
        assert_eq!(store.map.lock().unwrap()[&b.id].bytes.len() as u64, b.size);
    }

    #[test]
    fn video_passes_through_untouched_and_respects_cap() {
        let store = Store::default();
        let v = stage(&store, vec![1; 100], "a.MOV", "image/png", 1000).unwrap();
        assert!(v.video && !v.shrunk && v.size == 100);
        assert_eq!(store.map.lock().unwrap()[&v.id].mime, "video/quicktime");
        assert!(stage(&store, vec![1; 100], "a.mp4", "video/mp4", 50).is_err());
    }

    #[test]
    fn bad_input_and_big_gif() {
        let store = Store::default();
        assert!(stage(&store, b"not an image".to_vec(), "x.png", "image/png", 10).is_err());
        let mut gif = Vec::new();
        DynamicImage::ImageRgb8(image::RgbImage::new(64, 64)).write_to(&mut Cursor::new(&mut gif), image::ImageFormat::Gif).unwrap();
        assert!(stage(&store, gif, "x.gif", "image/gif", 10).is_err());
    }
}
