<script lang="ts">
  import { tone } from "../lib/limits.ts";
  import { t } from "../lib/i18n.svelte.ts";
  let { n, max }: { n: number; max: number } = $props();
</script>

<span class="mono c {tone(n, max)}" aria-label={t("counter.aria", { n, max })}>{n} / {max}</span>

<style>
  .c { font-size: 12px; color: var(--muted); }
  .warn { color: var(--warn); }
  .danger { color: var(--danger); font-weight: 600; }
</style>
