<script lang="ts">
  import { ask, open } from "@tauri-apps/plugin-dialog";
  import { api, errMsg } from "../lib/api.ts";
  import { addFiles, addPaths, draft, media, mediaAccept, moveSlot, notify, removeSlot, toggleMedia } from "../lib/store.svelte.ts";

  let picker: HTMLInputElement;
  // ใช้ตัวเลือกโฟลเดอร์ของระบบ (ไม่มี popup "Upload N files" ของ WebView) แล้วถามยืนยันก่อนเพิ่ม
  async function pickFolder() {
    const dir = await open({ directory: true, title: "เลือกโฟลเดอร์ที่มีรูป/คลิป" });
    if (typeof dir !== "string") return;
    try {
      const { files, skipped } = await api.listFolder(dir, media.image, media.video);
      if (!files.length) { notify("ไม่พบไฟล์ที่ตรงกับที่ติ๊กไว้ในโฟลเดอร์นี้", undefined, true); return; }
      const v = files.filter((f) => f.video).length, n = files.length - v;
      const msgs = Math.ceil((draft.slots.length + files.length) / 10);
      const lines = [
        `พบ ${[n && `รูป ${n}`, v && `คลิป ${v}`].filter(Boolean).join(" + ")} ไฟล์ ใน "${dir.split(/[\\/]/).pop()}"`,
        skipped ? `ข้ามไฟล์อื่นที่ไม่ใช่รูป/คลิป ${skipped} ไฟล์` : "",
        msgs > 1 ? `เมื่อโพสต์จะแบ่งส่งประมาณ ${msgs} ข้อความ (ข้อความละ 10 ไฟล์)` : "",
      ].filter(Boolean);
      const ok = await ask(lines.join("\n") + "\n\nเพิ่มทั้งหมดเข้ารายการไหม?", { title: "เพิ่มทั้งโฟลเดอร์", kind: "info", okLabel: "เพิ่ม", cancelLabel: "ยกเลิก" });
      if (ok) await addPaths(files.sort((x, y) => x.path.localeCompare(y.path, undefined, { numeric: true })));
    } catch (e) { notify(errMsg(e), undefined, true); }
  }
  let dragFrom = $state(-1);
  let over = $state(false);
  const mb = (n: number) => (n / 1e6).toFixed(1);

  function drop(e: DragEvent, to: number) {
    e.preventDefault(); over = false;
    if (dragFrom >= 0) moveSlot(dragFrom, to);
    else if (e.dataTransfer?.files.length) addFiles([...e.dataTransfer.files]);
    dragFrom = -1;
  }
  function key(e: KeyboardEvent, i: number) {
    if (!e.altKey) return;
    const to = e.key === "ArrowLeft" ? i - 1 : e.key === "ArrowRight" ? i + 1 : -1;
    if (to < 0) return;
    e.preventDefault(); moveSlot(i, to);
    queueMicrotask(() => (document.querySelectorAll<HTMLElement>(".tile")[Math.min(to, draft.slots.length - 1)])?.focus());
  }
</script>

<div class="f" id="f-images">
  <div class="lbl"><span>{media.image && media.video ? "รูป / คลิป" : media.image ? "รูป" : "คลิป"}</span>
    <span class="r">
      <label class="chk"><input type="checkbox" checked={media.image} onchange={() => toggleMedia("image")} />รูป</label>
      <label class="chk"><input type="checkbox" checked={media.video} onchange={() => toggleMedia("video")} />คลิป</label>
      <button class="ghost lnk" onclick={pickFolder}>เลือกทั้งโฟลเดอร์</button><span class="mono cnt">{draft.slots.length} ไฟล์</span></span></div>
  <div class="grid" class:over role="group" aria-label="ไฟล์ที่แนบ"
    ondragover={(e) => { e.preventDefault(); if (dragFrom < 0) over = true; }} ondragleave={() => (over = false)} ondrop={(e) => drop(e, draft.slots.length - 1)}>
    {#each draft.slots as s, i (s.key)}
      {@const bad = !!s.error}
      <!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
      <div class="tile" class:cover={i === 0} class:bad tabindex="0" role="listitem" draggable="true"
        aria-label="ไฟล์ที่ {i + 1} {s.name}" ondragstart={() => (dragFrom = i)} ondragend={() => (dragFrom = -1)}
        ondragover={(e) => e.preventDefault()} ondrop={(e) => { e.stopPropagation(); drop(e, i); }} onkeydown={(e) => key(e, i)}>
        {#if s.img}
          {#if s.img.video}
            <video src={s.img.url + "#t=0.1"} muted preload="metadata" playsinline draggable="false"></video>
            <span class="play" aria-hidden="true">▶</span>
          {:else}<img src={s.img.url} alt={s.name} loading="lazy" decoding="async" draggable="false" />{/if}
          <span class="badge" class:shrunk={s.img.shrunk}>{s.img.shrunk ? `ย่อแล้ว ${mb(s.img.original_size)}→${mb(s.img.size)}MB` : `${mb(s.img.size)}MB`}</span>
        {:else if s.error}
          <span class="err">{s.error}</span>
        {:else}<div class="skel"></div>{/if}
        {#if i === 0 && s.img}<span class="tag">ปก</span>{/if}
        <button class="x" aria-label="ลบ {s.name}" onclick={() => removeSlot(s.key)}>✕</button>
      </div>
    {/each}
    <button class="tile add" onclick={() => picker.click()} aria-label="เพิ่มไฟล์">+</button>
  </div>
  <input bind:this={picker} type="file" accept={mediaAccept()} multiple hidden onchange={(e) => { addFiles([...(e.currentTarget.files ?? [])]); e.currentTarget.value = ""; }} />
</div>

<style>
  .cnt { font-size: 12px; color: var(--muted); }
  .r { display: flex; align-items: center; gap: 12px; }
  .lnk { color: var(--accent); padding: 0; text-decoration: underline; font-size: 12px; }
  .grid { display: grid; grid-template-columns: repeat(5, 1fr); gap: 8px; padding: 6px; border-radius: var(--r); border: 1px dashed transparent; }
  .grid.over { border-color: var(--accent); background: var(--accent-soft); }
  .tile { position: relative; aspect-ratio: 1; border-radius: var(--r); overflow: hidden; background: var(--raised); border: 1px solid var(--line); padding: 0; }
  .tile.cover { border: 2px solid var(--accent); }
  .tile.bad { border-color: var(--danger); }
  .tile img, .tile video { width: 100%; height: 100%; object-fit: cover; display: block; }
  .play { position: absolute; left: 50%; top: 50%; transform: translate(-50%, -50%); width: 30px; height: 30px; border-radius: 50%; display: grid; place-items: center; background: rgba(0,0,0,.6); color: #fff; font-size: 12px; pointer-events: none; }
  .chk { display: inline-flex; align-items: center; gap: 4px; font-size: 12px; color: var(--text); cursor: pointer; }
  .add { font-size: 28px; color: var(--muted); }
  .badge { position: absolute; left: 4px; bottom: 4px; font-size: 10px; background: rgba(0,0,0,.7); color: #7ee2a8; border-radius: 4px; padding: 0 5px; max-width: calc(100% - 8px); }
  .badge.shrunk { color: #ffd27a; }
  .tag { position: absolute; left: 4px; top: 4px; background: var(--accent); color: var(--accent-fg); font-size: 11px; border-radius: 4px; padding: 0 6px; }
  .err { display: block; padding: 6px; font-size: 11px; color: var(--danger); }
  .x { position: absolute; right: 4px; top: 4px; padding: 0 6px; opacity: 0; background: rgba(0,0,0,.7); color: #fff; border: 0; }
  .tile:hover .x, .tile:focus-within .x { opacity: 1; }
  .skel { width: 100%; height: 100%; background: linear-gradient(90deg, var(--raised), var(--line), var(--raised)); background-size: 200% 100%; animation: sh 1.2s infinite linear; }
  @keyframes sh { to { background-position: -200% 0; } }
</style>
