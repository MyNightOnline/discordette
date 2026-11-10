<script lang="ts">
  import { composeText, withSource } from "../lib/limits.ts";
  import { md } from "../lib/markdown.ts";
  import { draft, room as currentRoom } from "../lib/store.svelte.ts";

  const r = $derived(currentRoom());
  const mode = $derived(r?.kind === "forum" ? "forum" : "message");
  const imgs = $derived(draft.slots.filter((s) => s.img).map((s) => ({ u: s.img!.url, v: s.img!.video })));
  const tags = $derived((r?.tags ?? []).filter((t) => draft.tagIds.includes(t.id)));
  const full = $derived(withSource(draft.content, draft.source));
  const body = $derived(r?.kind === "forum" ? full : composeText(draft.title, full));
</script>

<aside class="pv">
  <div class="stage">
    {#if mode === "forum"}
      <div class="fcard">
        <div class="ftext">
          <div class="tags">{#each tags as t}<span class="t">{t.emoji ?? ""} {t.name}</span>{/each}</div>
          <div class="ft">{draft.title || "หัวข้อโพสต์"}</div>
          <div class="fc">{@html md(full) || "<span class='dim'>รายละเอียด…</span>"}</div>
        </div>
        {#if imgs[0]}{#if imgs[0].v}<video src={imgs[0].u + "#t=0.1"} muted preload="metadata"></video>{:else}<img src={imgs[0].u} alt="รูปปก" />{/if}{/if}
      </div>
    {:else}
      <div class="msg">
        <div class="av">{(r?.name ?? "?").slice(0, 1).toUpperCase()}</div>
        <div class="mb">
          <div class="who">{r?.name ?? "webhook"} <span class="bot">BOT</span></div>
          <div class="mc">{@html md(body) || "<span class='dim'>ข้อความ…</span>"}</div>
          {#if imgs.length}
            <div class="ig n{Math.min(imgs.length, 4)}">{#each imgs.slice(0, 10) as m}{#if m.v}<video src={m.u + "#t=0.1"} muted preload="metadata"></video>{:else}<img src={m.u} alt="" decoding="async" />{/if}{/each}</div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
</aside>

<style>
  .pv { padding: 16px; }
  .stage { background: #313338; color: #dbdee1; border-radius: var(--r-card); padding: 16px; min-height: 160px; font-size: 14px; }
  .stage :global(.dim) { color: #80848e; }
  .stage :global(code) { background: #1e1f22; padding: 0 4px; border-radius: 4px; font-family: var(--mono); }
  .stage :global(.sp) { background: #1e1f22; color: transparent; border-radius: 3px; padding: 0 2px; }
  .stage :global(.sp:hover) { background: #3f4147; color: inherit; }
  .stage :global(pre) { background: #1e1f22; padding: 6px 8px; border-radius: 4px; margin: 4px 0; white-space: pre-wrap; }
  .stage :global(pre code) { padding: 0; }
  .stage :global(.q) { display: block; border-left: 3px solid #4e5058; padding-left: 10px; }
  .stage :global(a) { color: #00a8fc; }
  .fcard { display: flex; gap: 12px; background: #2b2d31; border-radius: var(--r); padding: 12px; }
  .ftext { flex: 1; min-width: 0; }
  .fcard img, .fcard video { width: 96px; height: 96px; object-fit: cover; border-radius: 6px; }
  .tags { display: flex; gap: 4px; flex-wrap: wrap; margin-bottom: 4px; }
  .t { background: #3f4147; border-radius: 4px; padding: 0 6px; font-size: 12px; }
  .ft { font-weight: 600; color: #f2f3f5; overflow-wrap: anywhere; }
  .fc { display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; color: #b5bac1; overflow-wrap: anywhere; }
  .msg { display: flex; gap: 12px; }
  .av { flex: none; width: 40px; height: 40px; border-radius: 50%; background: #5865f2; display: grid; place-items: center; font-weight: 600; color: #fff; }
  .mb { min-width: 0; flex: 1; }
  .who { font-weight: 600; color: #f2f3f5; }
  .bot { background: #5865f2; color: #fff; font-size: 10px; border-radius: 3px; padding: 0 4px; vertical-align: middle; }
  .mc { overflow-wrap: anywhere; }
  .ig { display: grid; gap: 4px; margin-top: 8px; max-width: 400px; }
  .ig img, .ig video { width: 100%; height: 100%; object-fit: cover; border-radius: 6px; aspect-ratio: 16/10; }
  .ig.n1 { grid-template-columns: 1fr; } .ig.n1 img, .ig.n1 video { aspect-ratio: auto; max-height: 300px; }
  .ig.n2, .ig.n4 { grid-template-columns: 1fr 1fr; }
  .ig.n3 { grid-template-columns: 1fr 1fr; } .ig.n3 :first-child { grid-row: span 2; aspect-ratio: auto; }
</style>
