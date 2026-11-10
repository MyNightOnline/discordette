import { test } from "node:test";
import assert from "node:assert/strict";
import { decodeErr, fmt } from "./i18n/index.ts";

test("Rust coded errors decode per language, unknown text passes through", () => {
  const raw = "err.titleLen\u001f100\u001f120";
  assert.equal(decodeErr("en", raw), "Title must be 1–100 characters (currently 120)");
  assert.equal(decodeErr("th", raw), "หัวข้อต้องยาว 1–100 ตัวอักษร (ตอนนี้ 120)");
  assert.equal(decodeErr("th", "plain os error"), "plain os error");
});

test("fmt fills params and keeps literal braces", () => {
  assert.equal(fmt("en", "prompt.hint", { rooms: "{{rooms}}", tags: "{{tags}}" }).startsWith("{{rooms}} and {{tags}}"), true);
  assert.equal(fmt("en", "img.count", { n: 3 }), "3 files");
});
