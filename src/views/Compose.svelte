<script lang="ts">
  import { listen } from "@tauri-apps/api/event";
  import { ask } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, errMsg, type Entry } from "../lib/api.ts";
  import { locale, t } from "../lib/i18n.svelte.ts";
  import { parseAi, parseClaude } from "../lib/jsonImport.ts";
  import { composeText, withSource, len, MAX_CONTENT, MAX_TAGS, MAX_TITLE } from "../lib/limits.ts";
  import { addFiles, app, clearDraft, draft, editor, notify, problems, removeSlot, room as currentRoom, saveDraft } from "../lib/store.svelte.ts";
  import Counter from "../components/Counter.svelte";
  import ImageGrid from "../components/ImageGrid.svelte";
  import Preview from "../components/Preview.svelte";
  import TagPicker from "../components/TagPicker.svelte";

  let mode = $state<"manual" | "json" | "ai">("manual");
  let aiText = $state(""), aiExtra = $state(""), aiErr = $state(""), aiBusy = $state(0), aiPick = $state(localStorage.getItem("ai.pick") ?? "");
  // ตัวเลือก = โมเดลที่เปิดอยู่ของ provider ที่พร้อมใช้ ("providerId|model")
  const aiChoices = $derived(app.config.providers.filter((p) => p.enabled && p.api_key.trim()).flatMap((p) => p.models.filter((m) => m.enabled && m.id.trim()).map((m) => ({ key: p.id + "|" + m.id, label: `${p.name} · ${m.id}` }))));
  const aiSel = $derived(aiChoices.find((c) => c.key === aiPick) ?? aiChoices[0]);
  let elapsed = $state(0);
  $effect(() => {
    if (!aiBusy) return;
    const t0 = Date.now(); elapsed = 0;
    const h = setInterval(() => (elapsed = (Date.now() - t0) / 1000), 100);
    return () => clearInterval(h);
  });
  let tab = $state<"form" | "preview">("form");
  let narrow = $state(innerWidth < 1200);
  let json = $state(""), jsonErr = $state(""), warns = $state<string[]>([]), lowConf = $state(false);
  let flash = $state<string[]>([]);
  let sending = $state(false), cancelling = $state(false), prog = $state({ sent: 0, total: 0 }), dup = $state<Entry | null>(null);
  let ta = $state<HTMLTextAreaElement>();

  const r = $derived(currentRoom());
  const forum = $derived(r?.kind === "forum");
  const probs = $derived(problems());
  const contentLen = $derived(forum ? len(withSource(draft.content, draft.source)) : len(composeText(draft.title, withSource(draft.content, draft.source))));

  $effect(() => { draft.title; draft.content; draft.source; draft.tagIds.length; app.roomId; saveDraft(); });
  $effect(() => {
    const t = draft.title, s = draft.source, rid = app.roomId;
    const h = setTimeout(async () => { dup = t.trim() || s.trim() ? await api.findDuplicate(rid, t, s).catch(() => null) : null; }, 400);
    return () => clearTimeout(h);
  });
  $effect(() => {
    const on = () => (narrow = innerWidth < 1200);
    const paste = (e: ClipboardEvent) => {
      const files = [...(e.clipboardData?.files ?? [])];
      if (files.length) { e.preventDefault(); addFiles(files); }
    };
    const key = (e: KeyboardEvent) => { if (e.ctrlKey && e.key === "Enter") { e.preventDefault(); send(); } };
    addEventListener("resize", on); addEventListener("paste", paste); addEventListener("keydown", key);
    return () => { removeEventListener("resize", on); removeEventListener("paste", paste); removeEventListener("keydown", key); };
  });
  $effect(() => { if (ta) { ta.style.height = "auto"; ta.style.height = Math.max(140, ta.scrollHeight) + "px"; } draft.content; });

  const clearWarns = () => { warns = []; lowConf = false; };

  function wrap(a: string, b = a) {
    const s = ta!.selectionStart, e = ta!.selectionEnd, v = draft.content;
    draft.content = v.slice(0, s) + a + v.slice(s, e) + b + v.slice(e);
    queueMicrotask(() => { ta!.focus(); ta!.setSelectionRange(s + a.length, e + a.length); });
  }
  function quote() {
    const v = draft.content, s = ta!.selectionStart, e = ta!.selectionEnd;
    const a = v.lastIndexOf("\n", s - 1) + 1;
    draft.content = v.slice(0, a) + v.slice(a, e).replace(/^|\n/g, (m) => m + "> ") + v.slice(e);
    queueMicrotask(() => { ta!.focus(); ta!.setSelectionRange(s + 2, s + 2); });
  }
  const link = () => {
    const sel = draft.content.slice(ta!.selectionStart, ta!.selectionEnd);
    wrap("[" + (sel ? "" : t("link.text")), "](https://)");
  };
  function keys(e: KeyboardEvent) {
    const f = e.ctrlKey && { b: () => wrap("**"), i: () => wrap("*"), u: () => wrap("__"), k: link }[e.key.toLowerCase()];
    if (f) { e.preventDefault(); f(); }
  }

  function applyJson() {
    const res = parseClaude(json, app.config.rooms, app.roomId, t);
    if (!res.ok) { jsonErr = res.error; return; }
    const v = res.v;
    jsonErr = ""; warns = v.warnings; lowConf = v.confidence === "low";
    app.roomId = v.roomId; draft.title = v.title; draft.content = v.content; draft.source = v.source; draft.tagIds = v.tagIds;
    if (v.imageOrder.length) {
      const old = [...draft.slots];
      const keep = [...new Set(v.imageOrder)].map((i) => old[i]).filter(Boolean);
      for (const s of old) if (!keep.includes(s)) removeSlot(s.key);
      draft.slots.splice(0, draft.slots.length, ...keep);
    }
    flash = ["title", "content", "source", "tags"];
    setTimeout(() => (flash = []), 1500);
    mode = "manual"; json = "";
  }

  async function runAi() {
    if (!aiSel || aiBusy || !aiText.trim()) return;
    if ((draft.title.trim() || draft.content.trim()) && !(await ask(t("confirm.aiReplace"), { kind: "warning" }))) return;
    const id = Date.now(); aiBusy = id; aiErr = "";
    const [pid, ...m] = aiSel.key.split("|");
    try {
      const raw = await api.aiSummarize(pid, m.join("|"), aiText, aiExtra, forum && r ? r.tags.map((t) => t.name) : []);
      if (aiBusy !== id) return; // ผู้ใช้กดยกเลิกไปแล้ว
      const res = parseAi(raw, t);
      if (!res.ok) { aiErr = res.error; return; }
      draft.title = res.title; draft.content = res.content; clearWarns();
      if (forum && r) { // แท็กที่ตรงกับของห้องใส่ให้เลย · ชื่อที่ยังไม่มีเก็บไว้เป็นข้อเสนอให้กดสร้าง
        const have = (n: string) => r.tags.find((t) => t.name.toLowerCase() === n.toLowerCase());
        draft.tagIds = [...new Set(res.tags.map((n) => have(n)?.id).filter((x): x is string => !!x))].slice(0, MAX_TAGS);
        draft.suggest = [...new Set([...res.newTags, ...res.tags.filter((n) => !have(n))])].filter((n) => !have(n)).slice(0, 2);
      }
      flash = ["title", "content", "tags"]; setTimeout(() => (flash = []), 1500);
      mode = "manual";
    } catch (e) { if (aiBusy === id) aiErr = errMsg(e); } finally { if (aiBusy === id) aiBusy = 0; }
  }

  function go(field: string) {
    const el = document.getElementById("f-" + field);
    el?.scrollIntoView({ block: "center", behavior: "smooth" });
    el?.querySelector<HTMLElement>("input,textarea,button")?.focus();
  }

  /** ล้างเฉพาะแท็บที่เปิดอยู่: AI / JSON ล้างช่องวางของตัวเอง ส่วน "กรอกเอง" ล้างฟอร์มและรูป */
  async function clear() {
    if (mode === "ai") {
      if ((aiText.trim() || aiExtra.trim()) && !(await ask(t("confirm.clearAi"), { kind: "warning" }))) return;
      aiBusy = 0; aiText = ""; aiExtra = ""; aiErr = ""; return;
    }
    if (mode === "json") { json = ""; jsonErr = ""; return; }
    const dirty = draft.title || draft.content || draft.source || draft.tagIds.length || draft.slots.length;
    if (dirty && !(await ask(t("confirm.clearAll"), { kind: "warning" }))) return;
    await clearDraft();
  }

  async function send() {
    if (sending || !r || probs.length) return;
    if (dup && !(await ask(t("confirm.dup"), { kind: "warning" }))) return;
    const imgs = draft.slots.filter((s) => s.img);
    sending = true; cancelling = false; prog = { sent: 0, total: imgs.length };
    const stop = await listen<{ sent: number }>("send-progress", (e) => (prog.sent = e.payload.sent));
    try {
      const res = await api.sendPost({ roomId: r.id, title: draft.title.trim(), content: withSource(draft.content, draft.source), tagIds: forum ? draft.tagIds : [],
        imageIds: imgs.map((s) => s.img!.id), sourceUrl: draft.source.trim() });
      if (res.cancelled) { notify(t("toast.cancelled", { sent: res.sent, total: imgs.length }), res.link ?? undefined, true); return; }
      await clearDraft();
      warns = []; lowConf = false;
      notify(t("toast.posted"), res.link ?? undefined);
    } catch (e) { notify(errMsg(e), undefined, true); } finally { stop(); sending = false; }
  }
  const cancel = () => { cancelling = true; api.cancelSend().catch(() => {}); };
</script>

<div class="wrap" class:narrow>
  <div class="cols">
    {#if narrow}
      <div class="tabs row"><button class:on={tab === "form"} onclick={() => (tab = "form")}>{t("tab.form")}</button><button class:on={tab === "preview"} onclick={() => (tab = "preview")}>{t("tab.preview")}</button></div>
    {/if}
    {#if !narrow || tab === "form"}
    <div class="form">
      {#if !r}
        <div class="empty">{t("empty.noRoom")}<button class="primary" onclick={() => { editor.room = undefined; editor.open = true; }}>{t("empty.add")}</button></div>
      {:else}
        <div class="room-h">{t("room.header")} <b>#{r.name}</b> · {forum ? t("room.forum") : t("room.text")}</div>
        <div class="row top">
          <div class="seg"><button class:on={mode === "manual"} onclick={() => (mode = "manual")}>{t("mode.manual")}</button><button class:on={mode === "json"} onclick={() => (mode = "json")}>{t("mode.json")}</button><button class:on={mode === "ai"} onclick={() => (mode = "ai")}>{t("mode.ai")}</button></div>
        </div>
        {#if lowConf}<div class="banner">{t("banner.lowConf")}</div>{/if}
        {#each warns as w}<div class="banner">{w}</div>{/each}

        {#if mode === "ai"}
          {#if !aiChoices.length}
            <div class="banner">{t("ai.noProvider")}<button class="ghost lnk" onclick={() => (app.view = "settings")}>{t("ai.goSettings")}</button></div>
          {:else}
            <label class="f"><div class="lbl"><span>{t("ai.model")}</span></div>
              <select value={aiSel?.key} onchange={(e) => { aiPick = e.currentTarget.value; try { localStorage.setItem("ai.pick", aiPick); } catch { /* ไม่เก็บก็ไม่เป็นไร */ } }}>
                {#each aiChoices as c}<option value={c.key}>{c.label}</option>{/each}</select></label>
            <label class="f"><div class="lbl"><span>{t("ai.source")}</span><span class="hint">{t("ai.chars", { n: len(aiText) })}</span></div>
              <textarea rows="9" bind:value={aiText} placeholder={t("ai.srcPh")}></textarea></label>
            <label class="f"><div class="lbl"><span>{t("ai.extra")} <span class="hint">{t("dc.optional")}</span></span></div>
              <input bind:value={aiExtra} placeholder={t("ai.extraPh")} /></label>
            {#if aiErr}<div class="banner err">{aiErr}</div>{/if}
            <div class="row">
              {#if aiBusy}<button onclick={() => { aiBusy = 0; }}>{t("ai.cancel")}</button><span class="hint">{t("ai.working")}</span>
              {:else}<button class="primary" disabled={!aiText.trim()} onclick={runAi}>{t("ai.run")}</button>
                <span class="hint">{t("ai.runHint")}</span>{/if}
            </div>
          {/if}
        {:else if mode === "json"}
          <textarea class="mono" rows="10" bind:value={json} placeholder={t("json.ph")} spellcheck="false"></textarea>
          {#if jsonErr}<div class="banner err" style="margin-top:8px">{jsonErr}</div>{/if}
          <div style="margin-top:8px"><button class="primary" disabled={!json.trim()} onclick={applyJson}>{t("json.fill")}</button>
            <span class="hint">{t("json.hint")}</span></div>
        {:else}
          <label class="f" id="f-title"><div class="lbl"><span>{t("f.title")}{#if forum} <b class="req" aria-hidden="true">*</b>{/if}</span><Counter n={len(draft.title.trim())} max={MAX_TITLE} /></div>
            <input class:flash={flash.includes("title")} bind:value={draft.title} oninput={clearWarns} /></label>
          <div class="f" id="f-content"><div class="lbl"><span>{t("f.details")} <b class="req" aria-hidden="true">*</b></span><Counter n={contentLen} max={MAX_CONTENT} /></div>
            <div class="tb" role="toolbar" aria-label={t("tb.aria")}>
              <button aria-label={t("tb.bold")} onclick={() => wrap("**")}><b>B</b></button>
              <button aria-label={t("tb.italic")} onclick={() => wrap("*")}><i>I</i></button>
              <button aria-label={t("tb.underline")} onclick={() => wrap("__")}><u>U</u></button>
              <button aria-label={t("tb.strike")} onclick={() => wrap("~~")}><s>S</s></button>
              <i class="sep"></i>
              <button aria-label={t("tb.quote")} onclick={quote}>❝</button>
              <button aria-label={t("tb.code")} onclick={() => wrap("`")}>&lt;&gt;</button>
              <button aria-label={t("tb.spoiler")} onclick={() => wrap("||")}><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8S1 12 1 12z" /><circle cx="12" cy="12" r="3" /></svg></button>
              <button aria-label={t("tb.link")} onclick={link}>🔗</button>
            </div>
            <textarea class:flash={flash.includes("content")} bind:this={ta} bind:value={draft.content} oninput={clearWarns} onkeydown={keys}></textarea></div>
          <label class="f" id="f-source"><div class="lbl"><span>{t("f.source")}</span></div>
            <input class:flash={flash.includes("source")} bind:value={draft.source} oninput={clearWarns} placeholder="https://" /></label>
          {#if dup}<div class="banner">{t("dup.banner", { date: new Date(dup.at).toLocaleDateString(locale()), room: dup.room_name })}
            {#if dup.link}· <button class="ghost lnk" onclick={() => openUrl(dup!.link!)}>{t("dup.open")}</button>{/if}</div>{/if}
          {#if forum && r}<div class:flash={flash.includes("tags")}><TagPicker room={r} onchange={saveDraft} /></div>{/if}
          <ImageGrid />
        {/if}
      {/if}
    </div>
    {/if}
    {#if !narrow || tab === "preview"}<div class="pv">
      {#if mode === "ai" && aiBusy}
        <div class="aip">
          <div class="aic"><div class="sk w1"></div><div class="sk w2"></div></div>
          <div class="ait"><span class="dots"><i></i><i></i><i></i></span> {t("ai.thinking")} <b class="mono">{elapsed.toFixed(1)}s</b></div>
          <details class="aid"><summary>{t("ai.reqDetails")}</summary>
            <dl>
              <dt>{t("ai.provider")}</dt><dd>{aiSel?.label.split(" · ")[0]}</dd>
              <dt>{t("ai.model")}</dt><dd class="mono">{aiSel?.key.split("|").slice(1).join("|")}</dd>
              <dt>{t("ai.source")}</dt><dd>{t("ai.chars", { n: len(aiText) })}</dd>
              <dt>{t("ai.extra")}</dt><dd>{aiExtra.trim() ? t("ai.chars", { n: len(aiExtra) }) : "—"}</dd>
              <dt>{t("ai.status")}</dt><dd>{elapsed < 0.5 ? t("ai.sending") : t("ai.waiting")}</dd>
            </dl>
          </details>
        </div>
      {:else}<Preview />{/if}
    </div>{/if}
  </div>
  <footer>
    <div class="status">
      {#if sending && prog.total}
        <div class="prog" role="progressbar" aria-valuemin="0" aria-valuemax={prog.total} aria-valuenow={prog.sent}>
          <div class="bar"><i style="width:{(prog.sent / prog.total) * 100}%"></i></div>
          <span class="mono">{prog.sent}/{prog.total}</span>
        </div>
      {:else if sending}<span class="ready">{t("post.sending")}</span>
      {:else if !r}{:else if !probs.length}<span class="ready"><i></i>{t("post.ready")}</span>
      {:else}{#each probs as p, i}{#if i}, {/if}<button class="ghost lnk" onclick={() => go(p.field)}>{p.msg}</button>{/each}{/if}
    </div>
    {#if sending}<button disabled={cancelling} onclick={cancel}>{cancelling ? t("post.cancelling") : t("post.cancel")}</button>
    {:else}<button onclick={clear}>{t("post.clear")}</button>{/if}
    <button class="primary post" disabled={sending || !r || probs.length > 0} onclick={send}>
      {sending ? t("post.sending") : t("post.to", { room: r?.name ?? "" })}</button>
  </footer>
</div>

<style>
  .wrap { height: 100%; display: flex; flex-direction: column; min-height: 0; }
  .cols { flex: 1; display: flex; min-height: 0; overflow: auto; }
  .form { flex: 1; max-width: 640px; padding: 24px; margin: 0 auto; min-width: 0; }
  .pv { width: 380px; flex: none; border-left: 1px solid var(--line); }
  .narrow .cols { flex-direction: column; }
  .narrow .form { width: 100%; box-sizing: border-box; }
  .narrow .pv { width: auto; flex: 1; border: 0; }
  .aip { padding: 16px; display: flex; flex-direction: column; gap: 14px; }
  .aic { background: var(--line); border-radius: 8px; padding: 14px; display: flex; flex-direction: column; gap: 8px; }
  .sk { height: 12px; border-radius: 6px; background: linear-gradient(90deg, #8884 25%, #8887 50%, #8884 75%); background-size: 200% 100%; animation: sh 1.2s linear infinite; }
  .sk.w1 { width: 45%; } .sk.w2 { width: 80%; }
  @keyframes sh { to { background-position: -200% 0; } }
  .ait { color: var(--muted); font-size: 13px; display: flex; align-items: center; gap: 6px; }
  .ait b { color: var(--text); font-weight: 500; }
  .dots { display: inline-flex; gap: 3px; }
  .dots i { width: 5px; height: 5px; border-radius: 50%; background: currentColor; animation: bl 1s infinite; }
  .dots i:nth-child(2) { animation-delay: .2s; } .dots i:nth-child(3) { animation-delay: .4s; }
  @keyframes bl { 0%, 80%, 100% { opacity: .2; } 40% { opacity: 1; } }
  .aid { font-size: 12px; color: var(--muted); }
  .aid summary { cursor: pointer; }
  .aid dl { display: grid; grid-template-columns: auto 1fr; gap: 4px 12px; margin: 8px 0 0; }
  .aid dt { color: var(--muted); } .aid dd { margin: 0; color: var(--text); word-break: break-all; }
  .room-h { color: var(--muted); font-size: 13px; margin-bottom: 8px; }
  .room-h b { color: var(--text); }
  .top { margin-bottom: 16px; }
  .seg { display: flex; gap: 4px; }
  .seg button.on, .tabs button.on { background: var(--accent-soft); border-color: var(--accent); color: var(--accent); }
  .tabs { padding: 8px 24px 0; max-width: 640px; width: 100%; margin: 0 auto; box-sizing: border-box; }
  .tb { display: inline-flex; align-items: center; gap: 2px; padding: 4px; margin-bottom: 6px; background: var(--raised); border: 1px solid var(--line); color: var(--text); border-radius: 8px; }
  .tb button { display: grid; place-items: center; min-width: 30px; height: 28px; padding: 0 6px; background: transparent; border-color: transparent; color: inherit; }
  .tb button:hover { background: var(--line); border-color: transparent; }
  .tb .sep { width: 1px; height: 18px; margin: 0 4px; background: var(--line); }
  .lnk { color: var(--accent); padding: 0; text-decoration: underline; }
  footer { display: flex; gap: 8px; align-items: center; padding: 12px 24px; border-top: 1px solid var(--line); background: var(--surface); }
  .post { max-width: 280px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: none; }
  .status { min-width: 0; flex: 1; color: var(--warn); } .ready { color: var(--ok); display: inline-flex; align-items: center; gap: 6px; font-weight: 500; }
  .prog { display: flex; align-items: center; gap: 10px; color: var(--text); }
  .bar { flex: 1; max-width: 360px; height: 8px; border-radius: 4px; background: var(--line); overflow: hidden; }
  .bar i { display: block; height: 100%; background: var(--accent); transition: width .25s; }
  .prog .mono { font-size: 12px; color: var(--muted); white-space: nowrap; }
  .ready i { width: 8px; height: 8px; border-radius: 50%; background: currentColor; }
</style>
