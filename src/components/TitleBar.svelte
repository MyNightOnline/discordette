<script lang="ts">
  import { t } from "../lib/i18n.svelte.ts";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  const win = getCurrentWindow();
</script>

<header data-tauri-drag-region>
  <div class="ctl">
    <button aria-label={t("win.min")} onclick={() => win.minimize()}>
      <svg width="10" height="10" viewBox="0 0 10 10" stroke="currentColor" fill="none"><path d="M0 5h10" /></svg>
    </button>
    <button aria-label={t("win.max")} onclick={() => win.toggleMaximize()}>
      <svg width="10" height="10" viewBox="0 0 10 10" stroke="currentColor" fill="none"><rect x=".5" y=".5" width="9" height="9" /></svg>
    </button>
    <button class="close" aria-label={t("win.close")} onclick={() => win.close()}>
      <svg width="10" height="10" viewBox="0 0 10 10" stroke="currentColor" fill="none"><path d="M0 0l10 10M10 0L0 10" /></svg>
    </button>
  </div>
</header>

<style>
  header { display: flex; justify-content: flex-end; height: 48px; box-sizing: border-box; flex: none; border-bottom: 1px solid var(--line); user-select: none; }
  .ctl { display: flex; height: 100%; }
  .ctl button { width: 46px; height: 100%; padding: 0; display: grid; place-items: center; border: 0; border-radius: 0; background: transparent; color: var(--text); }
  .ctl button:hover:not(:disabled) { background: color-mix(in srgb, var(--muted) 18%, transparent); }
  .ctl .close:hover:not(:disabled) { background: #e81123; color: #fff; }
</style>
