<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { app } from "../lib/store.svelte.ts";
</script>

{#if app.toast}
  <div class="toast" class:err={app.toast.err} role="status">
    <span class="dot" aria-hidden="true"></span><span>{app.toast.msg}</span>
    {#if app.toast.link}<button class="primary" onclick={() => openUrl(app.toast!.link!)}>เปิดใน Discord</button>{/if}
    <button class="ghost" aria-label="ปิด" onclick={() => (app.toast = null)}>✕</button>
  </div>
{/if}

<style>
  .toast { position: fixed; right: 24px; bottom: 80px; z-index: 50; display: flex; gap: 12px; align-items: center; max-width: 480px;
    background: var(--raised); border: 1px solid var(--line); border-radius: var(--r); padding: 10px 12px; box-shadow: var(--shadow); }
  .dot { flex: none; width: 8px; height: 8px; border-radius: 50%; background: var(--ok); }
  .err .dot { background: var(--danger); }
</style>
