<script lang="ts">
  import { api } from "../api";
  import { app, dataVersion, openQuickLog, go, toast, reportError, bump } from "../state.svelte";
  import { fmtDateLong, fmtDuration, fmtTime, fmtDate } from "../format";
  import CheckinPanel from "./CheckinPanel.svelte";
  import PreceptsPanel from "./PreceptsPanel.svelte";
  import ContextModal from "./ContextModal.svelte";
  import EmptyState from "../components/EmptyState.svelte";

  let view = $state<any | null>(null);
  let contextOpen = $state(false);
  let preceptsOpen = $state(false);
  let checkinOpen = $state(false);
  let backfillDate = $state<string | null>(null);

  $effect(() => {
    dataVersion.n;
    load();
  });

  async function load() {
    try {
      view = await api("view.today");
    } catch (e) { reportError(e); }
  }

  async function setPlanStatus(plan: any, status: string) {
    try {
      await api("plans.set_status", { id: plan.id, status });
      bump();
    } catch (e) { reportError(e); }
  }

  const latestCheckin = $derived(view?.checkins_today?.[0] ?? null);
  const hasPrecepts = $derived(view != null && view.precepts_today !== null);
  const yesterdayGaps = $derived.by(() => {
    if (!view) return [];
    const y = view.yesterday_status;
    const gaps: string[] = [];
    if (!y.has_checkin) gaps.push("check-in");
    if (!y.has_events) gaps.push("activity");
    return gaps;
  });

  function greeting(): string {
    const h = new Date().getHours();
    if (h < 5) return "Late night";
    if (h < 12) return "Morning";
    if (h < 18) return "Afternoon";
    return "Evening";
  }
</script>

{#if view}
  <div class="page">
    <header class="page-head">
      <div>
        <div class="overline">{greeting()}</div>
        <h1 class="display">{fmtDateLong(view.today)}</h1>
      </div>
      <div class="row">
        {#if view.baseline.state === "in_baseline"}
          <span class="pill info" title="During baseline LIAN only describes — no pattern claims yet.">
            baseline · day {view.baseline.day_number}
          </span>
        {/if}
        <button class="btn" onclick={() => (contextOpen = true)}>+ Context</button>
        <button class="btn primary" onclick={() => openQuickLog()}>Log practice</button>
      </div>
    </header>

    <div class="columns">
      <!-- ============ left: the day ============ -->
      <div class="col main-col">
        {#if yesterdayGaps.length > 0}
          <div class="gentle card pad row between">
            <span class="small dim">
              Yesterday has no {yesterdayGaps.join(" or ")} recorded — it stays <em>unknown</em> unless you add it.
            </span>
            <div class="row">
              <button class="btn sm" onclick={() => { backfillDate = view.yesterday; checkinOpen = true; }}>
                Backfill check-in
              </button>
              <button class="btn sm" onclick={() => go({ name: "timeline" })}>Open timeline</button>
            </div>
          </div>
        {/if}

        {#if view.due_assessments.length > 0}
          <section class="card pad">
            <div class="overline" style="margin-bottom:10px;">Assessments available</div>
            <div class="col">
              {#each view.due_assessments as due}
                <div class="row between assess-row">
                  <div>
                    <span class="a-name">
                      {due.kind === "pvt_v1" ? "Psychomotor Vigilance (PVT)" :
                       due.kind === "go_no_go_v1" ? "Go / No-Go" : "Weekly physical check"}
                    </span>
                    <span class="small faint" style="margin-left:8px;">
                      window {due.window_start}–{due.window_end}
                      {#if !due.inside_window_now}· outside window now (allowed, noted for validity){/if}
                    </span>
                  </div>
                  <button class="btn sm" onclick={() => go({ name: "assessments" })}>Open</button>
                </div>
              {/each}
            </div>
          </section>
        {/if}

        <section class="card pad">
          <div class="row between" style="margin-bottom:10px;">
            <span class="overline">Planned today</span>
            <button class="btn ghost sm" onclick={() => go({ name: "calendar" })}>Calendar →</button>
          </div>
          {#if view.plans.length === 0}
            <p class="small faint">Nothing planned. That is a fine kind of day.</p>
          {:else}
            <div class="col" style="gap:6px;">
              {#each view.plans as plan (plan.id)}
                <div class="plan-row" class:done={plan.effective_status === "completed_linked"}>
                  <span class="dot" style:background={plan.effective_status === "completed_linked" ? "var(--ok)" : plan.effective_status === "skipped" || plan.effective_status === "cancelled" ? "var(--paper-ghost)" : "var(--cinnabar)"}></span>
                  <span class="grow p-title" class:struck={["skipped","cancelled"].includes(plan.effective_status)}>
                    {plan.title}
                    {#if plan.scheduled_start}<span class="small faint mono" style="margin-left:8px;">{fmtTime(plan.scheduled_start)}</span>{/if}
                  </span>
                  {#if plan.effective_status === "completed_linked"}
                    <span class="pill ok">done</span>
                  {:else if ["skipped","cancelled"].includes(plan.effective_status)}
                    <span class="pill">{plan.effective_status}</span>
                    <button class="btn ghost sm" onclick={() => setPlanStatus(plan, "upcoming")}>undo</button>
                  {:else}
                    {#if plan.kind === "assessment"}
                      <button class="btn sm" onclick={() => go({ name: "assessments" })}>start</button>
                    {:else}
                      <button class="btn sm" onclick={() => openQuickLog({ templateId: plan.activity_template_id, planId: plan.id, durationSeconds: plan.target_duration_seconds })}>
                        log it
                      </button>
                    {/if}
                    <button class="btn ghost sm" title="Skipping is information, not failure" onclick={() => setPlanStatus(plan, "skipped")}>skip</button>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </section>

        <section class="card pad">
          <div class="overline" style="margin-bottom:10px;">Recorded today</div>
          {#if view.events_today.length === 0}
            <EmptyState glyph="◦" title="Nothing recorded yet"
              body="Press L or the Log button — a quick entry takes seconds. Unlogged time simply stays unknown." />
          {:else}
            <div class="col" style="gap:6px;">
              {#each view.events_today as ev (ev.id)}
                <button class="event-row cat-{ev.template_color}" onclick={() => openQuickLog({ eventId: ev.id })}>
                  <span class="e-glyph">{ev.template_glyph}</span>
                  <span class="grow" style="text-align:left;">
                    {ev.template_name}{#if ev.subtype}<span class="faint"> · {ev.subtype}</span>{/if}
                  </span>
                  {#if ev.source === "timer"}<span class="pill" title="recorded with the timer">timer</span>{/if}
                  {#if ev.plan_id}<span class="pill accent" title="linked to a plan">planned</span>{/if}
                  <span class="mono small dim">{ev.time_known ? fmtTime(ev.occurred_at) : "time unknown"}</span>
                  <span class="mono small">{fmtDuration(ev.duration_seconds)}</span>
                </button>
              {/each}
            </div>
          {/if}
        </section>

        {#if view.context_today.length > 0}
          <section class="card pad">
            <div class="overline" style="margin-bottom:8px;">Context in effect</div>
            <div class="row wrap">
              {#each view.context_today as c}
                <span class="pill caution">{c.label}{#if !c.end_date}&nbsp;· ongoing{/if}</span>
              {/each}
            </div>
          </section>
        {/if}
      </div>

      <!-- ============ right: reflection ============ -->
      <div class="col side-col">
        <section class="card pad">
          <div class="row between" style="margin-bottom:6px;">
            <span class="overline">Daily check-in</span>
            {#if latestCheckin}<span class="pill ok">recorded</span>{/if}
          </div>
          {#if checkinOpen || !latestCheckin}
            <CheckinPanel
              date={backfillDate ?? view.today}
              existing={backfillDate ? null : latestCheckin}
              onsaved={() => { checkinOpen = false; backfillDate = null; }}
            />
            {#if backfillDate}
              <p class="small faint" style="margin-top:8px;">Backfilling {fmtDate(backfillDate)} —
                <button class="btn ghost sm" onclick={() => { backfillDate = null; }}>switch to today</button></p>
            {/if}
          {:else}
            <div class="row wrap" style="margin: 6px 0;">
              {#each latestCheckin.ratings as r}
                <span class="pill">{r.label} <b class="mono">{r.value}</b></span>
              {/each}
              {#if latestCheckin.sleep_duration_minutes != null}
                <span class="pill">sleep <b class="mono">{fmtDuration(latestCheckin.sleep_duration_minutes * 60)}</b></span>
              {/if}
            </div>
            <button class="btn ghost sm" onclick={() => (checkinOpen = true)}>Edit today's check-in</button>
          {/if}
        </section>

        <section class="card pad quiet">
          <button class="row between disclosure" onclick={() => (preceptsOpen = !preceptsOpen)}
            aria-expanded={preceptsOpen}>
            <span class="overline">Five precepts</span>
            <span class="row" style="gap:8px;">
              {#if hasPrecepts}<span class="pill ok">reflected</span>{/if}
              <span class="faint">{preceptsOpen ? "–" : "+"}</span>
            </span>
          </button>
          {#if preceptsOpen}
            <div style="margin-top:12px;">
              <PreceptsPanel date={view.today} existing={view.precepts_today} onsaved={() => (preceptsOpen = false)} />
            </div>
          {/if}
        </section>

        {#if view.active_determinations.length > 0 || view.determinations_due_review.length > 0}
          <section class="card pad">
            <div class="row between" style="margin-bottom:8px;">
              <span class="overline">Determinations</span>
              <button class="btn ghost sm" onclick={() => go({ name: "determinations" })}>All →</button>
            </div>
            <div class="col" style="gap:6px;">
              {#each view.active_determinations.slice(0, 4) as d (d.id)}
                <div class="det-row">
                  <span class="det-mark" class:paused={d.lifecycle_state === "paused"}>❖</span>
                  <span class="grow small">{d.title}</span>
                  {#if view.determinations_due_review.some((x: any) => x.id === d.id)}
                    <button class="btn sm" onclick={() => go({ name: "determinations" })}>review</button>
                  {:else if d.lifecycle_state === "paused"}
                    <span class="pill">paused</span>
                  {/if}
                </div>
              {/each}
            </div>
          </section>
        {/if}

        {#if view.baseline.state === "not_started"}
          <section class="card pad">
            <p class="small dim">No baseline period is running.
              A 4–6 week baseline gives later comparisons a footing.</p>
            <button class="btn sm" style="margin-top:8px;" onclick={() => go({ name: "settings" })}>
              Start baseline in Settings
            </button>
          </section>
        {/if}
      </div>
    </div>
  </div>
{:else}
  <div class="page"><div class="skeleton card" style="height: 180px;"></div></div>
{/if}

{#if contextOpen}
  <ContextModal onclose={() => (contextOpen = false)} />
{/if}

<style>
  .page { padding: 28px 32px 48px; max-width: 1200px; margin: 0 auto; }
  .page-head {
    display: flex; justify-content: space-between; align-items: flex-end;
    margin-bottom: 22px; gap: 16px; flex-wrap: wrap;
  }
  h1 { font-size: 27px; }
  .columns { display: grid; grid-template-columns: minmax(0, 1.5fr) minmax(300px, 1fr); gap: 16px; align-items: start; }
  @media (max-width: 980px) { .columns { grid-template-columns: 1fr; } }
  .main-col, .side-col { gap: 14px; }

  .gentle { background: var(--ink-1); border-style: dashed; box-shadow: none; }

  .plan-row, .event-row, .det-row, .assess-row {
    display: flex; align-items: center; gap: 10px;
    padding: 8px 10px; border-radius: var(--r-sm);
  }
  .plan-row:hover, .event-row:hover { background: var(--ink-3); }
  .plan-row.done { opacity: 0.75; }
  .p-title.struck { color: var(--paper-faint); text-decoration: line-through; text-decoration-color: var(--paper-ghost); }
  .event-row { width: 100%; border: none; font-size: 14px; color: var(--paper); }
  .e-glyph { color: var(--cat, var(--paper-faint)); width: 18px; text-align: center; }
  .a-name { font-size: 14px; }
  .det-mark { color: var(--cinnabar); font-size: 12px; }
  .det-mark.paused { color: var(--paper-ghost); }
  .disclosure { width: 100%; }
  .quiet { background: var(--ink-1); box-shadow: none; }
  .skeleton { animation: breathe 1.6s ease-in-out infinite; }
  @keyframes breathe { 50% { opacity: 0.5; } }
</style>
