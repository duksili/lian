<script lang="ts">
  import type { Snippet } from "svelte";

  let {
    title,
    subtitle = "",
    width = "560px",
    onclose,
    children,
    footer,
  }: {
    title: string;
    subtitle?: string;
    width?: string;
    onclose: () => void;
    children: Snippet;
    footer?: Snippet;
  } = $props();

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      onclose();
    }
  }
</script>

<svelte:window on:keydown={onkeydown} />

<div class="scrim" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) onclose(); }}>
  <div class="modal card" style:max-width={width} role="dialog" aria-modal="true" aria-label={title}>
    <header>
      <div>
        <h2 class="display">{title}</h2>
        {#if subtitle}<p class="small faint">{subtitle}</p>{/if}
      </div>
      <button class="btn ghost sm" onclick={onclose} aria-label="Close">✕</button>
    </header>
    <div class="body">
      {@render children()}
    </div>
    {#if footer}
      <footer>{@render footer()}</footer>
    {/if}
  </div>
</div>

<style>
  .scrim {
    position: fixed; inset: 0; z-index: 100;
    background: rgba(10, 9, 8, 0.66);
    backdrop-filter: blur(3px);
    display: flex; align-items: flex-start; justify-content: center;
    padding: 7vh 24px 24px;
    animation: fade 140ms ease;
  }
  @keyframes fade { from { opacity: 0; } }
  .modal {
    width: 100%;
    max-height: 84vh;
    display: flex; flex-direction: column;
    box-shadow: var(--shadow-2);
    animation: rise 160ms cubic-bezier(0.2, 0.8, 0.3, 1);
  }
  @keyframes rise { from { opacity: 0; transform: translateY(10px); } }
  header {
    display: flex; align-items: flex-start; justify-content: space-between; gap: 12px;
    padding: 18px 20px 12px;
  }
  h2 { font-size: 20px; }
  .body { padding: 4px 20px 18px; overflow-y: auto; }
  footer {
    padding: 12px 20px;
    border-top: 1px solid var(--line-soft);
    display: flex; justify-content: flex-end; gap: 10px;
  }
</style>
