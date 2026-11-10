import { test } from "node:test";
import assert from "node:assert/strict";
import { parseAi, parseClaude } from "./jsonImport.ts";
import { md } from "./markdown.ts";

const rooms: any[] = [{ id: "a", name: "showcase", kind: "forum", tags: [{ id: "1", name: "LINE" }, { id: "2", name: "AI" }] }];

test("fenced and bare JSON, unknown tags dropped, bad room falls back", () => {
  const body = '{"room":"showcase","title":"t","content":"c","tags":["LINE","nope"],"images":[2,0,-1],"confidence":"low"}';
  for (const raw of ["```json\n" + body + "\n```", "นี่ครับ\n" + body]) {
    const r = parseClaude(raw, rooms, "a");
    assert.ok(r.ok);
    if (r.ok) {
      assert.deepEqual(r.v.tagIds, ["1"]);
      assert.deepEqual(r.v.imageOrder, [2, 0]);
      assert.equal(r.v.confidence, "low");
      assert.equal(r.v.warnings.length, 1);
    }
  }
  const r = parseClaude('{"room":"x","title":"t"}', rooms, "a");
  assert.ok(r.ok && r.v.roomId === "a" && r.v.warnings.length === 1);
});

test("bad JSON reports an error with line", () => {
  const r = parseClaude('{\n"a": 1,\n"b": }', rooms, "a");
  assert.ok(!r.ok && r.error.includes("บรรทัด"));
});

test("markdown escapes html", () => {
  assert.equal(md("<script>**x**</script>"), "&lt;script&gt;<b>x</b>&lt;/script&gt;");
  assert.ok(!md("[a](javascript:alert(1))").includes("<a"));
  assert.equal(md("`**x**`"), "<code>**x**</code>");
  assert.equal(md("~~a~~ __b__ _c_"), "<s>a</s> <u>b</u> <i>c</i>");
  assert.equal(md("> q\nnext"), '<span class="q">q</span>next');
  assert.equal(md("- a\n# h"), "• a<br><b>h</b>");
  assert.equal(md("[x](https://a.b/_k_/c)"), '<a href="https://a.b/_k_/c" target="_blank" rel="noreferrer">x</a>');
  assert.equal(md("snake_case_name"), "snake_case_name");
  assert.equal(md("a ||secret|| b"), 'a <span class="sp">secret</span> b');
  assert.equal(md("***x***"), "<b><i>x</i></b>");
  assert.equal(md("```js\n<b>1</b>\n**no**\n```\nafter"), "<pre><code>&lt;b&gt;1&lt;/b&gt;\n**no**</code></pre>after");
  assert.equal(md(">>> a\nb\nc"), '<span class="q">a</span><span class="q">b</span><span class="q">c</span>');
});

test("parseAi takes title/content from fenced or chatty replies, rejects empty", () => {
  const body = '{"title":" สรุป ","content":"เนื้อหา"}';
  for (const raw of ["```json\n" + body + "\n```", "ได้เลย\n" + body]) {
    const r = parseAi(raw);
    assert.ok(r.ok);
    if (r.ok) assert.deepEqual([r.title, r.content], ["สรุป", "เนื้อหา"]);
  }
  assert.equal(parseAi("{}").ok, false);
  assert.equal(parseAi("ไม่ใช่ json").ok, false);
});
