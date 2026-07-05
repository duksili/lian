<script lang="ts">
  import { api } from "../api";
  import { toast, reportError, bump } from "../state.svelte";
  import { PRECEPT_KEYS, PRECEPT_LABELS } from "../format";

  let { date, existing = null, onsaved }: {
    date: string;
    existing?: any | null;
    onsaved?: () => void;
  } = $props();

  const STATUSES = [
    { key: "observed", label: "observed", cls: "ok" },
    { key: "uncertain", label: "uncertain", cls: "caution" },
    { key: "not_observed", label: "not observed", cls: "" },
    { key: "not_reviewed", label: "not reviewed", cls: "" },
  ];

  let entries = $state<Record<string, { status: string; note: string }>>({});
  let overallNote = $state("");
  let saving = $state(false);
  let noteOpenFor = $state<string | null>(null);

  $effect(() => {
    const e = existing;
    const next: Record<string, { status: string; note: string }> = {};
    for (const k of PRECEPT_KEYS) next[k] = { status: "not_reviewed", note: "" };
    if (e && !e.__empty) {
      for (const en of e.entries ?? []) next[en.precept_key] = { status: en.status, note: en.note ?? "" };
      overallNote = e.overall_note ?? "";
    } else {
      overallNote = "";
    }
    entries = next;
  });

  async function save() {
    saving = true;
    try {
      await api("precepts.save", {
        local_date: date,
        entries: PRECEPT_KEYS.map((k) => ({
          precept_key: k,
          status: entries[k].status,
          note: entries[k].note || null,
        })),
        overall_note: overallNote || null,
      });
      bump();
      toast("Reflection saved", "ok");
      onsaved?.();
    } catch (e) { reportError(e); }
    saving = false;
  }
</script>

<div class="col" style="gap: 10px;">
  <p class="small faint">
    A private reflection — never scored, never counted, never shown in notifications.
  </p>
  {#each PRECEPT_KEYS as key (key)}
    <div class="precept inset">
      <div class="row between">
        <span class="p-label">{PRECEPT_LABELS[key]}</span>
        <div class="row" style="gap:4px;">
          {#each STATUSES as st}
            <button
              class="p-status"
              class:active={entries[key]?.status === st.key}
              class:ok={st.key === "observed" && entries[key]?.status === st.key}
              class:caution={st.key === "uncertain" && entries[key]?.status === st.key}
              onclick={() => (entries[key].status = st.key)}
            >{st.label}</button>
          {/each}
          <button class="btn ghost sm" title="private note"
            onclick={() => (noteOpenFor = noteOpenFor === key ? null : key)}>✎</button>
        </div>
      </div>
      {#if noteOpenFor === key || entries[key]?.note}
        <input class="p-note" bind:value={entries[key].note} placeholder="private note (optional)" />
      {/if}
    </div>
  {/each}
  <label class="field"><span>Overall reflection</span>
    <textarea bind:value={overallNote} placeholder="optional"></textarea>
  </label>
  <button class="btn" style="align-self:flex-end;" onclick={save} disabled={saving}>Save reflection</button>
</div>

<style>
  .precept { padding: 9px 12px; display: flex; flex-direction: column; gap: 8px; }
  .p-label { font-size: 13px; color: var(--paper-dim); }
  .p-status {
    font-size: 11px; padding: 3px 8px; border-radius: 99px;
    color: var(--paper-faint); background: transparent;
    border: 1px solid transparent;
    transition: all 100ms ease;
  }
  .p-status:hover { color: var(--paper); background: var(--ink-3); }
  .p-status.active { background: var(--ink-3); color: var(--paper); border-color: var(--line); }
  .p-status.active.ok { background: var(--ok-wash); color: var(--ok); border-color: transparent; }
  .p-status.active.caution { background: var(--caution-wash); color: var(--caution); border-color: transparent; }
  .p-note { font-size: 12.5px; }
</style>
