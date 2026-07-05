<script lang="ts">
  import { onMount } from "svelte";
  import { app, nav, go, loadGlobals, toasts, quickLog, timer, reportError } from "./lib/state.svelte";
  import type { Route } from "./lib/state.svelte";
  import Today from "./lib/views/Today.svelte";
  import Timeline from "./lib/views/Timeline.svelte";
  import Calendar from "./lib/views/Calendar.svelte";
  import Assessments from "./lib/views/Assessments.svelte";
  import Determinations from "./lib/views/Determinations.svelte";
  import Review from "./lib/views/Review.svelte";
  import Research from "./lib/views/Research.svelte";
  import Settings from "./lib/views/Settings.svelte";
  import Onboarding from "./lib/views/Onboarding.svelte";
  import QuickLog from "./lib/views/QuickLog.svelte";
  import { fmtDuration } from "./lib/format";

  const items: { route: Route["name"]; label: string; glyph: string; key: string }[] = [
    { route: "today", label: "Today", glyph: "◉", key: "1" },
    { route: "timeline", label: "Timeline", glyph: "≡", key: "2" },
    { route: "calendar", label: "Calendar", glyph: "▦", key: "3" },
    { route: "assessments", label: "Assess", glyph: "◔", key: "4" },
    { route: "determinations", label: "Determinations", glyph: "❖", key: "5" },
    { route: "review", label: "Review", glyph: "☷", key: "6" },
    { route: "research", label: "Research", glyph: "∴", key: "7" },
    { route: "settings", label: "Settings", glyph: "⚙", key: "8" },
  ];

  let ready = $state(false);

  onMount(async () => {
    try {
      await loadGlobals();
      if (!app.settings.onboarding_complete) go({ name: "onboarding" });
      ready = true;
    } catch (e) {
      reportError(e);
      ready = true;
    }
  });

  function onkeydown(e: KeyboardEvent) {
    const target = e.target as HTMLElement;
    const typing = ["INPUT", "TEXTAREA", "SELECT"].includes(target?.tagName) || target?.isContentEditable;
    if (typing || quickLog.open || nav.route.name === "onboarding") return;
    if (e.key === "l" && !e.ctrlKey && !e.metaKey) {
      e.preventDefault();
      quickLog.open = true;
    }
    const item = items.find((i) => i.key === e.key);
    if (item && !e.ctrlKey && !e.metaKey) go({ name: item.route } as Route);
  }
</script>

<svelte:window on:keydown={onkeydown} />

{#if !ready}
  <div class="boot"><span class="display">練</span></div>
{:else if nav.route.name === "onboarding"}
  <Onboarding />
{:else}
  <div class="shell">
    <nav class="rail" aria-label="Primary">
      <div class="brand display" title="LIAN — practice, refined">練<span class="brand-word">LIAN</span></div>
      <div class="rail-items">
        {#each items as item}
          <button
            class="rail-item"
            class:active={nav.route.name === item.route}
            onclick={() => go({ name: item.route } as Route)}
            title="{item.label}  ({item.key})"
          >
            <span class="glyph" aria-hidden="true">{item.glyph}</span>
            <span class="label">{item.label}</span>
          </button>
        {/each}
      </div>
      <div class="rail-foot">
        {#if timer.running}
          <button class="timer-chip mono" onclick={() => (quickLog.open = true)} title="Practice timer running — click to finish">
            ● {fmtDuration(timer.elapsed)}
          </button>
        {/if}
        <button class="btn primary log-btn" onclick={() => (quickLog.open = true)}>
          Log <kbd>L</kbd>
        </button>
      </div>
    </nav>

    <main class="content">
      {#if nav.route.name === "today"}<Today />
      {:else if nav.route.name === "timeline"}<Timeline />
      {:else if nav.route.name === "calendar"}<Calendar />
      {:else if nav.route.name === "assessments"}<Assessments />
      {:else if nav.route.name === "determinations"}<Determinations />
      {:else if nav.route.name === "review"}<Review />
      {:else if nav.route.name === "research"}<Research />
      {:else if nav.route.name === "settings"}<Settings />
      {/if}
    </main>
  </div>
{/if}

{#if quickLog.open}
  <QuickLog />
{/if}

<div class="toasts" aria-live="polite">
  {#each toasts.items as t (t.id)}
    <div class="toast" class:ok={t.kind === "ok"} class:error={t.kind === "error"}>{t.text}</div>
  {/each}
</div>

<style>
  .boot {
    height: 100%; display: flex; align-items: center; justify-content: center;
    font-size: 42px; color: var(--paper-ghost);
    animation: breathe 2.4s ease-in-out infinite;
  }
  @keyframes breathe { 50% { opacity: 0.4; } }

  .shell { display: flex; height: 100%; }

  .rail {
    width: 200px; flex: none;
    display: flex; flex-direction: column;
    background: var(--ink-1);
    border-right: 1px solid var(--line-soft);
    padding: 18px 12px 14px;
    gap: 18px;
  }
  .brand {
    font-size: 21px; padding: 0 10px;
    display: flex; align-items: baseline; gap: 9px;
    color: var(--cinnabar);
  }
  .brand-word {
    font-family: var(--font-ui); font-size: 12px; font-weight: 650;
    letter-spacing: 0.22em; color: var(--paper-dim);
  }
  .rail-items { display: flex; flex-direction: column; gap: 2px; flex: 1; }
  .rail-item {
    display: flex; align-items: center; gap: 11px;
    padding: 8px 10px; border-radius: var(--r-sm);
    color: var(--paper-dim); font-size: 13.5px; font-weight: 520;
    text-align: left; transition: background 100ms ease, color 100ms ease;
  }
  .rail-item:hover { background: var(--ink-2); color: var(--paper); }
  .rail-item.active {
    background: var(--ink-3); color: var(--paper);
    box-shadow: inset 2px 0 0 var(--cinnabar);
  }
  .rail-item .glyph { width: 18px; text-align: center; opacity: 0.8; font-size: 14px; }
  .rail-foot { display: flex; flex-direction: column; gap: 8px; }
  .timer-chip {
    font-size: 12.5px; color: var(--ok);
    background: var(--ok-wash); border-radius: var(--r-sm);
    padding: 6px 10px; text-align: center;
    animation: breathe 2.4s ease-in-out infinite;
  }
  .log-btn { justify-content: space-between; }

  .content { flex: 1; min-width: 0; overflow-y: auto; }

  @media (max-width: 1120px) {
    .rail { width: 64px; padding: 18px 8px 14px; }
    .brand-word, .rail-item .label { display: none; }
    .rail-item { justify-content: center; }
    .rail-item .glyph { width: auto; }
    .brand { justify-content: center; padding: 0; }
    .log-btn { padding: 7px; }
    .log-btn kbd { display: none; }
  }

  .toasts {
    position: fixed; bottom: 18px; left: 50%; transform: translateX(-50%);
    display: flex; flex-direction: column; gap: 8px; z-index: 300;
    align-items: center;
  }
  .toast {
    background: var(--ink-3); border: 1px solid var(--line);
    color: var(--paper); font-size: 13px;
    padding: 9px 16px; border-radius: var(--r-md);
    box-shadow: var(--shadow-2);
    animation: rise-toast 180ms cubic-bezier(0.2, 0.8, 0.3, 1);
    max-width: 480px;
  }
  .toast.ok { border-color: var(--ok); }
  .toast.error { border-color: var(--invalid); }
  @keyframes rise-toast { from { opacity: 0; transform: translateY(8px); } }
</style>
