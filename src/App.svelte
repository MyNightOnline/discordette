<script lang="ts">
  import { init, app, draft, editor, notify, setConfig, type Sec } from "./lib/store.svelte.ts";
  import { api, errMsg } from "./lib/api.ts";
  import { check } from "@tauri-apps/plugin-updater";
  import { relaunch } from "@tauri-apps/plugin-process";
  import { ask } from "@tauri-apps/plugin-dialog";
  import RoomEditor from "./components/RoomEditor.svelte";
  import Toast from "./components/Toast.svelte";
  import TitleBar from "./components/TitleBar.svelte";
  import icon from "./assets/icon.png";
  import Compose from "./views/Compose.svelte";
  import Settings from "./views/Settings.svelte";

  $effect(() => { init(); autoUpdate(); });
  // เช็กอัปเดตเงียบๆ ตอนเปิดแอป เจอแล้วค่อยถาม; เน็ตล่ม/ยังไม่มี release ก็ข้าม
  async function autoUpdate() {
    try {
      const u = await check();
      if (u && await ask(`มีเวอร์ชันใหม่ ${u.version}\n\n${u.body ?? ""}\n\nอัปเดตตอนนี้ไหม?`, { kind: "info" })) { await u.downloadAndInstall(); await relaunch(); }
    } catch { /* ignore */ }
  }
  const MIN_W = 220, MAX_W = 520;
  let navW = $state(MIN_W);
  try { navW = Math.min(MAX_W, Math.max(MIN_W, +(localStorage.getItem("navW") ?? MIN_W))); } catch { /* ไม่มี storage ก็ใช้ค่าเริ่มต้น */ }
  function drag(e: PointerEvent) {
    const el = e.currentTarget as HTMLElement; el.setPointerCapture(e.pointerId);
    const move = (m: PointerEvent) => { navW = Math.min(MAX_W, Math.max(MIN_W, m.clientX)); };
    const up = () => { el.removeEventListener("pointermove", move); el.removeEventListener("pointerup", up); try { localStorage.setItem("navW", String(navW)); } catch { /* ignore */ } };
    el.addEventListener("pointermove", move); el.addEventListener("pointerup", up);
  }
  // ลากสลับลำดับห้อง (ใช้ pointer events เพราะ HTML5 drag ชนกับ drop ไฟล์ของ Tauri)
  let dragId = $state(""), overIdx = $state(-1);
  function rowDown(e: PointerEvent, id: string) {
    if (e.button !== 0) return;
    const sy = e.clientY; let on = false;
    const move = (m: PointerEvent) => {
      if (!on && Math.abs(m.clientY - sy) < 5) return;
      on = true; dragId = id;
      const rs = [...document.querySelectorAll<HTMLElement>(".roomrow")];
      const i = rs.findIndex((r) => m.clientY < r.getBoundingClientRect().top + r.offsetHeight / 2);
      overIdx = i < 0 ? rs.length : i;
    };
    const up = async () => {
      window.removeEventListener("pointermove", move); window.removeEventListener("pointerup", up);
      const to0 = overIdx; dragId = ""; overIdx = -1;
      if (!on) return;
      const swallow = (c: Event) => c.stopPropagation(); // กันคลิกเด้งเปลี่ยนห้องหลังปล่อย
      window.addEventListener("click", swallow, { capture: true, once: true }); setTimeout(() => window.removeEventListener("click", swallow, true), 0);
      const ids = app.config.rooms.map((r) => r.id), from = ids.indexOf(id), to = to0 > from ? to0 - 1 : to0;
      if (from < 0 || to === from) return;
      ids.splice(to, 0, ids.splice(from, 1)[0]);
      const prev = app.config.rooms;
      app.config.rooms = ids.map((i) => prev.find((r) => r.id === i)!);
      try { setConfig(await api.reorderRooms(ids)); } catch (err) { app.config.rooms = prev; notify(errMsg(err), undefined, true); }
    };
    window.addEventListener("pointermove", move); window.addEventListener("pointerup", up);
  }
  const SECS: { grp: string; items: [Sec, string][] }[] = [
    { grp: "พื้นฐาน", items: [["general", "ทั่วไป"], ["discord", "Discord"], ["data", "นำเข้า / ส่งออก"]] },
    { grp: "AI", items: [["ai", "AI provider"], ["prompt", "Prompt template"]] },
    { grp: "ข้อมูล", items: [["about", "เกี่ยวกับ"]] },
  ];
</script>

<div class="shell" style="grid-template-columns: {navW}px 1fr">
  <nav>
    <div class="logo" data-tauri-drag-region><img class="mark" src={icon} alt="" draggable="false" />Discordette</div>
    {#if app.view === "settings"}
      <button class="item back" onclick={() => (app.view = "compose")}>
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m15 6-6 6 6 6" /></svg>
        กลับไปโพสต์
      </button>
      {#each SECS as g}
        <div class="grp">{g.grp}</div>
        {#each g.items as [id, label]}
          <button class="item" class:on={app.sec === id} aria-current={app.sec === id} onclick={() => (app.sec = id)}>{label}</button>
        {/each}
      {/each}
    {:else}
    <div class="grph"><span>เขียนโพสต์ลงห้อง</span>
      <button class="gear plus" aria-label="เพิ่มห้องใหม่" title="เพิ่มห้องใหม่" onclick={() => { editor.room = undefined; editor.open = true; }}>
        <svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" aria-hidden="true"><path d="M8 3v10M3 8h10" /></svg>
      </button>
    </div>
    {#each app.config.rooms as r, i (r.id)}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div class="roomrow" class:dragging={dragId === r.id} class:before={dragId && overIdx === i} class:after={dragId && overIdx === app.config.rooms.length && i === overIdx - 1} onpointerdown={(e) => rowDown(e, r.id)}>
        <button class="item" class:on={app.view === "compose" && app.roomId === r.id} onclick={() => { if (app.roomId !== r.id) draft.tagIds = []; app.roomId = r.id; app.view = "compose"; }}>
          {#if r.kind === "forum"}
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path d="M18.91 12.98a5.45 5.45 0 0 1 2.18 6.2c-.1.33-.09.68.1.96l.83 1.32a1 1 0 0 1-.84 1.54h-5.5A5.6 5.6 0 0 1 10 17.5a5.6 5.6 0 0 1 5.68-5.5c1.2 0 2.32.36 3.23.98Z" /><path d="M19.24 10.86c.32.16.72-.02.74-.38L20 10c0-4.42-4.03-8-9-8s-9 3.58-9 8c0 1.5.47 2.91 1.28 4.11.14.21.12.49-.06.67l-1.51 1.51A1 1 0 0 0 2.4 18h5.1a.5.5 0 0 0 .49-.5c0-4.2 3.5-7.5 7.68-7.5 1.28 0 2.5.3 3.56.86Z" /></svg>
          {:else}
            <svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" aria-hidden="true"><path fill-rule="evenodd" clip-rule="evenodd" d="M10.99 3.16A1 1 0 1 0 9 2.84L8.15 8H4a1 1 0 0 0 0 2h3.82l-.67 4H3a1 1 0 1 0 0 2h3.82l-.8 4.84a1 1 0 0 0 1.97.32L8.85 16h4.97l-.8 4.84a1 1 0 0 0 1.97.32l.86-5.16H20a1 1 0 1 0 0-2h-3.82l.67-4H21a1 1 0 1 0 0-2h-3.82l.8-4.84a1 1 0 1 0-1.97-.32L15.15 8h-4.97l.8-4.84ZM14.15 14l.67-4H9.85l-.67 4h4.97Z" /></svg>
          {/if}
          <span class="nm" title={r.name}>{r.name}</span>
        </button>
        <button class="gear" aria-label="ตั้งค่าห้อง {r.name}" onclick={() => { editor.room = r; editor.open = true; }}>
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" /></svg>
        </button>
      </div>
    {/each}
    <button class="item settings" onclick={() => (app.view = "settings")}>
      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" /></svg>
      ตั้งค่า
    </button>
    {/if}
    <div class="grip" role="separator" aria-orientation="vertical" aria-label="ปรับความกว้างแถบข้าง" onpointerdown={drag}></div>
  </nav>
  <div class="right">
    <TitleBar />
    <main>
      {#if app.view === "compose"}<Compose />{:else}<Settings />{/if}
    </main>
  </div>
</div>
<RoomEditor />
<Toast />

<style>
  .shell { display: grid; height: 100%; }
  .grip { position: absolute; top: 0; bottom: 0; right: 0; width: 5px; cursor: col-resize; z-index: 5; touch-action: none; }
  .grip:hover { background: color-mix(in srgb, var(--accent) 35%, transparent); }
  nav { position: relative; background: var(--surface); border-right: 1px solid var(--line); padding: 0 12px; display: flex; flex-direction: column; gap: 2px; overflow-y: auto; overflow-x: hidden; scroll-behavior: smooth; overscroll-behavior: contain; scrollbar-width: thin; }
  .right { min-width: 0; min-height: 0; display: flex; flex-direction: column; }
  main { flex: 1; min-height: 0; overflow: auto; }
  .logo { position: sticky; top: 0; z-index: 2; background: var(--surface); height: 48px; flex: none; margin: 0 -12px 4px; padding: 0 20px; font-weight: 600; font-size: 15px; display: flex; gap: 10px; align-items: center; user-select: none; }
  .mark { width: 26px; height: 26px; pointer-events: none; }
  .item { display: flex; gap: 8px; align-items: center; text-align: left; background: transparent; border-color: transparent; }
  .item.on { background: var(--accent-soft); color: var(--accent); }
  .item:hover:not(:disabled), .gear:hover:not(:disabled) { border-color: transparent; background: color-mix(in srgb, var(--muted) 14%, transparent); }
  .item.on:hover:not(:disabled) { background: var(--accent-soft); }
  .grp { margin: 16px 8px 4px; color: var(--muted); font-size: 12px; }
  .nm { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .roomrow { position: relative; display: flex; user-select: none; }
  .roomrow.dragging { opacity: .45; }
  .roomrow.before::before, .roomrow.after::after { content: ""; position: absolute; left: 4px; right: 4px; height: 2px; border-radius: 1px; background: var(--accent); }
  .roomrow.before::before { top: -2px; }
  .roomrow.after::after { bottom: -2px; }
  .roomrow .item { flex: 1; min-width: 0; padding-right: 34px; }
  .gear { position: absolute; right: 4px; top: 50%; transform: translateY(-50%); display: flex; align-items: center; justify-content: center; width: 26px; height: 26px; padding: 0; background: transparent; border-color: transparent; color: var(--muted); opacity: 0; }
  .roomrow:hover .gear, .gear:focus-visible { opacity: 1; }
  .gear:hover:not(:disabled) { color: var(--accent); }
  .back { color: var(--muted); }
  .grph { display: flex; align-items: center; justify-content: space-between; margin: 16px 4px 4px 8px; color: var(--muted); font-size: 12px; }
  .gear.plus { position: static; transform: none; opacity: 1; }
  .back { margin-top: 4px; }
  .settings { margin-top: auto; position: sticky; bottom: 8px; margin-bottom: 8px; z-index: 2; background: var(--surface); box-shadow: 0 8px 0 var(--surface); flex: none; }
</style>
