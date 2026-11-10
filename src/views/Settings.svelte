<script lang="ts">
  import { ask, open, save } from "@tauri-apps/plugin-dialog";
  import defaultPrompt from "../../prompts/showcase-summarizer.md?raw";
  import { api, errMsg } from "../lib/api.ts";
  import ProviderSettings from "../components/ProviderSettings.svelte";
  import { app, notify, setConfig } from "../lib/store.svelte.ts";
  import { i18n, setLang, t } from "../lib/i18n.svelte.ts";
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
    img: { mb: app.config.max_file_mb, custom: !PRESETS.includes(app.config.max_file_mb), title: "set.imgTitle" as const, hint: "set.imgHint" as const },
    vid: { mb: app.config.max_video_mb, custom: !PRESETS.includes(app.config.max_video_mb), title: "set.vidTitle" as const, hint: "set.vidHint" as const },
  });
  let theme = $state(app.config.theme), prompt = $state(app.config.prompt_template);

  const run = async (label: string, f: () => Promise<unknown>) => { busy = label; try { await f(); } catch (e) { notify(errMsg(e), undefined, true); } finally { busy = ""; } };

  let show = $state(false);
  // เปิดตาครั้งแรกตอนยังไม่แตะช่อง → ดึง token ที่เก็บไว้มาแสดง (ดึงเฉพาะตอนผู้ใช้ขอ)
  const toggleShow = () => run("eye", async () => {
    show = !show;
    if (show && token === null && app.hasBot) { const t = await api.getBotToken(); if (t) token = t; }
  });
  const saveToken =() => run("token", async () => { await api.setBotToken(token ?? ""); app.hasBot = await api.hasBotToken(); token = null; notify(t(app.hasBot ? "set.tokenSaved" : "set.tokenRemoved")); });
  const saveSettings = () => run("settings", async () => { setConfig(await api.saveSettings(lim.img.mb, lim.vid.mb, theme, prompt)); notify(t("set.saved")); });

  const upd = () => run("upd", async () => {
    const u = await check();
    if (!u) return notify(t("upd.latest"));
    if (!(await ask(t("upd.available", { version: u.version, notes: u.body ?? "" }), { kind: "info" }))) return;
    await u.downloadAndInstall();
    await relaunch();
  });
  const exp = () => run("exp", async () => {
    if (!(await ask(t("data.exportWarn"), { kind: "warning" }))) return;
    const path = await save({ defaultPath: "discordette-config.json", filters: [{ name: "JSON", extensions: ["json"] }] });
    if (path) { await api.exportConfig(path); notify(t("data.exported")); }
  });
  const imp = () => run("imp", async () => {
    const path = await open({ filters: [{ name: "JSON", extensions: ["json"] }] });
    if (typeof path !== "string") return;
    const dup = await api.importConflicts(path);
    const replace = dup.length > 0 && (await ask(t("data.dup", { n: dup.length, names: dup.join(", ") }), { kind: "warning" }));
    const s = await api.importConfig(path, replace);
    setConfig(await api.getConfig());
    notify(t("data.imported", { added: s.added, replaced: s.replaced, skipped: s.skipped }));
  });
</script>

<div class="p" class:wide={app.sec === "ai"}>
  {#if app.sec === "general"}
    <h1>{t("nav.general")}</h1>
    {#each [lim.img, lim.vid] as l}
      <h2>{t(l.title)}</h2>
      <div class="row">
        <select aria-label={t(l.title)} style="width:200px" value={l.custom ? "custom" : String(l.mb)}
          onchange={(e) => { const v = e.currentTarget.value; if (v === "custom") l.custom = true; else { l.custom = false; l.mb = +v; saveSettings(); } }}>
          {#each PRESETS as n}<option value={String(n)}>{n} MB</option>{/each}<option value="custom">{t("set.custom")}</option>
        </select>
        {#if l.custom}<input type="number" min="1" max="500" bind:value={l.mb} onchange={saveSettings} style="width:100px" aria-label="MB" /> MB{/if}
      </div>
      <p class="hint">{t(l.hint)}</p>
    {/each}
    <h2>{t("set.theme")}</h2>
    <select bind:value={theme} onchange={saveSettings} aria-label={t("set.theme")} style="width:200px"><option value="system">{t("set.themeSystem")}</option><option value="light">{t("set.themeLight")}</option><option value="dark">{t("set.themeDark")}</option></select>
    <h2>{t("set.language")}</h2>
    <select value={i18n.lang} onchange={(e) => setLang(e.currentTarget.value === "th" ? "th" : "en")} aria-label={t("set.language")} style="width:200px"><option value="en">English</option><option value="th">ไทย</option></select>

  {:else if app.sec === "discord"}
    <h1>Discord</h1>
    <h2>Bot token <span class="hint">{t("dc.optional")}</span></h2>
    <p class="hint">{t("dc.tokenHint", { state: app.hasBot ? t("dc.tokenSet") : "" })}</p>
    <!-- token=null → ยังไม่แตะ: แสดงจุดแทนค่าที่เก็บไว้ (ไม่เคยดึง token จริงมาที่ UI) -->
    <div class="row"><div class="tk"><input type={show ? "text" : "password"} value={token ?? (app.hasBot ? "••••••••••••••••" : "")} oninput={(e) => (token = e.currentTarget.value)}
        onfocus={(e) => token === null && e.currentTarget.select()} placeholder={app.hasBot ? t("dc.phKeep") : t("dc.ph")} autocomplete="off" spellcheck="false" aria-label="bot token" />
        <button class="eye ghost" aria-label={show ? t("dc.hide") : t("dc.show")} aria-pressed={show} onclick={toggleShow}>
          <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
            {#if show}<path d="M3 3l18 18M10.6 6.2A9.8 9.8 0 0 1 12 6c6 0 9.5 6 9.5 6a16 16 0 0 1-3 3.5M6.6 7.6C4 9.3 2.5 12 2.5 12S6 18 12 18c1.4 0 2.7-.3 3.8-.8M9.9 9.9a3 3 0 0 0 4.2 4.2" />
            {:else}<path d="M2.5 12S6 6 12 6s9.5 6 9.5 6-3.5 6-9.5 6-9.5-6-9.5-6z" /><circle cx="12" cy="12" r="3" />{/if}</svg></button></div>
      <button disabled={!!busy || token === null} onclick={saveToken}>{t("dc.save")}</button></div>

    <h2>{t("dc.how")}</h2>
    <ol class="steps">
      <li>{t("dc.s1a")} <button class="ghost lnk" onclick={() => openUrl("https://discord.com/developers/applications")}>Discord Developer Portal</button> → <b>New Application</b> {t("dc.s1b")}</li>
      <li>{t("dc.menu")} <b>Bot</b> → <b>Reset Token</b> → {t("dc.s2b")}</li>
      <li>{t("dc.menu")} <b>General Information</b> → {t("dc.s3b")}<b>Application ID</b>{t("dc.s3c")}</li>
      <li>{t("dc.s4")}</li>
    </ol>
    <div class="row"><input bind:value={appId} inputmode="numeric" placeholder={t("dc.appId")} aria-label="application id" />
      <button class="primary" disabled={!validId} onclick={() => openUrl(inviteUrl)}>{t("dc.invite")}</button></div>
    <p class="hint">{t("dc.perm1")} <b>View Channels</b> {t("dc.perm2")} <b>Manage Channels</b> {t("dc.perm3")} <code>bot</code> {t("dc.perm4")}</p>
    <p class="hint">{t("dc.trouble")}</p>

  {:else if app.sec === "data"}
    <h1>{t("nav.data")}</h1>
    <div class="row"><button disabled={!!busy} onclick={exp}>{t("data.export")}</button><button disabled={!!busy} onclick={imp}>{t("data.import")}</button></div>
    <p class="hint">{t("data.hint")}</p>

  {:else if app.sec === "ai"}
    <h1>{t("nav.provider")}</h1>
    <p class="hint lead">{t("ai.lead")}</p>
    <ProviderSettings />

  {:else if app.sec === "about"}
    <h1>{t("nav.about")}</h1>
    <div class="row"><img class="logo" src={icon} alt="" draggable="false" /><div><b>Discordette</b><div class="hint">{t("about.version", { v: ver })}</div></div></div>
    <div class="row"><button class="primary" disabled={!!busy} onclick={upd}>{busy === "upd" ? t("upd.checking") : t("upd.check")}</button></div>
    <div class="row">
      <button class="ghost lnk" onclick={() => openUrl("https://github.com/MyNightOnline/discordette")}>GitHub</button>
      <button class="ghost lnk" onclick={() => openUrl("https://github.com/MyNightOnline/discordette/releases")}>Releases</button>
    </div>
    <p class="hint">{t("about.license")}</p>

  {:else}
    <h1>{t("nav.prompt")}</h1>
    <textarea rows="14" class="mono" bind:value={prompt} aria-label="prompt template"></textarea>
    <p class="hint">{t("prompt.hint", { rooms: "{{rooms}}", tags: "{{tags}}" })}</p>
    <div class="row"><button class="primary" disabled={!!busy} onclick={saveSettings}>{t("prompt.save")}</button><button onclick={() => (prompt = defaultPrompt)}>{t("prompt.reset")}</button></div>
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
