import { decodeErr, fmt, type Key, type Lang, type Params } from "./i18n/index.ts";

const read = (): Lang => {
  try { return localStorage.getItem("lang") === "th" ? "th" : "en"; } catch { return "en"; }
};

export const i18n = $state({ lang: read() });
document.documentElement.lang = i18n.lang;

export function setLang(l: Lang) {
  i18n.lang = l;
  document.documentElement.lang = l;
  try { localStorage.setItem("lang", l); } catch { /* ไม่เก็บก็ไม่เป็นไร */ }
}

/** Reactive: reads i18n.lang, so templates and $derived re-run when the language changes. */
export const t = (key: Key, p?: Params) => fmt(i18n.lang, key, p);
export const errText = (s: string) => decodeErr(i18n.lang, s);
export const locale = () => (i18n.lang === "th" ? "th-TH" : "en-US");
