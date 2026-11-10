<script lang="ts">
  import { ask } from "@tauri-apps/plugin-dialog";
  import { api, errMsg, type WebhookInfo } from "../lib/api.ts";
  import { app, editor, notify, setConfig } from "../lib/store.svelte.ts";

  const URL_RE = /^https:\/\/(discord\.com|discordapp\.com|ptb\.discord\.com|canary\.discord\.com)\/api\/webhooks\/\d+\/[\w-]+/;
  let url = $state(""), name = $state(""), kind = $state<"forum" | "text">("forum"), requireTag = $state(false);
  let info = $state<WebhookInfo | null>(null), check = $state(""), busy = $state(false), error = $state("");
  let timer: ReturnType<typeof setTimeout>;
  let stored = $state(""), shown = $state(false), act = $state("");
  const cur = $derived(editor.room && app.config.rooms.find((r) => r.id === editor.room!.id));
  const mask = (u: string) => u.replace(/(\/api\/webhooks\/\d{2})\d*\/.*/, "$1…/••••");

  $effect(() => {
    if (!editor.open) return;
    const r = editor.room;
    url = ""; info = null; check = ""; error = "";
    name = r?.name ?? ""; kind = r?.kind ?? "forum"; requireTag = r?.require_tag ?? false;
    stored = ""; shown = false; act = "";
    if (r) api.getWebhookUrl(r.id).then((u) => (stored = u ?? ""));
  });

  function onUrl() {
    clearTimeout(timer); info = null; error = "";
    const u = url.trim();
    if (!u) { check = ""; return; }
    if (!URL_RE.test(u)) { check = "รูปแบบ URL ไม่ถูกต้อง"; return; }
    check = "กำลังตรวจ…";
    timer = setTimeout(async () => {
      try { info = await api.webhookInfo(u); check = `✓ ${info.name}`; if (!name) name = info.name; }
      catch (e) { check = errMsg(e); }
    }, 300);
  }

  async function save() {
    busy = true; error = "";
    try {
      const old = cur ?? editor.room;
      const room = {
        id: old?.id ?? "", name: name.trim(), kind, require_tag: kind === "forum" && requireTag,
        channel_id: info?.channel_id ?? old?.channel_id ?? null, guild_id: info?.guild_id ?? old?.guild_id ?? null,
        tags: kind === "forum" ? old?.tags ?? [] : [],
      };
      const before = new Set(app.config.rooms.map((r) => r.id));
      let c = await api.saveRoom(room, url.trim() || undefined);
      const added = c.rooms.find((r) => !before.has(r.id));
      if (added && app.hasBot && kind === "forum" && added.channel_id) { try { c = await api.fetchTags(added.id); } catch { /* ดึงแท็กทีหลังได้ */ } }
      setConfig(c);
      if (added) app.roomId = added.id;
      editor.open = false;
      notify(old ? "บันทึกห้องแล้ว" : "เพิ่มห้องแล้ว");
    } catch (e) { error = errMsg(e); } finally { busy = false; }
  }
  const run = async (label: string, f: () => Promise<unknown>) => { act = label; try { await f(); } catch (e) { error = errMsg(e); } finally { act = ""; } };
  const test = () => run("test", async () => {
    const r = cur!, i = await api.testRoom(r.id);
    // เก็บ channel/guild ไว้ใช้ดึงแท็กและสร้างลิงก์
    setConfig(await api.saveRoom({ ...r, channel_id: i.channel_id, guild_id: i.guild_id ?? r.guild_id ?? null }));
    notify(`✓ webhook ใช้งานได้: ${i.name}`);
  });
  const tags = () => run("tags", async () => { setConfig(await api.fetchTags(cur!.id)); notify("ดึงแท็กแล้ว"); });
  const del = () => run("del", async () => {
    if (!(await ask(`ลบห้อง "${cur!.name}" ?`, { kind: "warning" }))) return;
    setConfig(await api.deleteRoom(cur!.id));
    editor.open = false;
  });
  const canSave = $derived(!!name.trim() && (editor.room ? !url.trim() || URL_RE.test(url.trim()) : URL_RE.test(url.trim())) && !busy);
</script>

{#if editor.open}
  <div class="back" role="presentation" onclick={() => (editor.open = false)} onkeydown={(e) => e.key === "Escape" && (editor.open = false)}>
    <div class="dlg card" role="dialog" aria-modal="true" aria-label={editor.room ? "แก้ไขห้อง" : "เพิ่มห้อง"} tabindex="-1" onclick={(e) => e.stopPropagation()} onkeydown={(e) => e.stopPropagation()}>
      <div class="head">
        <h1>{editor.room ? "แก้ไขห้อง" : "เพิ่มห้อง"}</h1>
        {#if cur}
          <div class="row acts">
            <button disabled={!!act} onclick={test}>{act === "test" ? "กำลังทดสอบ…" : "ทดสอบ"}</button>
            {#if kind === "forum"}<button disabled={!!act || !app.hasBot || !cur.channel_id} onclick={tags}>ดึงแท็ก ({cur.tags.length})</button>{/if}
            <button class="danger" disabled={!!act} onclick={del}>ลบห้อง</button>
          </div>
        {/if}
      </div>
      {#if cur && kind === "forum" && !app.hasBot}<div class="hint note">ดึงแท็กต้องตั้ง bot token ก่อน (ที่หน้าตั้งค่า)</div>
      {:else if cur && kind === "forum" && !cur.channel_id}<div class="hint note">กดทดสอบก่อน จึงจะดึงแท็กได้</div>{/if}
      <label class="f"><div class="lbl">URL webhook{#if cur && stored}<button type="button" class="ghost eye" aria-label={shown ? "ซ่อน URL" : "แสดง URL"} onclick={(e) => { e.preventDefault(); shown = !shown; }}><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{#if shown}<path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24M1 1l22 22" />{:else}<path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8S1 12 1 12z" /><circle cx="12" cy="12" r="3" />{/if}</svg></button>{/if}</div>
        <input class:mono={!!cur && !!stored} bind:value={url} oninput={onUrl} placeholder={cur && stored ? (shown ? stored : mask(stored)) : editor.room ? "เว้นว่างไว้เพื่อใช้ URL เดิม" : "https://discord.com/api/webhooks/…"} spellcheck="false" autocomplete="off" />
        {#if check}<div class="hint" class:ok={!!info}>{check}</div>{/if}
      </label>
      <label class="f"><div class="lbl">ชื่อห้อง</div><input bind:value={name} /></label>
      <label class="f"><div class="lbl">ชนิด</div>
        <select bind:value={kind}><option value="forum">ฟอรั่ม</option><option value="text">ห้องข้อความ</option></select>
        {#if app.hasBot}<div class="hint">เมื่อมี bot token กดดึงแท็กแล้วโปรแกรมจะตั้งชนิดให้อัตโนมัติ</div>{/if}
      </label>
      {#if kind === "forum"}<label class="row f"><input type="checkbox" style="width:auto" bind:checked={requireTag} /> ห้องนี้บังคับเลือกแท็ก</label>{/if}
      {#if error}<div class="banner err">{error}</div>{/if}
      <div class="row" style="justify-content:flex-end">
        <button onclick={() => (editor.open = false)}>ยกเลิก</button>
        <button class="primary" disabled={!canSave} onclick={save}>{busy ? "กำลังบันทึก…" : "บันทึก"}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .back { position: fixed; inset: 0; background: rgba(0,0,0,.5); display: grid; place-items: center; z-index: 40; }
  .dlg { width: 480px; max-width: 92vw; box-shadow: var(--shadow); }
  .ok { color: var(--ok); }
  .head { display: flex; justify-content: space-between; align-items: flex-start; gap: 12px; margin-bottom: 16px; }
  .head h1 { margin: 0; }
  .acts { flex-wrap: wrap; justify-content: flex-end; }
  .note { margin: -8px 0 16px; text-align: right; }
  .eye { padding: 2px 4px; line-height: 0; color: var(--muted); }
  .eye:hover:not(:disabled), .eye:focus-visible { color: var(--accent); outline: none; }
</style>
