import type { Room } from "./api.ts";

export interface Imported {
  roomId: string; title: string; content: string; tagIds: string[]; imageOrder: number[];
  source: string; confidence: string; warnings: string[];
}

function lineOf(text: string, err: unknown): string {
  const m = String((err as Error).message);
  const line = m.match(/line (\d+)/)?.[1];
  if (line) return ` (บรรทัด ${line})`;
  const pos = m.match(/position (\d+)/)?.[1];
  if (pos) return ` (บรรทัด ${text.slice(0, +pos).split("\n").length})`;
  // V8 รุ่นใหม่ไม่บอกตำแหน่ง: เดาจากอักขระที่ไม่คาดคิดซึ่งตามหลัง : , { [
  const tok = m.match(/Unexpected token '(.)'/)?.[1];
  const at = tok ? text.search(new RegExp(`[:,{\\[]\\s*${tok.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}`)) : -1;
  return at >= 0 ? ` (ใกล้บรรทัด ${text.slice(0, text.indexOf(tok!, at)).split("\n").length})` : "";
}

/** แกะ JSON ออกจากข้อความที่ AI ตอบ (มี ```json ห่อหรือมีคำนำก็ได้) */
function parseObject(raw: string): { ok: true; j: any } | { ok: false; error: string } {
  let text = raw.trim();
  const fence = text.match(/```(?:json)?\s*([\s\S]*?)```/i);
  if (fence) text = fence[1].trim();
  else if (text.includes("{")) text = text.slice(text.indexOf("{"), text.lastIndexOf("}") + 1);
  let j: any;
  try { j = JSON.parse(text); } catch (e) { return { ok: false, error: `อ่าน JSON ไม่ได้${lineOf(text, e)} · รูปแบบไม่ถูกต้อง (ตรวจเครื่องหมาย , " { } [ ])` }; }
  if (!j || typeof j !== "object" || Array.isArray(j)) return { ok: false, error: "JSON ต้องเป็นออบเจกต์ { … }" };
  return { ok: true, j };
}

/** คำตอบของแท็บ AI: เอาแค่ title + content */
export function parseAi(raw: string): { ok: true; title: string; content: string; tags: string[]; newTags: string[] } | { ok: false; error: string } {
  const r = parseObject(raw);
  if (!r.ok) return r;
  const title = String(r.j.title ?? "").trim(), content = String(r.j.content ?? "").trim();
  const names = (v: unknown) => (Array.isArray(v) ? v.map((x) => String(x).trim()).filter(Boolean) : []);
  return title || content ? { ok: true, title, content, tags: names(r.j.tags), newTags: names(r.j.new_tags) } : { ok: false, error: "AI ไม่ได้ส่ง title/content มา" };
}

export function parseClaude(raw: string, rooms: Room[], currentRoomId: string): { ok: true; v: Imported } | { ok: false; error: string } {
  const p = parseObject(raw);
  if (!p.ok) return p;
  const j = p.j;

  const warnings: string[] = [];
  const named = rooms.find((r) => r.name === String(j.room ?? ""));
  if (!named) warnings.push(`ไม่พบห้อง "${j.room ?? ""}" ใช้ห้องที่เลือกอยู่แทน`);
  const room = named ?? rooms.find((r) => r.id === currentRoomId);
  const tagIds: string[] = [];
  const unknown: string[] = [];
  for (const n of Array.isArray(j.tags) ? j.tags : []) {
    const t = room?.tags.find((t) => t.name === String(n));
    if (t) { if (!tagIds.includes(t.id)) tagIds.push(t.id); } else unknown.push(String(n));
  }
  if (unknown.length) warnings.push(`แท็กที่ไม่มีในห้องนี้ (ตัดทิ้ง): ${unknown.join(", ")}`);
  const imageOrder = (Array.isArray(j.images) ? j.images : []).filter((n: unknown) => Number.isInteger(n) && (n as number) >= 0) as number[];
  return { ok: true, v: {
    roomId: room?.id ?? currentRoomId, title: String(j.title ?? ""), content: String(j.content ?? ""),
    tagIds, imageOrder, source: String(j.source_url ?? ""), confidence: String(j.confidence ?? ""), warnings,
  } };
}

/** แทน {{rooms}} / {{tags}} ด้วยของจริงในโปรแกรม */
export function fillPrompt(tpl: string, rooms: Room[]): string {
  return tpl
    .replace("{{rooms}}", rooms.map((r) => r.name).join(", ") || "(ยังไม่มีห้อง)")
    .replace("{{tags}}", rooms.filter((r) => r.kind === "forum" && r.tags.length).map((r) => `${r.name}: ${r.tags.map((t) => t.name).join(", ")}`).join("\n") || "(ไม่มี)");
}
