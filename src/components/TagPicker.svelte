<script lang="ts">
  import type { Room } from "../lib/api.ts";
  import { MAX_TAGS } from "../lib/limits.ts";
  import { api, errMsg } from "../lib/api.ts";
  import { app, draft, editor, notify, setConfig } from "../lib/store.svelte.ts";
  import Counter from "./Counter.svelte";
  let { room, onchange }: { room: Room; onchange: () => void } = $props();

  let busy = $state(false);
  // ดึงแท็กได้เลยจากตรงนี้ ถ้ายังไม่พร้อม (ไม่มี token / ยังไม่เคยทดสอบห้อง) ให้เปิดหน้าแก้ไขห้องซึ่งบอกสาเหตุ
  async function pull() {
    if (!app.hasBot || !room.channel_id) { editor.room = room; editor.open = true; return; }
    busy = true;
    try { setConfig(await api.fetchTags(room.id)); notify("ดึงแท็กแล้ว"); } catch (e) { notify(errMsg(e), undefined, true); } finally { busy = false; }
  }

  // สร้างแท็กใหม่ใน Discord (ต้องให้บอทมีสิทธิ์ Manage Channels) แล้วเลือกให้เลย
  let name = $state(""), creating = $state(false);
  const canCreate = $derived(app.hasBot && !!room.channel_id);
  async function create(n: string) {
    n = n.trim();
    if (!n || creating) return;
    creating = true;
    try {
      const c = await api.createTag(room.id, n);
      setConfig(c);
      const t = c.rooms.find((r) => r.id === room.id)?.tags.find((t) => t.name.toLowerCase() === n.toLowerCase());
      if (t && !draft.tagIds.includes(t.id) && draft.tagIds.length < MAX_TAGS) draft.tagIds.push(t.id);
      draft.suggest = draft.suggest.filter((x) => x.toLowerCase() !== n.toLowerCase());
      name = ""; onchange(); notify(`สร้างแท็ก "${n}" แล้ว`);
    } catch (e) { notify(errMsg(e), undefined, true); } finally { creating = false; }
  }

  function toggle(id: string) {
    const i = draft.tagIds.indexOf(id);
    if (i >= 0) draft.tagIds.splice(i, 1);
    else if (draft.tagIds.length < MAX_TAGS) draft.tagIds.push(id);
    onchange();
  }
</script>

<div class="f" id="f-tags">
  <div class="lbl">
    <span>แท็ก {#if room.require_tag}<span class="req">ต้องเลือก</span>{/if}</span>
    <Counter n={draft.tagIds.length} max={MAX_TAGS} />
  </div>
  {#if room.tags.length}
    <div class="row wrap">
      {#each room.tags as t (t.id)}
        {@const on = draft.tagIds.includes(t.id)}
        <button class="chip" class:on disabled={!on && draft.tagIds.length >= MAX_TAGS} aria-pressed={on} onclick={() => toggle(t.id)}>
          {#if t.emoji}{t.emoji} {/if}{t.name}
        </button>
      {/each}
    </div>
  {:else}
    <p class="hint">ยังไม่มีแท็กของห้องนี้ — <button class="ghost link" disabled={busy} onclick={pull}>{busy ? "กำลังดึง…" : "ดึงแท็ก"}</button></p>
  {/if}
  {#if canCreate}
    <div class="row wrap new">
      {#each draft.suggest as n (n)}
        <button class="chip sug" disabled={creating} title="AI เสนอแท็กนี้ แต่ห้องยังไม่มี · กดเพื่อสร้างใน Discord" onclick={() => create(n)}>＋ สร้าง "{n}"</button>
      {/each}
      <input class="nt" bind:value={name} maxlength="20" placeholder="สร้างแท็กใหม่…" aria-label="ชื่อแท็กใหม่" onkeydown={(e) => e.key === "Enter" && create(name)} />
      <button disabled={creating || !name.trim()} onclick={() => create(name)}>{creating ? "กำลังสร้าง…" : "สร้าง"}</button>
    </div>
  {/if}
</div>

<style>
  .new { margin-top: 8px; align-items: center; }
  .nt { width: 160px; }
  .sug { border-style: dashed; }
  .wrap { flex-wrap: wrap; }
  .req { background: var(--danger-bg); color: var(--danger); border-radius: 4px; padding: 0 6px; font-size: 12px; margin-left: 6px; }
  .link { color: var(--accent); padding: 0; text-decoration: underline; }
</style>
