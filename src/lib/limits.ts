export const MAX_TITLE = 100, MAX_CONTENT = 2000, MAX_TAGS = 5;
export const len = (s: string) => [...s].length;
/** ห้องข้อความส่งเป็น **หัวข้อ**\nเนื้อหา — ต้องตรงกับ compose_text ใน Rust */
export const composeText = (title: string, content: string) => (title.trim() ? `**${title.trim()}**\n${content}` : content);
/** ต่อท้ายเนื้อหาด้วย [LINK](ลิงก์ต้นทาง) ถ้ามีลิงก์ */
export const withSource = (content: string, source: string) =>
  source.trim() ? `${content.trimEnd()}${content.trim() ? "\n\n" : ""}[LINK](${source.trim()})` : content;
export const tone = (n: number, max: number) => (n > max ? "danger" : n >= max * 0.9 ? "warn" : "");
