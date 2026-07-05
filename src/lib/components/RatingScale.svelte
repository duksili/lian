<script lang="ts">
  let {
    value = null,
    min = 1,
    max = 5,
    anchorLow = "",
    anchorHigh = "",
    onchange,
  }: {
    value?: number | null;
    min?: number;
    max?: number;
    anchorLow?: string;
    anchorHigh?: string;
    onchange: (v: number | null) => void;
  } = $props();

  const steps = $derived(Array.from({ length: max - min + 1 }, (_, i) => min + i));
</script>

<div class="scale">
  <div class="steps" role="radiogroup">
    {#each steps as s}
      <button
        class="step"
        class:active={value === s}
        role="radio"
        aria-checked={value === s}
        onclick={() => onchange(value === s ? null : s)}
        title={value === s ? "Click again to clear" : String(s)}
      >{s}</button>
    {/each}
  </div>
  {#if anchorLow || anchorHigh}
    <div class="anchors small faint">
      <span>{anchorLow}</span>
      <span>{anchorHigh}</span>
    </div>
  {/if}
</div>

<style>
  .scale { display: flex; flex-direction: column; gap: 3px; }
  .steps { display: flex; gap: 5px; }
  .step {
    flex: 1; min-width: 34px; padding: 6px 0;
    border-radius: var(--r-sm);
    background: var(--ink-1); border: 1px solid var(--line);
    font-family: var(--font-mono); font-size: 13px; color: var(--paper-dim);
    transition: all 100ms ease;
  }
  .step:hover { border-color: var(--ink-4); color: var(--paper); }
  .step.active {
    background: var(--cinnabar-wash); border-color: var(--cinnabar-dim);
    color: var(--cinnabar-bright); font-weight: 600;
  }
  .anchors { display: flex; justify-content: space-between; font-size: 11px; }
</style>
