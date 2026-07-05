<script lang="ts">
  import { api } from "../api";
  import { dataVersion, openQuickLog, reportError, bump } from "../state.svelte";
  import { todayStr, addDays, weekStart, monthStart, fmtDate, fmtTime, fmtDuration, WEEKDAY_LABELS } from "../format";
  import PlanEditor from "./PlanEditor.svelte";

  type Mode = "month" | "week" | "day";
  let mode = $state<Mode>("week");
  let anchor = $state(todayStr());
  let plans = $state<any[]>([]);
  let events = $state<any[]>([]);
  let contexts = $state<any[]>([]);
  let series = $state<any[]>([]);
  let editorOpen = $state(false);
  let editPlan = $state<any | null>(null);
  let newPlanDate = $state(todayStr());
  let manageSeries = $state(false);

  const range = $derived.by(() => {
    if (mode === "day") return { from: anchor, to: anchor };
    if (mode === "week") { const s = weekStart(anchor); return { from: s, to: addDays(s, 6) }; }
    const ms = monthStart(anchor);
    const gridStart = weekStart(ms);
    return { from: gridStart, to: addDays(gridStart, 41) };
  });

  $effect(() => {
    dataVersion.n; range.from; range.to;
    load();
  });

  async function load() {
    try {
      const [p, e, c, s] = await Promise.all([
        api("plans.list", { from: range.from, to: range.to }),
        api("events.list", { from: range.from, to: range.to }),
        api("context.list", { from: range.from, to: range.to }),
        api("series.list"),
      ]);
      plans = p; events = e; contexts = c; series = s;
    } catch (err) { reportError(err); }
  }

  function shift(dir: number) {
    if (mode === "day") anchor = addDays(anchor, dir);
    else if (mode === "week") anchor = addDays(anchor, dir * 7);
    else {
      const d = new Date(monthStart(anchor) + "T12:00:00");
      d.setMonth(d.getMonth() + dir);
      anchor = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-01`;
    }
  }

  const gridDays = $derived.by(() => {
    const out: string[] = [];
    let d = range.from;
    while (d <= range.to) { out.push(d); d = addDays(d, 1); }
    return out;
  });

  const monthLabel = $derived(new Date(anchor + "T12:00:00")
    .toLocaleDateString(undefined, { month: "long", year: "numeric" }));

  function plansFor(date: string) { return plans.filter((p) => p.local_date === date); }
  function eventsFor(date: string) { return events.filter((e) => e.local_date === date); }
  function contextFor(date: string) {
    return contexts.filter((c) => c.start_date <= date && (!c.end_date || c.end_date >= date));
  }

  function openNew(date: string) {
    newPlanDate = date; editPlan = null; editorOpen = true;
  }

  function statusColor(s: string): string {
    switch (s) {
      case "completed_linked": case "completed_unlinked": return "var(--ok)";
      case "skipped": case "cancelled": return "var(--paper-ghost)";
      case "expired_unresolved": return "var(--unknown)";
      case "due": return "var(--cinnabar)";
      default: return "var(--paper-faint)";
    }
  }

  function hhmmOf(instant: string): string {
    const d = new Date(instant);
    return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  }

  async function reschedule(plan: any, days: number) {
    try {
      await api("plans.save", {
        id: plan.id, title: plan.title, kind: plan.kind,
        activity_template_id: plan.activity_template_id, assessment_kind: plan.assessment_kind,
        local_date: addDays(plan.local_date, days),
        time_of_day: plan.date_only || !plan.scheduled_start ? null : hhmmOf(plan.scheduled_start),
        target_duration_seconds: plan.target_duration_seconds,
        note: plan.note, determination_id: plan.determination_id,
        reminder_offset_minutes: plan.reminder_offset_minutes,
      });
      bump();
    } catch (e) { reportError(e); }
  }
</script>

<div class="page">
  <header class="page-head">
    <div>
      <div class="overline">Planning</div>
      <h1 class="display">{mode === "month" ? monthLabel : mode === "week" ? `Week of ${fmtDate(weekStart(anchor))}` : fmtDate(anchor)}</h1>
    </div>
    <div class="row">
      <div class="row" style="gap:2px;">
        <button class="btn sm" onclick={() => shift(-1)} aria-label="Previous">‹</button>
        <button class="btn sm" onclick={() => (anchor = todayStr())}>Today</button>
        <button class="btn sm" onclick={() => shift(1)} aria-label="Next">›</button>
      </div>
      <div class="row" style="gap:2px;">
        {#each ["day", "week", "month"] as m}
          <button class="btn sm" class:primary={mode === m} onclick={() => (mode = m as Mode)}>{m}</button>
        {/each}
      </div>
      <button class="btn ghost sm" onclick={() => (manageSeries = !manageSeries)}>Series ({series.length})</button>
      <button class="btn primary" onclick={() => openNew(anchor)}>+ Plan</button>
    </div>
  </header>

  {#if manageSeries}
    <section class="card pad" style="margin-bottom:14px;">
      <div class="overline" style="margin-bottom:8px;">Recurring series</div>
      {#if series.length === 0}
        <p class="small faint">No recurring plans yet. Create one with “+ Plan → Repeats”.</p>
      {:else}
        <div class="col" style="gap:6px;">
          {#each series as s (s.id)}
            <div class="row between series-row">
              <span>{s.title} <span class="small faint">· {s.frequency}{s.time_of_day ? ` at ${s.time_of_day}` : ""}{s.until ? ` until ${s.until}` : ""}</span></span>
              <button class="btn ghost sm" onclick={() => api("series.end", { id: s.id }).then(() => bump())}
                title="Stops future occurrences; past ones remain">end series</button>
            </div>
          {/each}
        </div>
      {/if}
    </section>
  {/if}

  {#if mode === "month"}
    <div class="month-grid">
      {#each WEEKDAY_LABELS as wl}<div class="mg-head overline">{wl}</div>{/each}
      {#each gridDays as date (date)}
        {@const inMonth = date.slice(0, 7) === anchor.slice(0, 7)}
        <button class="mg-cell" class:dim-cell={!inMonth} class:today-cell={date === todayStr()}
          onclick={() => { anchor = date; mode = "day"; }}>
          <span class="mg-date mono">{Number(date.slice(8))}</span>
          <div class="mg-marks">
            {#each plansFor(date).slice(0, 4) as p}
              <span class="mg-dot" style:background={statusColor(p.effective_status)} title={p.title}></span>
            {/each}
            {#each eventsFor(date).slice(0, 4) as e}
              <span class="mg-tick" title={e.template_name}>▪</span>
            {/each}
          </div>
          {#if contextFor(date).length > 0}<span class="mg-ctx"></span>{/if}
        </button>
      {/each}
    </div>
    <div class="legend row wrap small faint">
      <span><span class="mg-dot" style="background:var(--paper-faint);"></span> planned</span>
      <span><span class="mg-dot" style="background:var(--ok);"></span> completed (linked)</span>
      <span><span class="mg-dot" style="background:var(--unknown);"></span> unresolved</span>
      <span><span class="mg-tick">▪</span> recorded activity</span>
      <span><span class="mg-ctx" style="position:static; display:inline-block;"></span> context event</span>
    </div>
  {:else}
    <div class="cols" class:single={mode === "day"}>
      {#each gridDays as date (date)}
        <section class="daycol card" class:today-col={date === todayStr()}>
          <header class="dc-head">
            <span class="display">{fmtDate(date)}</span>
            <button class="btn ghost sm" onclick={() => openNew(date)}>+</button>
          </header>
          {#each contextFor(date) as c}
            <span class="pill caution" style="margin: 0 10px 6px;">{c.label}</span>
          {/each}
          <div class="dc-body">
            {#if plansFor(date).length === 0 && eventsFor(date).length === 0}
              <p class="small dc-empty">—</p>
            {/if}
            {#each plansFor(date) as p (p.id)}
              <div class="dc-plan" style:border-left-color={statusColor(p.effective_status)}>
                <button class="dc-plan-main" onclick={() => { editPlan = p; editorOpen = true; }}>
                  <span class="dc-title" class:struck={["skipped","cancelled"].includes(p.effective_status)}>{p.title}</span>
                  <span class="small faint mono">
                    {p.date_only ? "any time" : fmtTime(p.scheduled_start)}
                    {#if p.target_duration_seconds}· {fmtDuration(p.target_duration_seconds)}{/if}
                    {#if p.series_id}· ↻{/if}
                  </span>
                </button>
                <div class="dc-actions row" style="gap:3px;">
                  {#if ["upcoming","due"].includes(p.effective_status)}
                    <button class="btn ghost sm" title="move to previous day" onclick={() => reschedule(p, -1)}>‹</button>
                    <button class="btn ghost sm" title="move to next day" onclick={() => reschedule(p, 1)}>›</button>
                    {#if p.kind !== "assessment"}
                      <button class="btn ghost sm" title="record completion"
                        onclick={() => openQuickLog({ templateId: p.activity_template_id, planId: p.id, occurredAt: date + "T12:00:00", durationSeconds: p.target_duration_seconds })}>✓</button>
                    {/if}
                  {:else if p.effective_status === "completed_linked"}
                    <span class="pill ok">done</span>
                  {:else if p.effective_status === "expired_unresolved"}
                    <span class="pill" title="No record of what happened — unknown, not failure">?</span>
                  {/if}
                </div>
              </div>
            {/each}
            {#each eventsFor(date) as e (e.id)}
              <button class="dc-event cat-{e.template_color}" onclick={() => openQuickLog({ eventId: e.id })}>
                <span class="e-glyph">{e.template_glyph}</span>
                <span class="grow dc-event-name">{e.template_name}</span>
                <span class="mono small dim" style="flex:none;">{fmtDuration(e.duration_seconds)}</span>
              </button>
            {/each}
          </div>
        </section>
      {/each}
    </div>
  {/if}
</div>

{#if editorOpen}
  <PlanEditor existing={editPlan} initialDate={newPlanDate} onclose={() => { editorOpen = false; editPlan = null; }} />
{/if}

<style>
  .page { padding: 28px 32px 48px; max-width: 1280px; margin: 0 auto; }
  .page-head { display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 20px; flex-wrap: wrap; gap: 12px; }
  h1 { font-size: 25px; }

  .month-grid { display: grid; grid-template-columns: repeat(7, 1fr); gap: 5px; }
  .mg-head { text-align: center; padding: 4px 0; }
  .mg-cell {
    position: relative; min-height: 86px; padding: 8px;
    background: var(--ink-2); border: 1px solid var(--line-soft); border-radius: var(--r-md);
    display: flex; flex-direction: column; align-items: flex-start; gap: 6px;
    transition: border-color 100ms ease;
  }
  .mg-cell:hover { border-color: var(--ink-4); }
  .dim-cell { opacity: 0.42; }
  .today-cell { border-color: var(--cinnabar-dim); }
  .mg-date { font-size: 12px; color: var(--paper-dim); }
  .today-cell .mg-date { color: var(--cinnabar-bright); font-weight: 600; }
  .mg-marks { display: flex; flex-wrap: wrap; gap: 4px; align-items: center; }
  .mg-dot { width: 7px; height: 7px; border-radius: 99px; display: inline-block; }
  .mg-tick { color: var(--paper-faint); font-size: 9px; }
  .mg-ctx {
    position: absolute; top: 8px; right: 8px;
    width: 6px; height: 6px; border-radius: 2px; background: var(--caution);
  }
  .legend { gap: 18px; margin-top: 12px; }
  .legend span { display: inline-flex; align-items: center; gap: 6px; }

  .cols { display: grid; grid-template-columns: repeat(7, 1fr); gap: 8px; align-items: start; }
  .cols.single { grid-template-columns: minmax(0, 560px); justify-content: center; }
  .daycol { overflow: hidden; }
  .today-col { border-color: var(--cinnabar-dim); }
  .dc-head { display: flex; justify-content: space-between; align-items: center; padding: 10px 12px 6px; font-size: 13.5px; }
  .dc-body { padding: 2px 8px 10px; display: flex; flex-direction: column; gap: 5px; }
  .dc-empty { color: var(--paper-ghost); text-align: center; padding: 8px 0 4px; }
  .dc-plan {
    border-left: 2px solid var(--paper-faint);
    background: var(--ink-1); border-radius: var(--r-xs);
    padding: 6px 8px; display: flex; flex-direction: column; gap: 4px;
  }
  .dc-plan-main { display: flex; flex-direction: column; align-items: flex-start; gap: 1px; text-align: left; color: var(--paper); }
  .dc-title { font-size: 12.5px; line-height: 1.35; }
  .dc-title.struck { text-decoration: line-through; color: var(--paper-faint); }
  .dc-actions { justify-content: flex-end; }
  .dc-event {
    display: flex; align-items: center; gap: 7px;
    padding: 5px 8px; border-radius: var(--r-xs);
    font-size: 12.5px; color: var(--paper);
  }
  .dc-event:hover { background: var(--ink-3); }
  .dc-event-name {
    text-align: left; min-width: 0;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }
  .e-glyph { color: var(--cat, var(--paper-faint)); flex: none; }
  .series-row { padding: 6px 8px; border-radius: var(--r-sm); }
  .series-row:hover { background: var(--ink-3); }

  @media (max-width: 1180px) {
    .cols:not(.single) { grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); }
  }
</style>
