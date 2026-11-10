<script lang="ts">
  import { ask, open, save } from "@tauri-apps/plugin-dialog";
  import defaultPrompt from "../../prompts/showcase-summarizer.md?raw";
  import { api, errMsg } from "../lib/api.ts";
  import ProviderSettings from "../components/ProviderSettings.svelte";
  import { app, notify, setConfig } from "../lib/store.svelte.ts";
  import icon from "../assets/icon.png";

  import { openUrl } from "@tauri-apps/plugin-opener";
  import { check } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { getVersion } from "@tauri-apps/api/app";
  let ver = $state("");
  getVersion().then((v) => (ver = v));

  let token = $state<string | null>(null), busy = $state(""), appId = $state("");
  const validId = $derived(/^\d{15,22}$/.test(appId.trim()));
  // permissions=1040 = View Channels (1024) + Manage Channels (16): อ่านแท็กและสร้างแท็กใหม่ได้
  const inviteUrl = $derived(`https://discord.com/oauth2/authorize?client_id=${appId.trim()}&scope=bot&permissions=1040`);
  const PRESETS = [10, 25, 50, 100];
  const lim = $state({
    img: { mb: app.config.max_file_mb, custom: !PRESETS.includes(app.config.max_file_mb), title: "ขนาดไฟล์สูงสุดต่อรูป",
      hint: "ค่าเริ่มต้น 10MB ปลอดภัยที่สุดกับทุกเซิร์ฟเวอร์ รูปที่เกินจะถูกย่อให้อัตโนมัติ (GIF ย่อไม่ได้)" },
    vid: { mb: app.config.max_video_mb, custom: !PRESETS.includes(app.config.max_video_mb), title: "ขนาดไฟล์สูงสุดต่อคลิป",
      hint: "ค่าเริ่มต้น 25MB คลิปย่อให้อัตโนมัติไม่ได้ ถ้าเกินจะแนบไม่สำเร็จ และเซิร์ฟเวอร์ Discord ต้องรับไฟล์ขนาดนี้ได้ด้วย" },
  });
  let theme = $state(app.config.theme), prompt = $state(app.config.prompt_template);

  const run = async (label: string, f: () => Promise<unknown>) => { busy = label; try { await f(); } catch (e) { notify(errMsg(e), undefined, true); } finally { busy = ""; } };

  let show = $state(false);
  // เปิดตาครั้งแรกตอนยังไม่แตะช่อง → ดึง token ที่เก็บไว้มาแสดง (ดึงเฉพาะตอนผู้ใช้ขอ)
  const toggleShow = () => run("eye", async () => {
    show = !show;
    if (show && token === null && app.hasBot) { const t = await api.getBotToken(); if (t) token = t; }
  });
  const saveToken =() => run("token", async () => { await api.setBotToken(token ?? ""); app.hasBot = await api.hasBotToken(); token = null; notify(app.hasBot ? "บันทึก bot token แล้ว" : "ลบ bot token แล้ว"); });
  const saveSettings = () => run("settings", async () => { setConfig(await api.saveSettings(lim.img.mb, lim.vid.mb, theme, prompt)); notify("บันทึกการตั้งค่าแล้ว"); });

  const upd = () => run("upd", async () => {
    const u = await check();
    if (!u) return notify("เป็นเวอร์ชันล่าสุดแล้ว");
    if (!(await ask(`มีเวอร์ชันใหม่ ${u.version}\n\n${u.body ?? ""}\n\nอัปเดตตอนนี้ไหม?`, { kind: "info" }))) return;
    await u.downloadAndInstall();
    await relaunch();
  });
  const exp = () => run("exp", async () => {
    if (!(await ask("ไฟล์ที่ส่งออกมี URL webhook อยู่ในไฟล์ ใครได้ไฟล์ไปก็โพสต์ลงห้องของคุณได้ เก็บให้ปลอดภัย\n\nส่งออกต่อไหม?", { kind: "warning" }))) return;
    const path = await save({ defaultPath: "discordette-config.json", filters: [{ name: "JSON", extensions: ["json"] }] });
    if (path) { await api.exportConfig(path); notify("ส่งออกแล้ว"); }
  });
  const imp = () => run("imp", async () => {
    const path = await open({ filters: [{ name: "JSON", extensions: ["json"] }] });
    if (typeof path !== "string") return;
    const dup = await api.importConflicts(path);
    const replace = dup.length > 0 && (await ask(`มีห้องชื่อซ้ำ ${dup.length} ห้อง (${dup.join(", ")})\nแทนที่ด้วยของในไฟล์ไหม? (ไม่ = ข้ามห้องที่ซ้ำ)`, { kind: "warning" }));
    const s = await api.importConfig(path, replace);
    setConfig(await api.getConfig());
    notify(`นำเข้าแล้ว: เพิ่ม ${s.added} · แทนที่ ${s.replaced} · ข้าม ${s.skipped}`);
  });
</script>

<div class="p" class:wide={app.sec === "ai"}>
  {#if app.sec === "general"}
    <h1>ทั่วไป</h1>
    {#each [lim.img, lim.vid] as l}
      <h2>{l.title}</h2>
      <div class="row">
        <select aria-label={l.title} style="width:200px" value={l.custom ? "custom" : String(l.mb)}
          onchange={(e) => { const v = e.currentTarget.value; if (v === "custom") l.custom = true; else { l.custom = false; l.mb = +v; saveSettings(); } }}>
          {#each PRESETS as n}<option value={String(n)}>{n} MB</option>{/each}<option value="custom">กำหนดเอง</option>
        </select>
        {#if l.custom}<input type="number" min="1" max="500" bind:value={l.mb} onchange={saveSettings} style="width:100px" aria-label="MB" /> MB{/if}
      </div>
      <p class="hint">{l.hint}</p>
    {/each}
    <h2>ธีม</h2>
    <select bind:value={theme} onchange={saveSettings} aria-label="ธีม" style="width:200px"><option value="system">ตามระบบ</option><option value="light">สว่าง</option><option value="dark">มืด</option></select>

  {:else if app.sec === "discord"}
    <h1>Discord</h1>
    <h2>Bot token <span class="hint">ไม่บังคับ</span></h2>
    <p class="hint">ใช้กับปุ่ม "ดึงแท็ก" อย่างเดียว (อ่านรายการแท็กของห้องฟอรั่ม) ไม่ได้ใช้โพสต์ เก็บใน keyring ของระบบ {app.hasBot ? "· ตั้งไว้แล้ว" : ""}</p>
    <!-- token=null → ยังไม่แตะ: แสดงจุดแทนค่าที่เก็บไว้ (ไม่เคยดึง token จริงมาที่ UI) -->
    <div class="row"><div class="tk"><input type={show ? "text" : "password"} value={token ?? (app.hasBot ? "••••••••••••••••" : "")} oninput={(e) => (token = e.currentTarget.value)}
        onfocus={(e) => token === null && e.currentTarget.select()} placeholder={app.hasBot ? "ล้างช่อง + บันทึก = ลบ token" : "bot token"} autocomplete="off" spellcheck="false" aria-label="bot token" />
        <button class="eye ghost" aria-label={show ? "ซ่อน token" : "แสดง token"} aria-pressed={show} onclick={toggleShow}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            {#if show}<path d="M3 3l18 18M10.6 6.2A9.8 9.8 0 0 1 12 6c6 0 9.5 6 9.5 6a16 16 0 0 1-3 3.5M6.6 7.6C4 9.3 2.5 12 2.5 12S6 18 12 18c1.4 0 2.7-.3 3.8-.8M9.9 9.9a3 3 0 0 0 4.2 4.2" />
            {:else}<path d="M2.5 12S6 6 12 6s9.5 6 9.5 6-3.5 6-9.5 6-9.5-6-9.5-6z" /><circle cx="12" cy="12" r="3" />{/if}</svg></button></div>
      <button disabled={!!busy || token === null} onclick={saveToken}>บันทึก</button></div>

    <h2>วิธีตั้งค่าบอท</h2>
    <ol class="steps">
      <li>เปิด <button class="ghost lnk" onclick={() => openUrl("https://discord.com/developers/applications")}>Discord Developer Portal</button> → <b>New Application</b> ตั้งชื่ออะไรก็ได้</li>
      <li>เมนู <b>Bot</b> → <b>Reset Token</b> → คัดลอก token มาวางในช่องด้านบนแล้วกดบันทึก (token แสดงครั้งเดียว หายแล้วต้อง Reset ใหม่)</li>
      <li>เมนู <b>General Information</b> → คัดลอก <b>Application ID</b> มาวางด้านล่าง</li>
      <li>กด "เชิญบอท" เลือกเซิร์ฟเวอร์ที่ต้องการ แล้วกดอนุญาต</li>
    </ol>
    <div class="row"><input bind:value={appId} inputmode="numeric" placeholder="Application ID" aria-label="application id" />
      <button class="primary" disabled={!validId} onclick={() => openUrl(inviteUrl)}>เชิญบอท</button></div>
    <p class="hint">สิทธิ์ที่ขอ: <b>View Channels</b> (อ่านแท็ก) และ <b>Manage Channels</b> (สร้างแท็กใหม่ ถ้าไม่ใช้ฟีเจอร์นี้ปิดสิทธิ์นี้ตอนเชิญได้) · scope <code>bot</code> ไม่ต้องเปิด Privileged Intents และไม่ต้องให้สิทธิ์ส่งข้อความ เพราะโพสต์ผ่าน webhook</p>
    <p class="hint">ถ้า "ดึงแท็ก" ยังขึ้นว่าบอทมองไม่เห็นห้อง: เป็นห้องส่วนตัว → เข้าตั้งค่าห้อง → Permissions → เพิ่มบอทแล้วเปิด View Channel · ถ้าสร้างแท็กไม่ได้: เปิด Manage Channels ตรงนั้นด้วย (เชิญบอทไว้แล้วก่อนหน้านี้ ต้องเพิ่มสิทธิ์เองที่ Permissions หรือกด "เชิญบอท" ซ้ำ)</p>

  {:else if app.sec === "data"}
    <h1>นำเข้า / ส่งออก</h1>
    <div class="row"><button disabled={!!busy} onclick={exp}>ส่งออกไฟล์ตั้งค่า</button><button disabled={!!busy} onclick={imp}>นำเข้าไฟล์ตั้งค่า</button></div>
    <p class="hint">ไฟล์ส่งออกมี URL webhook อยู่ด้วย อย่าแชร์ให้คนที่ไม่ไว้ใจ (API key ของ AI ไม่ถูกรวมในไฟล์)</p>

  {:else if app.sec === "ai"}
    <h1>AI provider</h1>
    <p class="hint lead">จัดการ provider ที่ใช้กับแท็บ "AI" ในหน้าโพสต์ (ให้ AI สรุปข้อความเป็นหัวข้อ + รายละเอียด) ตั้งค่าแล้วเลือกใช้ได้ตอนสรุป</p>
    <ProviderSettings />

  {:else if app.sec === "about"}
    <h1>เกี่ยวกับ</h1>
    <div class="row"><img class="logo" src={icon} alt="" draggable="false" /><div><b>Discordette</b><div class="hint">เวอร์ชัน {ver}</div></div></div>
    <div class="row"><button class="primary" disabled={!!busy} onclick={upd}>{busy === "upd" ? "กำลังตรวจสอบ…" : "ตรวจสอบอัปเดต"}</button></div>
    <div class="row">
      <button class="ghost lnk" onclick={() => openUrl("https://github.com/MyNightOnline/discordette")}>GitHub</button>
      <button class="ghost lnk" onclick={() => openUrl("https://github.com/MyNightOnline/discordette/releases")}>Releases</button>
    </div>
    <p class="hint">ใบอนุญาต MIT</p>

  {:else}
    <h1>Prompt template</h1>
    <textarea rows="14" class="mono" bind:value={prompt} aria-label="prompt template"></textarea>
    <p class="hint">{"{{rooms}}"} และ {"{{tags}}"} จะถูกแทนด้วยชื่อห้องและแท็กจริงตอนกด "คัดลอก prompt"</p>
    <div class="row"><button class="primary" disabled={!!busy} onclick={saveSettings}>บันทึก prompt</button><button onclick={() => (prompt = defaultPrompt)}>คืนค่าเริ่มต้น</button></div>
  {/if}
</div>

<style>
  .p { padding: 32px 24px; max-width: 720px; margin: 0 auto; }
  .p.wide { max-width: 900px; }
  .steps { margin: 0 0 16px; padding-left: 20px; font-size: 13px; line-height: 1.8; }
  .lnk { color: var(--accent); padding: 0; text-decoration: underline; }
  .row button { white-space: nowrap; }
  .tk { position: relative; flex: 1; }
  .tk input { width: 100%; padding-right: 40px; }
  .row .eye { position: absolute; right: 3px; top: 50%; translate: 0 -50%; width: 30px; height: 30px; padding: 0; display: grid; place-items: center; color: var(--muted); }
  .logo { width: 56px; height: 56px; border-radius: 12px; }
  .lead { margin: -8px 0 20px; font-size: 13px; }
</style>
