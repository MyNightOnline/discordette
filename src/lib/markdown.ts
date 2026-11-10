const MAP: Record<string, string> = { "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" };
const esc = (s: string) => s.replace(/[&<>"']/g, (c) => MAP[c]);

/** markdown ชุดเล็กของ Discord: escape ก่อนเสมอ แล้วค่อยใส่ tag ที่เราสร้างเอง */
export function md(src: string): string {
  const stash: string[] = [];
  const hold = (h: string) => `\u0000${stash.push(h) - 1}\u0000`;
  let s = esc(src)
    .replace(/```(?:[\w+-]*\n)?([\s\S]*?)```/g, (_, c) => hold(`<pre><code>${c.replace(/\n$/, "")}</code></pre>`))
    .replace(/`([^`\n]+)`/g, (_, c) => hold(`<code>${c}</code>`));
  // ลิงก์: เก็บแท็กไว้ก่อน กัน _ หรือ * ใน URL ถูกตีเป็นตัวเอียง
  s = s.replace(/\[([^\]\n]+)\]\((https?:\/\/[^\s)]+)\)/g, (_, t, u) => hold(`<a href="${u}" target="_blank" rel="noreferrer">`) + t + hold("</a>"));
  let multi = false;
  s = s.split("\n").map((l) => {
    let m;
    if ((m = l.match(/^&gt;&gt;&gt; ?(.*)/))) { multi = true; return `\u0001${m[1]}`; }
    if (multi) return `\u0001${l}`;
    if ((m = l.match(/^#{1,3} (.+)/))) return `**${m[1]}**`;
    if ((m = l.match(/^[-*] (.+)/))) return `• ${m[1]}`;
    if ((m = l.match(/^&gt; ?(.*)/))) return `\u0001${m[1]}`;
    return l;
  }).join("\n");
  s = s
    .replace(/\*\*\*([^*\n]+)\*\*\*/g, "<b><i>$1</i></b>")
    .replace(/\*\*([^*\n]+)\*\*/g, "<b>$1</b>")
    .replace(/\|\|([^\n]+?)\|\|/g, '<span class="sp">$1</span>')
    .replace(/__([^_\n]+)__/g, "<u>$1</u>")
    .replace(/~~([^~\n]+)~~/g, "<s>$1</s>")
    .replace(/(^|[^*])\*([^*\n]+)\*/g, "$1<i>$2</i>")
    .replace(/(^|[^\w_])_([^_\n]+)_(?![\w_])/g, "$1<i>$2</i>")
    .replace(/\u0001([^\n]*)\n?/g, '<span class="q">$1</span>')
    .replace(/\n/g, "<br>");
  return s.replace(/\u0000(\d+)\u0000/g, (_, i) => stash[+i]).replace(/<\/pre><br>/g, "</pre>");
}
