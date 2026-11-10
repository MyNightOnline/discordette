import { api, errMsg, type Config, type FolderFile, type Room } from "./api.ts";
import { len, MAX_CONTENT, MAX_TAGS, MAX_TITLE, composeText, withSource } from "./limits.ts";

export type View = "compose" | "settings";
export type Sec = "general" | "discord" | "data" | "ai" | "prompt" | "about";
export interface Img { id: number; url: string; size: number; w: number; h: number; shrunk: boolean; original_size: number; name: string; video: boolean; mime: string }
export interface Slot { key: number; img?: Img; error?: string; name: string }

export const app = $state({
  config: { rooms: [], max_file_mb: 10, max_video_mb: 25, theme: "system", prompt_template: "", providers: [] } as Config,
  view: "compose" as View,
  sec: "general" as Sec,
  roomId: "",
  hasBot: false,
  toast: null as null | { msg: string; link?: string; err?: boolean },
});

const DRAFT_KEY = "draft.v1";
export const draft = $state({ title: "", content: "", source: "", tagIds: [] as string[], slots: [] as Slot[], suggest: [] as string[] });
let nextKey = 1;
export const newKey = () => nextKey++;

export const room = (): Room | undefined => app.config.rooms.find((r) => r.id === app.roomId);

export function setConfig(c: Config) {
  app.config = c;
  if (!c.rooms.some((r) => r.id === app.roomId)) app.roomId = c.rooms[0]?.id ?? "";
  applyTheme();
}

export function applyTheme() {
  const t = app.config.theme;
  const dark = t === "dark" || (t === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
}

export async function init() {
  try {
    const d = JSON.parse(localStorage.getItem(DRAFT_KEY) ?? "null");
    if (d) Object.assign(draft, { title: d.title ?? "", content: d.content ?? "", source: d.source ?? "", tagIds: d.tagIds ?? [] });
    app.roomId = localStorage.getItem("room") ?? "";
  } catch { /* ไม่มีร่างก็ไม่เป็นไร */ }
  loadMedia();
  try {
    setConfig(await api.getConfig());
    app.hasBot = await api.hasBotToken();
  } catch (e) { notify(errMsg(e), undefined, true); }
  matchMedia("(prefers-color-scheme: dark)").addEventListener("change", applyTheme);
}

export function saveDraft() {
  try {
    localStorage.setItem(DRAFT_KEY, JSON.stringify({ title: draft.title, content: draft.content, source: draft.source, tagIds: draft.tagIds }));
    localStorage.setItem("room", app.roomId);
  } catch { /* เก็บร่างไม่ได้ก็ข้าม */ }
}

export function notify(msg: string, link?: string, err = false) {
  app.toast = { msg, link, err };
  setTimeout(() => { if (app.toast?.msg === msg) app.toast = null; }, link ? 8000 : 5000);
}

export async function clearDraft() {
  for (const s of draft.slots) if (s.img) URL.revokeObjectURL(s.img.url);
  Object.assign(draft, { title: "", content: "", source: "", tagIds: [], slots: [], suggest: [] });
  await api.clearImages().catch(() => {});
  saveDraft();
}

/** รายการสิ่งที่ยังขาด/ผิด — field ใช้เลื่อนไปที่ช่องนั้น */
export function problems(): { field: string; msg: string }[] {
  const r = room(), p: { field: string; msg: string }[] = [];
  if (!r) return [{ field: "room", msg: "ยังไม่ได้เลือกห้อง" }];
  const forum = r.kind === "forum", t = len(draft.title.trim()), full = withSource(draft.content, draft.source), c = len(full);
  const imgs = draft.slots.filter((s) => s.img);
  if (forum) {
    if (t < 1 || t > MAX_TITLE) p.push({ field: "title", msg: t ? `หัวข้อยาวเกิน ${MAX_TITLE}` : "ยังไม่มีหัวข้อ" });
    if (c > MAX_CONTENT) p.push({ field: "content", msg: `รายละเอียดยาวเกิน ${MAX_CONTENT}` });
    if (!c && !imgs.length) p.push({ field: "content", msg: "ต้องมีรายละเอียดหรือรูป" });
    if (r.require_tag && !draft.tagIds.length) p.push({ field: "tags", msg: "ต้องเลือกแท็กอย่างน้อย 1" });
    if (draft.tagIds.length > MAX_TAGS) p.push({ field: "tags", msg: `แท็กได้ไม่เกิน ${MAX_TAGS}` });
  } else {
    const total = len(composeText(draft.title, full));
    if (total > MAX_CONTENT) p.push({ field: "content", msg: `ข้อความรวมหัวข้อยาวเกิน ${MAX_CONTENT}` });
    if (!total && !imgs.length) p.push({ field: "content", msg: "ยังไม่มีเนื้อหา" });
  }
  const failed = draft.slots.flatMap((s, i) => (s.error ? [i + 1] : []));
  if (failed.length) p.push({ field: "images", msg: `รูปที่ ${failed.join(", ")} แนบไม่สำเร็จ (ลบออกหรือลองใหม่)` });
  return p;
}

export const editor = $state({ open: false, room: undefined as Room | undefined });

/** ชนิดสื่อที่ผู้ใช้ติ๊กเลือกรับ (อย่างน้อย 1 อย่างเสมอ) */
const MEDIA_KEY = "media.v1";
export const media = $state({ image: true, video: true });
export function loadMedia() {
  try { Object.assign(media, JSON.parse(localStorage.getItem(MEDIA_KEY) ?? "{}")); } catch { /* ใช้ค่าเริ่มต้น */ }
  if (!media.image && !media.video) media.image = media.video = true;
}
export function toggleMedia(k: "image" | "video") {
  const other = k === "image" ? "video" : "image";
  if (media[k] && !media[other]) return;
  media[k] = !media[k];
  try { localStorage.setItem(MEDIA_KEY, JSON.stringify({ image: media.image, video: media.video })); } catch { /* ไม่เก็บก็ไม่เป็นไร */ }
}
export const mediaAccept = () => [media.image && "image/*", media.video && "video/*"].filter(Boolean).join(",");

const isImage = (f: File) => f.type.startsWith("image/") || /\.(png|jpe?g|webp|gif)$/i.test(f.name);
const isVideo = (f: File) => f.type.startsWith("video/") || /\.(mp4|mov|webm|mkv|m4v)$/i.test(f.name);
/** ไฟล์นี้ตรงกับชนิดที่ติ๊กไว้ไหม */
export const wanted = (f: File) => (media.image && isImage(f)) || (media.video && isVideo(f));

/** เพิ่มไฟล์: วางช่อง skeleton ก่อนแล้วค่อยเติมเมื่อ Rust ประมวลผลเสร็จ */
export async function addFiles(files: File[]) {
  const imgs = files.filter(wanted);
  if (!imgs.length) { notify(`ไม่พบไฟล์${media.image ? "รูป" : ""}${media.image && media.video ? "หรือ" : ""}${media.video ? "คลิป" : ""}ที่เลือกไว้`, undefined, true); return; }
  if (imgs.length < files.length) notify(`ข้ามไฟล์ที่ไม่ตรงกับที่เลือก ${files.length - imgs.length} ไฟล์`, undefined, true);
  // วางช่องทั้งหมดก่อน (เรียงตามลำดับเดิม) แล้วประมวลผลทีละ 4 ไฟล์ ไม่โหลดเข้าหน่วยความจำพร้อมกันหมด
  const jobs = imgs.map((f) => {
    draft.slots.push({ key: newKey(), name: f.name || "image" });
    return { f, s: draft.slots[draft.slots.length - 1] };
  });
  await pool(jobs, async (j) => {
    try { j.s.img = { ...(await api.stageImage(j.f)), url: URL.createObjectURL(j.f) }; } catch (e) { j.s.error = errMsg(e); }
  });
}

/** เพิ่มไฟล์จากพาธ (เลือกโฟลเดอร์): Rust อ่านจากดิสก์เอง ไม่ต้องผ่านหน้าเว็บ */
export async function addPaths(files: FolderFile[]) {
  const jobs = files.map((f) => {
    draft.slots.push({ key: newKey(), name: f.name });
    return { f, s: draft.slots[draft.slots.length - 1] };
  });
  await pool(jobs, async (j) => {
    try {
      const info = await api.stagePath(j.f.path);
      j.s.img = { ...info, url: URL.createObjectURL(new Blob([await api.getStaged(info.id)], { type: info.mime })) };
    } catch (e) { j.s.error = errMsg(e); }
  });
}

/** รันงานทีละ 4 ไม่โหลดทุกไฟล์เข้าหน่วยความจำพร้อมกัน */
async function pool<T>(items: T[], run: (x: T) => Promise<void>) {
  let next = 0;
  await Promise.all(Array.from({ length: Math.min(4, items.length) }, async () => {
    for (let x = items[next++]; x; x = items[next++]) await run(x);
  }));
}

export function removeSlot(key: number) {
  const i = draft.slots.findIndex((s) => s.key === key);
  if (i < 0) return;
  const [s] = draft.slots.splice(i, 1);
  if (s.img) { URL.revokeObjectURL(s.img.url); api.removeImage(s.img.id).catch(() => {}); }
}

export function moveSlot(from: number, to: number) {
  if (to < 0 || to >= draft.slots.length || from === to) return;
  const [s] = draft.slots.splice(from, 1);
  draft.slots.splice(to, 0, s);
}
