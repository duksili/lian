<script lang="ts">
  import type { Snippet } from "svelte";
  let {
    glyph = "◦",
    title,
    body = "",
    children,
  }: { glyph?: string; title: string; body?: string; children?: Snippet } = $props();
</script>

<div class="empty">
  <div class="glyph" aria-hidden="true">{glyph}</div>
  <p class="title display">{title}</p>
  {#if body}<p class="small dim">{body}</p>{/if}
  {#if children}
    <div class="actions">{@render children()}</div>
  {/if}
</div>

<style>
  .empty {
    display: flex; flex-direction: column; align-items: center; gap: 6px;
    padding: 42px 24px; text-align: center;
  }
  .glyph {
    font-size: 26px; color: var(--paper-ghost);
    width: 52px; height: 52px; border-radius: 99px;
    display: flex; align-items: center; justify-content: center;
    border: 1px dashed var(--ink-4);
    margin-bottom: 6px;
  }
  .title { font-size: 17px; color: var(--paper-dim); }
  p { max-width: 380px; }
  .actions { margin-top: 12px; display: flex; gap: 10px; }
</style>
