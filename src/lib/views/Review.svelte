<script lang="ts">
  import { api } from "../api";
  import { dataVersion, reportError, toast, bump } from "../state.svelte";
  import { fmtDate, fmtDuration, todayStr, addDays, weekStart, validityPill, PRECEPT_LABELS, PRECEPT_KEYS, WEEKDAY_LABELS } from "../format";
  import Sparkline from "../components/Sparkline.svelte";

  let tab = $state<"weekly" | "monthly">("weekly");
  let weekAnchor = $state(todayStr());
  let weekly = $state<any | null>(null);
  let monthly = $state<any | null>(null);
  let monthlyDays = $state(56);
  let reflectionDraft = $state("");
  let reflectionDirty = $state(false);

  $effect(() => {
    dataVersion.n; weekAnchor;
    loadWeekly();
  });
  $effect(() => {
    dataVersion.n; monthlyDays;
    if (tab === "monthly") loadMonthly();
  });

  async function loadWeekly() {
    try {
      weekly = await api("view.weekly", { date: weekAnchor });
      if (!reflectionDirty) reflectionDraft = weekly.reflection?.note ?? "";
    } catch (e) { reportError(e); }
  }

  async function loadMonthly() {
    try {
      monthly = await api("view.monthly", { from: addDays(todayStr(), -monthlyDays + 1), to: todayStr() });
    } catch (e) { reportError(e); }
  }

  async function saveReflection() {
    try {
      await api("review.save_reflection", { week_start: weekly.week_start, note: reflectionDraft });
      reflectionDirty = false;
      toast("Reflection saved", "ok");
      bump();
    } catch (e) { reportError(e); }
  }

  const planSummary = $derived.by(() => {
    if (!weekly) return null;
    const plans = weekly.plans as any[];
    const count = (s: string[]) => plans.filter((p) => s.includes(p.effective_status)).length;
    return {
      total: plans.length,
      completed: count(["completed_linked", "completed_unlinked"]),
      skipped: count(["skipped", "cancelled"]),
      unresolved: count(["expired_unresolved"]),
      open: count(["upcoming", "due"]),
    };
  });

  function preceptSummary() {
    const days = (weekly?.precepts ?? []) as any[];
    const byKey: Record<string, { observed: number; reviewed: number }> = {};
    for (const k of PRECEPT_KEYS) byKey[k] = { observed: 0, reviewed: 0 };
    for (const rec of days) {
      for (const e of rec.entries ?? []) {
        if (e.status !== "not_reviewed") byKey[e.precept_key].reviewed++;
        if (e.status === "observed") byKey[e.precept_key].observed++;
      }
    }
    return byKey;
  }

  function coverageColor(state: string): string {
    switch (state) {
      case "recorded": return "var(--ok)";
      case "future": return "transparent";
      default: return "var(--unknown-wash)";
    }
  }
</script>

<div class="page">
  <header class="page-head">
    <div>
      <div class="overline">Looking back</div>
      <h1 class="display">Review</h1>
    </div>
    <div class="row">
      <button class="btn sm" class:primary={tab === "weekly"} onclick={() => (tab = "weekly")}>Weekly</button>
      <button class="btn sm" class:primary={tab === "monthly"} onclick={() => { tab = "monthly"; loadMonthly(); }}>Monthly</button>
    </div>
  </header>

  {#if tab === "weekly" && weekly}
    <div class="row between" style="margin-bottom:16px;">
      <div class="row">
        <button class="btn sm" onclick={() => { weekAnchor = addDays(weekly.week_start, -7); reflectionDirty = false; }}>‹</button>
        <span class="display" style="font-size:17px;">{fmtDate(weekly.week_start)} – {fmtDate(weekly.week_end)}</span>
        <button class="btn sm" onclick={() => { weekAnchor = addDays(weekly.week_start, 7); reflectionDirty = false; }}
          disabled={weekly.week_end >= todayStr()}>›</button>
        <button class="btn ghost sm" onclick={() => { weekAnchor = todayStr(); reflectionDirty = false; }}>this week</button>
      </div>
    </div>

    <div class="grid">
      <!-- practice volume -->
      <section class="card pad">
        <div class="overline" style="margin-bottom:10px;">Practice</div>
        {#if weekly.volume.length === 0}
          <p class="small faint">No completed activity recorded this week. Days without entries stay unknown.</p>
        {:else}
          <div class="col" style="gap:8px;">
            {#each weekly.volume as v}
              <div class="vol-row cat-{v.color}">
                <span class="e-glyph">{v.glyph}</span>
                <span class="grow">{v.name}</span>
                <span class="mono small dim">{v.session_count}×</span>
                <span class="mono small" style="min-width:70px; text-align:right;">
                  {v.total_seconds != null ? fmtDuration(v.total_seconds) : "—"}
                  {#if v.unknown_duration_count > 0}<span class="faint" title="{v.unknown_duration_count} entries have unknown duration">+?</span>{/if}
                </span>
              </div>
            {/each}
          </div>
        {/if}
      </section>

      <!-- plan vs actual -->
      <section class="card pad">
        <div class="overline" style="margin-bottom:10px;">Planned vs actual</div>
        {#if planSummary && planSummary.total > 0}
          <div class="row wrap" style="gap:8px;">
            <span class="pill ok">{planSummary.completed} completed</span>
            <span class="pill">{planSummary.skipped} skipped / cancelled</span>
            <span class="pill" title="No record either way — unknown, not failure">{planSummary.unresolved} unresolved</span>
            {#if planSummary.open > 0}<span class="pill info">{planSummary.open} still open</span>{/if}
          </div>
          <p class="small faint" style="margin-top:10px;">
            A skipped or unresolved plan is information about the plan, not a verdict on you.
          </p>
        {:else}
          <p class="small faint">Nothing was planned this week.</p>
        {/if}
      </section>

      <!-- data coverage -->
      <section class="card pad">
        <div class="overline" style="margin-bottom:10px;">Data coverage</div>
        <div class="coverage">
          <div class="cov-row header">
            <span></span>
            {#each weekly.coverage as c}<span class="cov-day small faint">{WEEKDAY_LABELS[new Date(c.date + "T12:00:00").getDay() === 0 ? 6 : new Date(c.date + "T12:00:00").getDay() - 1].slice(0, 2)}</span>{/each}
          </div>
          {#each [["checkin", "Check-in"], ["activity", "Activity"], ["precepts", "Precepts"]] as [key, label]}
            <div class="cov-row">
              <span class="small dim">{label}</span>
              {#each weekly.coverage as c}
                <span class="cov-cell" style:background={coverageColor(c[key])}
                  title="{c.date}: {c[key] === 'unknown' ? 'not recorded (unknown)' : c[key]}"></span>
              {/each}
            </div>
          {/each}
        </div>
        <p class="small faint" style="margin-top:10px;">Grey means unrecorded — unknown, never “didn't happen”.</p>
      </section>

      <!-- assessments -->
      <section class="card pad">
        <div class="overline" style="margin-bottom:10px;">Assessments</div>
        {#if weekly.sessions.length === 0}
          <p class="small faint">No sessions this week.</p>
        {:else}
          <div class="col" style="gap:6px;">
            {#each weekly.sessions as s}
              {@const v = validityPill(s.validity_state)}
              <div class="row" style="gap:8px;">
                <span class="small">{s.kind === "pvt_v1" ? "PVT" : s.kind === "go_no_go_v1" ? "GNG" : "Physical"}</span>
                <span class="small faint">{fmtDate(s.local_date)}</span>
                <span class="grow"></span>
                {#if s.is_familiarization === 1}<span class="pill">fam.</span>{/if}
                <span class="pill {v.cls}">{v.label}</span>
              </div>
            {/each}
          </div>
        {/if}
      </section>

      <!-- context & precepts -->
      <section class="card pad">
        <div class="overline" style="margin-bottom:10px;">Context</div>
        {#if weekly.context.length === 0}
          <p class="small faint">No context events recorded.</p>
        {:else}
          <div class="row wrap">
            {#each weekly.context as c}
              <span class="pill caution" title={c.note ?? ""}>{c.label} · {fmtDate(c.start_date)}{c.end_date && c.end_date !== c.start_date ? `–${fmtDate(c.end_date)}` : ""}</span>
            {/each}
          </div>
        {/if}
      </section>

      <section class="card pad quiet-card">
        <div class="overline" style="margin-bottom:10px;">Five precepts · private</div>
        {#if weekly.precepts.length === 0}
          <p class="small faint">No reflections this week — that is simply unrecorded.</p>
        {:else}
          <div class="col" style="gap:5px;">
            {#each Object.entries(preceptSummary()) as [key, s]}
              <div class="row">
                <span class="small dim grow">{PRECEPT_LABELS[key]}</span>
                <span class="small faint">{s.reviewed} reviewed{s.reviewed > 0 ? `, ${s.observed} observed` : ""}</span>
              </div>
            {/each}
          </div>
          <p class="small faint" style="margin-top:8px;">Frequencies only — never a score.</p>
        {/if}
      </section>
    </div>

    <!-- reflection -->
    <section class="card pad" style="margin-top:14px;">
      <div class="overline" style="margin-bottom:8px;">Weekly reflection</div>
      <textarea bind:value={reflectionDraft} oninput={() => (reflectionDirty = true)}
        placeholder="A few lines before the week fades — what stood out, what changed, what context matters."></textarea>
      <div class="row" style="justify-content:flex-end; margin-top:8px;">
        <button class="btn primary sm" onclick={saveReflection} disabled={!reflectionDirty}>Save reflection</button>
      </div>
    </section>
  {/if}

  {#if tab === "monthly" && monthly}
    <div class="row" style="margin-bottom:16px;">
      {#each [[28, "4 weeks"], [56, "8 weeks"], [84, "12 weeks"]] as [d, label]}
        <button class="btn sm" class:primary={monthlyDays === d} onclick={() => (monthlyDays = d as number)}>{label}</button>
      {/each}
    </div>

    <section class="card pad" style="margin-bottom:14px;">
      <div class="overline" style="margin-bottom:10px;">Weekly practice minutes</div>
      <div class="weeks">
        {#each monthly.weeks as w}
          {@const total = (w.volume as any[]).reduce((a, v) => a + (v.total_seconds ?? 0), 0)}
          <div class="week-col" title="{fmtDate(w.week_start)}: {fmtDuration(total)} recorded, {w.checkin_days}/7 check-in days">
            <div class="week-bar-wrap">
              {#each w.volume as v}
                {#if v.total_seconds}
                  <div class="week-seg" style:height="{Math.max(2, (v.total_seconds / 3600) * 18)}px"
                    style:background={`var(--cat-${v.color})`} title="{v.name}: {fmtDuration(v.total_seconds)}"></div>
                {/if}
              {/each}
            </div>
            <span class="small faint mono">{w.week_start.slice(5)}</span>
          </div>
        {/each}
      </div>
      <p class="small faint" style="margin-top:8px;">Recorded practice only — unlogged time is unknown, so bars are lower bounds.</p>
    </section>

    <div class="grid">
      {#each monthly.dimension_series as ds}
        {#if ds.points.length > 0}
          <section class="card pad">
            <div class="row between" style="margin-bottom:6px;">
              <span class="overline">{ds.dimension.label}</span>
              <span class="small faint">{ds.points.length} entries</span>
            </div>
            <Sparkline points={ds.points.map((p: any) => ({ x: p.date, y: p.value }))}
              height={44} yMin={1} yMax={5} color="var(--cat-indigo)" />
            <div class="row between small faint"><span>{ds.dimension.anchor_low} (1)</span><span>(5) {ds.dimension.anchor_high}</span></div>
          </section>
        {/if}
      {/each}

      {#each monthly.assessment_series as as_}
        {#if as_.points.length > 0}
          <section class="card pad">
            <div class="row between" style="margin-bottom:6px;">
              <span class="overline">{as_.label}</span>
              <span class="small faint">{as_.points.length} sessions</span>
            </div>
            <Sparkline points={as_.points.map((p: any) => ({
              x: p.date, y: p.value,
              flag: p.is_familiarization === 1 ? "familiarization" : p.validity_state === "caution" ? "caution" : p.validity_state === "invalid" ? "invalid" : undefined,
            }))} height={44} color="var(--cat-teal)" />
            <p class="small faint">Grey dots are familiarization; amber/red are cautionary or invalid sessions.</p>
          </section>
        {/if}
      {/each}
    </div>

    {#if monthly.context.length > 0}
      <section class="card pad" style="margin-top:14px;">
        <div class="overline" style="margin-bottom:8px;">Context in this period</div>
        <div class="row wrap">
          {#each monthly.context as c}
            <span class="pill caution">{c.label} · {fmtDate(c.start_date)}</span>
          {/each}
        </div>
      </section>
    {/if}

    <p class="small faint" style="margin-top:16px;">
      These are descriptions of your record. To examine a relationship between an exposure and an outcome, use Research →.
    </p>
  {/if}
</div>

<style>
  .page { padding: 28px 32px 48px; max-width: 1100px; margin: 0 auto; }
  .page-head { display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 18px; flex-wrap: wrap; gap: 12px; }
  h1 { font-size: 27px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 12px; align-items: start; }
  .vol-row { display: flex; align-items: center; gap: 10px; font-size: 14px; }
  .e-glyph { color: var(--cat, var(--paper-faint)); width: 16px; text-align: center; }
  .coverage { display: flex; flex-direction: column; gap: 4px; }
  .cov-row { display: grid; grid-template-columns: 70px repeat(7, 1fr); gap: 4px; align-items: center; }
  .cov-day { text-align: center; }
  .cov-cell { height: 18px; border-radius: 4px; background: var(--unknown-wash); }
  .quiet-card { background: var(--ink-1); box-shadow: none; }
  .weeks { display: flex; gap: 14px; align-items: flex-end; overflow-x: auto; padding-bottom: 4px; }
  .week-col { display: flex; flex-direction: column; align-items: center; gap: 6px; }
  .week-bar-wrap { display: flex; flex-direction: column-reverse; gap: 1px; width: 26px; min-height: 4px; }
  .week-seg { width: 100%; border-radius: 2px; }
</style>
