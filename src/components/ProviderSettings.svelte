<script lang="ts">
  import { ask } from "@tauri-apps/plugin-dialog";
  import { api, errMsg, type Provider } from "../lib/api.ts";
  import { app, notify, setConfig } from "../lib/store.svelte.ts";

  const clone = () => $state.snapshot(app.config.providers) as Provider[];
  let list = $state<Provider[]>(clone());
  let sel = $state(app.config.providers[0]?.id ?? "");
  let adding = $state(false), showKey = $state(false), busy = $state(""), dirty = $state(false);
  const cur = $derived(list.find((p) => p.id === sel));
  const ready = (p: Provider) => p.enabled && !!p.api_key.trim() && p.models.some((m) => m.enabled);
  const touch = () => (dirty = true);
  const sub = (p: Provider) => !p.enabled ? "ปิดใช้งาน" : !p.api_key.trim() ? "ยังไม่ใส่ API key" : !p.models.some((m) => m.enabled) ? "ยังไม่มีโมเดล" : `${p.models.filter((m) => m.enabled).length} โมเดล`;

  const uid = () => "p" + Date.now().toString(36);
  const PRESET: Provider = {
    id: "", name: "Z.ai Coding Plan", preset: "zai-coding-plan", base_url: "https://api.z.ai/api/anthropic", format: "anthropic", api_key: "", enabled: true,
    models: [{ id: "GLM-5.3", ctx: "1M", vision: false, enabled: true }, { id: "GLM-5.3-Flash", ctx: "1M", vision: true, enabled: true }],
  };

  function add(preset: boolean) {
    const p: Provider = preset ? structuredClone(PRESET) : { id: "", name: "Custom provider", base_url: "", format: "chat", api_key: "", enabled: true, models: [] };
    p.id = uid();
    list.push(p); sel = p.id; adding = false; showKey = false; touch();
  }
  const pick = (id: string) => { sel = id; adding = false; showKey = false; };
  async function remove() {
    if (!cur || !(await ask(`ลบ provider "${cur.name}"?`, { kind: "warning" }))) return;
    list = list.filter((p) => p !== cur); sel = list[0]?.id ?? ""; touch();
  }
  const save = async () => {
    busy = "save";
    try { setConfig(await api.saveProviders($state.snapshot(list) as Provider[])); dirty = false; notify("บันทึก provider แล้ว"); }
    catch (e) { notify(errMsg(e), undefined, true); } finally { busy = ""; }
  };
  async function test(model: string) {
    if (!cur) return;
    busy = "test:" + model;
    try { await api.aiTest($state.snapshot(cur) as Provider, model); notify(`เชื่อมต่อ ${model} ได้`); }
    catch (e) { notify(errMsg(e), undefined, true); } finally { busy = ""; }
  }
</script>

{#snippet ico(d: string)}
  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">{@html d}</svg>
{/snippet}

{#snippet pcard(title: string, text: string, icon: string, onclick: () => void)}
  <button class="pc" {onclick}>
    <span class="pc-i">{@render ico(icon)}</span>
    <span class="pc-t"><b>{title}</b><span>{text}</span></span>
    {@render ico('<path d="m9 6 6 6-6 6"/>')}
  </button>
{/snippet}

<div class="ps">
  <nav aria-label="รายการ provider">
    <div class="cap">ผู้ให้บริการ</div>
    {#each list as p (p.id)}
      <button class="item" class:on={p.id === sel && !adding} aria-current={p.id === sel && !adding} onclick={() => pick(p.id)}>
        <i class="dot" class:ok={ready(p)} aria-hidden="true"></i>
        <span class="it"><b>{p.name}</b><span class:warn={!ready(p)}>{sub(p)}</span></span>
      </button>
    {/each}
    <button class="item add" class:on={adding} onclick={() => (adding = !adding)}>{@render ico('<path d="M12 5v14M5 12h14"/>')}เพิ่ม provider</button>
  </nav>

  <section>
    {#if adding}
      <div class="head"><button class="ib" aria-label="กลับ" onclick={() => (adding = false)}>{@render ico('<path d="m15 6-6 6 6 6"/>')}</button><h3>เพิ่ม provider</h3></div>
      <div class="choices">
        {@render pcard("Z.ai Coding Plan", "ใส่ URL และโมเดลให้แล้ว กรอกแค่ API key", '<path d="M13 3 5 14h6l-1 7 8-11h-6z"/>', () => add(true))}
        {@render pcard("Custom provider", "กำหนด Base URL, format และโมเดลเอง", '<path d="M12 3 4 7.5v9L12 21l8-4.5v-9z"/><path d="M12 12 4 7.5M12 12l8-4.5M12 12v9"/>', () => add(false))}
      </div>
    {:else if cur}
      <div class="head">
        <input class="name" bind:value={cur.name} oninput={touch} aria-label="ชื่อ provider" />
        <label class="sw-l">เปิดใช้งาน<input type="checkbox" role="switch" class="sw" bind:checked={cur.enabled} onchange={touch} /></label>
      </div>

      <label class="f"><span class="lbl">Base URL</span><input bind:value={cur.base_url} oninput={touch} placeholder="https://api.example.com/v1" spellcheck="false" /></label>
      <label class="f"><span class="lbl">API format</span>
        <select bind:value={cur.format} onchange={touch}>
          <option value="anthropic">Anthropic messages (/v1/messages)</option>
          <option value="chat">Chat completions (/chat/completions)</option>
        </select></label>
      <div class="f"><label class="lbl" for="ak">API key</label>
        <div class="key"><input id="ak" type={showKey ? "text" : "password"} bind:value={cur.api_key} oninput={touch} placeholder="วาง API key ที่นี่" autocomplete="off" spellcheck="false" />
          <button class="ib" aria-label={showKey ? "ซ่อน API key" : "แสดง API key"} aria-pressed={showKey} onclick={() => (showKey = !showKey)}>
            {#if showKey}{@render ico('<path d="M3 3l18 18M10.6 6.2A9.8 9.8 0 0 1 12 6c6 0 9.5 6 9.5 6a16 16 0 0 1-3 3.5M6.6 7.6C4 9.3 2.5 12 2.5 12S6 18 12 18c1.4 0 2.7-.3 3.8-.8M9.9 9.9a3 3 0 0 0 4.2 4.2"/>')}
            {:else}{@render ico('<path d="M2.5 12S6 6 12 6s9.5 6 9.5 6-3.5 6-9.5 6-9.5-6-9.5-6z"/><circle cx="12" cy="12" r="3"/>')}{/if}
          </button></div></div>

      <div class="mh"><span class="lbl" style="margin:0">โมเดล</span>
        <button class="sm" onclick={() => { cur.models.push({ id: "", vision: false, enabled: true }); touch(); }}>{@render ico('<path d="M12 5v14M5 12h14"/>')}เพิ่มโมเดล</button></div>
      <ul class="models">
        {#each cur.models as m, i}
          <li>
            <input class="mono mid" bind:value={m.id} oninput={touch} placeholder="ชื่อโมเดล เช่น glm-5.3" aria-label="ชื่อโมเดล" spellcheck="false" />
            <span class="act">
              {#if m.ctx}<span class="badge">{m.ctx}</span>{/if}{#if m.vision}<span class="badge">Vision</span>{/if}
              <button class="sm" disabled={!!busy || !m.id.trim()} onclick={() => test(m.id)}>{busy === "test:" + m.id ? "กำลังทดสอบ…" : "ทดสอบ"}</button>
              <input type="checkbox" role="switch" class="sw" bind:checked={m.enabled} onchange={touch} aria-label="เปิดใช้งาน {m.id}" />
              <button class="ib" aria-label="ลบโมเดล {m.id}" onclick={() => { cur.models.splice(i, 1); touch(); }}>{@render ico('<path d="M4 7h16M10 11v6M14 11v6M6 7l1 13h10l1-13M9 7V4h6v3"/>')}</button>
            </span>
          </li>
        {:else}<li class="none">ยังไม่มีโมเดล · กด "เพิ่มโมเดล" เพื่อใช้ในแท็บ AI</li>{/each}
      </ul>

      <div class="foot">
        <button class="danger" onclick={remove}>ลบ provider</button>
        <span class="hint" style="margin-left:auto">{dirty ? "มีการเปลี่ยนแปลงที่ยังไม่บันทึก" : ""}</span>
        <button class="primary" disabled={!dirty || !!busy} onclick={save}>บันทึก</button>
      </div>
      <p class="hint note">API key เก็บในไฟล์ตั้งค่าของโปรแกรมแบบข้อความธรรมดา และไม่ถูกรวมในไฟล์ส่งออก</p>
    {:else}
      <div class="empty">ยังไม่มี provider<br /><button class="primary" style="margin-top:12px" onclick={() => (adding = true)}>เพิ่ม provider</button></div>
    {/if}
    {#if dirty && !cur && !adding}<div class="foot"><button class="primary" onclick={save}>บันทึก</button></div>{/if}
  </section>
</div>

<style>
  .ps { display: grid; grid-template-columns: 220px minmax(0, 1fr); min-height: 420px; background: var(--surface); border: 1px solid var(--line); border-radius: var(--r-card); overflow: hidden; }
  nav { display: flex; flex-direction: column; gap: 4px; padding: 12px; background: var(--raised); border-right: 1px solid var(--line); }
  .cap { margin: 4px 12px 6px; font-size: 12px; color: var(--muted); }
  .item { display: flex; align-items: center; gap: 10px; width: 100%; text-align: left; padding: 9px 12px; background: transparent; border-color: transparent; }
  .item:hover:not(:disabled) { background: color-mix(in srgb, var(--muted) 14%, transparent); border-color: transparent; }
  .item.on { background: var(--accent-soft); color: var(--accent); }
  .it { display: grid; min-width: 0; line-height: 1.3; }
  .it b { font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .it span { font-size: 12px; color: var(--muted); } .it span.warn { color: var(--warn); }
  .item.on .it span:not(.warn) { color: inherit; opacity: .8; }
  .dot { width: 8px; height: 8px; border-radius: 50%; background: var(--warn); flex: none; }
  .dot.ok { background: var(--ok); }
  .item.add { color: var(--muted); margin-top: 4px; border: 1px dashed var(--line); }
  .item.add.on { color: var(--accent); border-color: var(--accent); }

  section { padding: 24px; min-width: 0; }
  .head { display: flex; align-items: center; gap: 12px; margin-bottom: 20px; }
  .head h3 { margin: 0; font-size: 16px; font-weight: 600; }
  .name { flex: 1; min-width: 0; font-size: 16px; font-weight: 600; background: transparent; border-color: transparent; padding: 4px 8px; margin-left: -8px; }
  .name:hover { border-color: var(--line); }
  .sw-l { white-space: nowrap; display: inline-flex; align-items: center; gap: 8px; font-size: 13px; color: var(--muted); cursor: pointer; }

  .key { position: relative; }
  .key input { padding-right: 40px; }
  .key .ib { position: absolute; right: 3px; top: 50%; translate: 0 -50%; }
  .ib { display: grid; place-items: center; width: 30px; height: 30px; padding: 0; background: transparent; border-color: transparent; color: var(--muted); }
  .ib:hover:not(:disabled) { color: var(--text); background: var(--raised); border-color: transparent; }
  .sm { display: inline-flex; align-items: center; gap: 6px; padding: 4px 10px; font-size: 13px; }

  .mh { display: flex; align-items: center; justify-content: space-between; margin: 4px 0 8px; }
  .models { list-style: none; margin: 0; padding: 0; border: 1px solid var(--line); border-radius: var(--r); }
  .models li { display: flex; flex-wrap: wrap; align-items: center; gap: 4px 8px; padding: 6px 8px 6px 6px; }
  .models li + li { border-top: 1px solid var(--line); }
  .models li.none { justify-content: center; padding: 20px 12px; color: var(--muted); font-size: 13px; }
  .act { display: flex; align-items: center; gap: 8px; margin-left: auto; }
  .mid { flex: 1 1 140px; min-width: 0; background: transparent; border-color: transparent; font-size: 13px; }
  .mid:hover { border-color: var(--line); }
  .badge { font-size: 11px; padding: 1px 7px; border-radius: 999px; background: var(--raised); border: 1px solid var(--line); color: var(--muted); white-space: nowrap; }

  .foot { display: flex; align-items: center; gap: 12px; margin-top: 20px; padding-top: 16px; border-top: 1px solid var(--line); }
  .note { margin: 12px 0 0; }

  .choices { display: grid; gap: 8px; }
  .pc { display: flex; align-items: center; gap: 14px; width: 100%; text-align: left; padding: 14px; background: var(--surface); }
  .pc:hover:not(:disabled) { border-color: var(--accent); background: var(--accent-soft); }
  .pc-i { display: grid; place-items: center; width: 38px; height: 38px; border-radius: 10px; background: var(--raised); color: var(--accent); flex: none; }
  .pc-t { display: grid; flex: 1; gap: 1px; } .pc-t b { font-weight: 600; } .pc-t span { font-size: 12px; color: var(--muted); }

  /* สวิตช์เปิด/ปิด: checkbox ที่แต่งเป็นสวิตช์ (ยังโฟกัสและใช้คีย์บอร์ดได้) */
  .sw { appearance: none; flex: none; width: 34px; height: 20px; padding: 0; border-radius: 999px; border: 0; background: var(--line); position: relative; cursor: pointer; transition: background .15s; }
  .sw::after { content: ""; position: absolute; left: 2px; top: 2px; width: 16px; height: 16px; border-radius: 50%; background: #fff; box-shadow: 0 1px 2px rgba(0,0,0,.3); transition: transform .15s cubic-bezier(.2, .8, .2, 1); }
  .sw:checked { background: var(--accent); } .sw:checked::after { transform: translateX(14px); }
  .sw:focus-visible { box-shadow: 0 0 0 3px var(--accent-soft); outline: 2px solid var(--accent); outline-offset: 1px; }

  @media (max-width: 760px) {
    .ps { grid-template-columns: 1fr; }
    nav { flex-direction: row; flex-wrap: wrap; border-right: 0; border-bottom: 1px solid var(--line); }
    .cap { width: 100%; margin: 0 4px; }
    .item { width: auto; }
  }
  @media (prefers-reduced-motion: reduce) { .sw, .sw::after { transition: none; } }
</style>
