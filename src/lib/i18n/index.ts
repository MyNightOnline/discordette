// Plain TS (no Svelte runes) so node tests can import it too.
import { en, type Key } from "./en.ts";
import { th } from "./th.ts";

export type Lang = "en" | "th";
export type { Key };
export type Params = Record<string | number, string | number>;
export type Tr = (key: Key, p?: Params) => string;

const dicts: Record<Lang, Record<Key, string>> = { en, th };

export const fmt = (lang: Lang, key: Key, p?: Params): string => {
  const s = dicts[lang][key] ?? en[key] ?? key;
  return p ? s.replace(/\{(\w+)\}/g, (m, k) => (k in p ? String(p[k]) : m)) : s;
};

/** Rust errors look like "err.code\u001farg0\u001farg1"; anything else is returned as-is. */
export const decodeErr = (lang: Lang, s: string): string => {
  const [code, ...args] = s.split("\u001f");
  return code in en ? fmt(lang, code as Key, Object.fromEntries(args.map((a, i) => [i, a]))) : s;
};
