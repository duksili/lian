<script lang="ts">
  import { api } from "../api";
  import { dataVersion, openQuickLog, reportError, bump, toast } from "../state.svelte";
  import { fmtDate, fmtDuration, fmtTime, todayStr, addDays, PRECEPT_LABELS } from "../format";
  import ContextModal from "./ContextModal.svelte";
  import EmptyState from "../components/EmptyState.svelte";

  let rangeDays = $state(14);
  let events = $state<any[]>([]);
  let checkins = $state<any[]>([]);
  let precepts = $state<any[]>([]);
  let contexts = $state<any[]>([]);
  let plans = $state<any[]>([]);
  let editContext = $state<any | null>(null);
  let confirmDelete = $state<any | null>(null);
  let loaded = $state(false);

  const from = $derived(addDays(todayStr(), -rangeDays + 1));
  const to = $derived(todayStr());

  $effect(() => {
    dataVersion.n; rangeDays;
    load();
  });

  async function load() {
    try {
      const [e, c, p, ctx, pl] = await Promise.all([
        api("events.list", { from, to }),
        api("checkins.list", { from, to }),
        api("precepts.list", { from, to }),
        api("context.list", { from, to }),
        api("plans.list", { from, to }),
      ]);
      events = e; checkins = c; precepts = p; contexts = ctx; plans = pl;
      loaded = true;
    } catch (err) { reportError(err); }
  }

  interface DayGroup {
    date: string;
    events: any[];
    checkin: any | null;
    precept: any | null;
    contexts: any[];
    unresolvedPlans: any[];
  }

  const days = $derived.by<DayGroup[]>(() => {
    const out: DayGroup[] = [];
    for (let i = 0; i < rangeDays; i++) {
      const date = addDays(to, -i);
      out.push({
        date,
        events: events.filter((e) => e.local_date === date),
        checkin: checkins.find((c) => c.local_date === date) ?? null,
        precept: precepts.find((p) => p.local_date === date) ?? null,
        contexts: contexts.filter((c) => c.start_date <= date && (!c.end_date || c.end_date >= date)),
        unresolvedPlans: plans.filter((p) => p.local_date === date && p.effective_status === "expired_unresolved"),
      });
    }
    return out;
  });

  async function deleteEvent(ev: any, hard: boolean) {
    try {
      await api("events.delete", { id: ev.id, hard });
      confirmDelete = null;
      bump();
      toast(hard ? "Entry permanently deleted" : "Entry deleted (recoverable in database until purge)", "ok");
    } catch (e) { reportError(e); }
  }
</script>

<div class="page">
  <header class="page-head">
    <div>
      <div class="overline">History</div>
      <h1 class="display">Timeline</h1>
    </div>
    <div class="row">
      {#each [7, 14, 30, 90] as d}
        <button class="btn sm" class:primary={rangeDays === d} onclick={() => (rangeDays = d)}>{d} days</button>
      {/each}
    </div>
  </header>

  {#if loaded && events.length === 0 && checkins.length === 0}
    <EmptyState glyph="≡" title="The record starts when you do"
      body="Entries you log appear here day by day. You can always backfill earlier days — a day without entries stays unknown, not empty.">
      <button class="btn primary" onclick={() => openQuickLog()}>Log something</button>
    </EmptyState>
  {:else}
    <div class="days">
      {#each days as day (day.date)}
        <section class="day" class:today={day.date === to}>
          <div class="day-head">
            <span class="day-date display">{fmtDate(day.date)}</span>
            <div class="row" style="gap:6px;">
              {#each day.contexts as c}
                <span class="pill caution" title={c.note ?? ""}>{c.label}</span>
              {/each}
              {#if day.precept}
                <span class="pill" title={day.precept.entries.map((e: any) => `${PRECEPT_LABELS[e.precept_key]}: ${e.status.replace("_", " ")}`).join("\n")}>reflected</span>
              {/if}
              <button class="btn ghost sm" title="Add an entry for this day"
                onclick={() => openQuickLog({ occurredAt: day.date + "T12:00:00" })}>+</button>
            </div>
          </div>

          {#if day.events.length === 0 && !day.checkin}
            <p class="small unknown-note">no entries — unknown, not “did nothing”</p>
          {/if}

          {#each day.events as ev (ev.id)}
            <div class="entry cat-{ev.template_color}">
              <span class="e-glyph">{ev.template_glyph}</span>
              <button class="grow entry-main" onclick={() => openQuickLog({ eventId: ev.id })}>
                <span>
                  {ev.template_name}{#if ev.subtype}<span class="faint"> · {ev.subtype}</span>{/if}
                  {#if ev.status === "cancelled"}<span class="pill" style="margin-left:6px;">cancelled</span>{/if}
                </span>
                {#if ev.note}<span class="small faint note-preview">{ev.note}</span>{/if}
              </button>
              {#if ev.source !== "manual"}<span class="pill">{ev.source}</span>{/if}
              {#if ev.plan_id}<span class="pill accent">planned</span>{/if}
              {#if ev.perceived_quality}<span class="pill">q {ev.perceived_quality}</span>{/if}
              <span class="mono small dim">{ev.time_known ? fmtTime(ev.occurred_at) : "—:—"}</span>
              <span class="mono small" style="min-width:56px; text-align:right;">{fmtDuration(ev.duration_seconds)}</span>
              <button class="btn ghost sm" title="Delete" onclick={() => (confirmDelete = ev)}>✕</button>
            </div>
          {/each}

          {#if day.checkin}
            <div class="entry checkin-entry">
              <span class="e-glyph" style="color:var(--paper-faint);">☰</span>
              <span class="grow small dim">
                Check-in:
                {#each day.checkin.ratings as r, i}{i > 0 ? " · " : " "}{r.label} <b class="mono">{r.value}</b>{/each}
                {#if day.checkin.sleep_duration_minutes != null}
                  · sleep <b class="mono">{fmtDuration(day.checkin.sleep_duration_minutes * 60)}</b>{/if}
              </span>
            </div>
          {/if}

          {#each day.unresolvedPlans as p (p.id)}
            <div class="entry unresolved">
              <span class="e-glyph faint">◌</span>
              <span class="grow small faint">“{p.title}” was planned — what happened is unrecorded</span>
              <button class="btn ghost sm" onclick={() => openQuickLog({ templateId: p.activity_template_id, planId: p.id, occurredAt: day.date + "T12:00:00" })}>it happened</button>
              <button class="btn ghost sm" onclick={() => api("plans.set_status", { id: p.id, status: "skipped" }).then(bump)}>skipped</button>
            </div>
          {/each}
        </section>
      {/each}
    </div>
  {/if}
</div>

{#if editContext}
  <ContextModal existing={editContext} onclose={() => (editContext = null)} />
{/if}

{#if confirmDelete}
  <div class="scrim-mini" role="presentation" onclick={() => (confirmDelete = null)}>
    <div class="card pad confirm" role="dialog" onclick={(e) => e.stopPropagation()}>
      <p>Delete this {confirmDelete.template_name} entry?</p>
      <p class="small faint">Soft delete keeps it recoverable in the database and audit trail; permanent delete removes it entirely.</p>
      <div class="row" style="justify-content:flex-end; margin-top:14px;">
        <button class="btn" onclick={() => (confirmDelete = null)}>Keep</button>
        <button class="btn" onclick={() => deleteEvent(confirmDelete, false)}>Delete</button>
        <button class="btn danger" onclick={() => deleteEvent(confirmDelete, true)}>Delete permanently</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .page { padding: 28px 32px 48px; max-width: 900px; margin: 0 auto; }
  .page-head { display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 22px; flex-wrap: wrap; gap: 12px; }
  h1 { font-size: 27px; }
  .days { display: flex; flex-direction: column; gap: 4px; }
  .day { padding: 10px 0 14px; border-bottom: 1px solid var(--line-soft); }
  .day.today .day-date { color: var(--cinnabar); }
  .day-head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px; }
  .day-date { font-size: 15.5px; color: var(--paper-dim); }
  .unknown-note { color: var(--paper-ghost); font-style: italic; padding: 2px 28px; }
  .entry {
    display: flex; align-items: center; gap: 10px;
    padding: 6px 8px; border-radius: var(--r-sm);
  }
  .entry:hover { background: var(--ink-2); }
  .entry-main { display: flex; flex-direction: column; align-items: flex-start; gap: 1px; text-align: left; font-size: 14px; color: var(--paper); }
  .note-preview { max-width: 420px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .e-glyph { color: var(--cat, var(--paper-faint)); width: 18px; text-align: center; flex: none; }
  .unresolved { border: 1px dashed var(--line); }
  .scrim-mini { position: fixed; inset: 0; z-index: 150; background: rgba(10,9,8,.6); display: flex; align-items: center; justify-content: center; }
  .confirm { max-width: 440px; }
</style>
