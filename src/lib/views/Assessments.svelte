<script lang="ts">
  import { api } from "../api";
  import { dataVersion, reportError, bump, toast } from "../state.svelte";
  import { fmtDate, fmtInstant, validityPill, WEEKDAY_LABELS } from "../format";
  import PvtRunner from "./PvtRunner.svelte";
  import GngRunner from "./GngRunner.svelte";
  import PhysicalForm from "./PhysicalForm.svelte";
  import Modal from "../components/Modal.svelte";
  import Sparkline from "../components/Sparkline.svelte";
  import EmptyState from "../components/EmptyState.svelte";

  const KINDS = [
    {
      kind: "pvt_v1", name: "Psychomotor Vigilance", short: "PVT",
      desc: "Five minutes of sustained attention: react to a counter as fast as you can.",
      metric: "median_rt_ms", metricLabel: "median RT (ms)",
    },
    {
      kind: "go_no_go_v1", name: "Go / No-Go", short: "GNG",
      desc: "160 rapid decisions: respond to circles, withhold for squares.",
      metric: "commission_error_rate", metricLabel: "commission rate",
    },
    {
      kind: "physical_weekly_v1", name: "Weekly physical check", short: "Physical",
      desc: "Single-leg stance and five-times sit-to-stand, entered manually.",
      metric: null, metricLabel: "",
    },
  ];

  let sessions = $state<any[]>([]);
  let schedules = $state<any[]>([]);
  let due = $state<any[]>([]);
  let runner = $state<"pvt" | "gng" | "physical" | null>(null);
  let detail = $state<any | null>(null);
  let showTrials = $state(false);
  let editSchedule = $state<any | null>(null);

  $effect(() => {
    dataVersion.n;
    load();
  });

  async function load() {
    try {
      const [s, sch, d] = await Promise.all([
        api("assessments.list", { limit: 200 }),
        api("assessments.schedules"),
        api("assessments.due_today"),
      ]);
      sessions = s; schedules = sch; due = d;
    } catch (e) { reportError(e); }
  }

  function sessionsFor(kind: string) {
    return sessions.filter((s) => s.kind === kind);
  }

  function trendPoints(k: (typeof KINDS)[0]) {
    if (!k.metric) return [];
    return sessionsFor(k.kind)
      .filter((s) => s.status === "completed" && s.derived_metrics?.[k.metric!] != null)
      .slice(0, 30)
      .reverse()
      .map((s) => ({
        x: s.local_date,
        y: s.derived_metrics[k.metric!],
        flag: s.is_familiarization ? "familiarization" :
          s.validity_state === "caution" ? "caution" :
          s.validity_state === "invalid" ? "invalid" : undefined,
      }));
  }

  async function openDetail(s: any) {
    try {
      detail = await api("assessments.get", { id: s.id });
      showTrials = false;
    } catch (e) { reportError(e); }
  }

  async function toggleFamiliarization() {
    try {
      detail = { ...(await api("assessments.update", {
        session_id: detail.id,
        is_familiarization: detail.is_familiarization !== 1,
      })), trials: detail.trials };
      bump();
    } catch (e) { reportError(e); }
  }

  async function saveInterruptionNote(noteText: string) {
    try {
      const updated = await api("assessments.update", {
        session_id: detail.id,
        self_reported_interruption: noteText,
      });
      detail = { ...updated, trials: detail.trials };
      bump();
      toast("Noted", "ok");
    } catch (e) { reportError(e); }
  }

  async function saveSchedule() {
    try {
      await api("assessments.save_schedule", {
        kind: editSchedule.kind,
        enabled: editSchedule.enabled === 1 || editSchedule.enabled === true,
        weekdays: editSchedule.weekdays,
        window_start: editSchedule.window_start,
        window_end: editSchedule.window_end,
      });
      editSchedule = null;
      bump();
    } catch (e) { reportError(e); }
  }

  function toggleScheduleDay(i: number) {
    const w = editSchedule.weekdays as number[];
    editSchedule.weekdays = w.includes(i) ? w.filter((x) => x !== i) : [...w, i].sort();
  }

  let interruptionDraft = $state("");
</script>

{#if runner === "pvt"}
  <PvtRunner onclose={() => (runner = null)} />
{:else if runner === "gng"}
  <GngRunner onclose={() => (runner = null)} />
{:else}
  <div class="page">
    <header class="page-head">
      <div>
        <div class="overline">Repeatable measures</div>
        <h1 class="display">Assessments</h1>
      </div>
    </header>
    <p class="small faint intro">
      Personal reference points, not diagnoses. Protocols are versioned, raw trials are kept,
      and sessions can be valid, cautionary, or invalid — all remain inspectable.
    </p>

    <div class="kind-grid">
      {#each KINDS as k}
        {@const sched = schedules.find((s) => s.kind === k.kind)}
        {@const isDue = due.some((d) => d.kind === k.kind)}
        {@const history = sessionsFor(k.kind)}
        <section class="card pad kind-card">
          <div class="row between">
            <h2 class="display" style="font-size:18px;">{k.name}</h2>
            {#if isDue}<span class="pill accent">due today</span>{/if}
          </div>
          <p class="small faint" style="min-height: 36px;">{k.desc}</p>

          {#if k.metric && trendPoints(k).length >= 2}
            <div class="trend">
              <Sparkline points={trendPoints(k)} height={40} color="var(--cat-teal)" />
              <span class="small faint">{k.metricLabel} · your own range, no norms</span>
            </div>
          {:else if history.length > 0}
            <p class="small faint">{history.length} session{history.length === 1 ? "" : "s"} recorded.</p>
          {:else}
            <p class="small faint">Not taken yet. The first runs are familiarization — expect them to be unrepresentative.</p>
          {/if}

          <div class="row between" style="margin-top:auto;">
            <button class="btn ghost sm" onclick={() => (editSchedule = { ...sched })}>
              {sched?.enabled ? `scheduled ${((sched.weekdays as number[]) ?? []).map((w: number) => WEEKDAY_LABELS[w]).join(" ")} ${sched.window_start}–${sched.window_end}` : "no schedule"}
            </button>
            <button class="btn primary sm"
              onclick={() => (runner = k.kind === "pvt_v1" ? "pvt" : k.kind === "go_no_go_v1" ? "gng" : "physical")}>
              Start
            </button>
          </div>
        </section>
      {/each}
    </div>

    <section style="margin-top:28px;">
      <div class="overline" style="margin-bottom:10px;">Session history</div>
      {#if sessions.length === 0}
        <EmptyState glyph="◔" title="No sessions yet"
          body="Assessments are opt-in. Enable a schedule on a card above, or just start one when conditions are right." />
      {:else}
        <div class="sessions card">
          {#each sessions as s (s.id)}
            {@const v = validityPill(s.validity_state)}
            <button class="session-row" onclick={() => openDetail(s)}>
              <span class="s-kind">{KINDS.find((k) => k.kind === s.kind)?.short ?? s.kind}</span>
              <span class="mono small dim">{fmtInstant(s.started_at)}</span>
              <span class="grow"></span>
              {#if s.is_familiarization === 1}<span class="pill">familiarization</span>{/if}
              {#if s.status !== "completed"}<span class="pill">{s.status}</span>{/if}
              <span class="pill {v.cls}">{v.label}</span>
              {#if s.kind === "pvt_v1" && s.derived_metrics}
                <span class="mono small">{s.derived_metrics.median_rt_ms ?? "—"} ms · {s.derived_metrics.lapse_count} lapses</span>
              {:else if s.kind === "go_no_go_v1" && s.derived_metrics}
                <span class="mono small">{s.derived_metrics.go_rt_median_ms ?? "—"} ms · {s.derived_metrics.commission_error_count} comm.</span>
              {:else}
                <span class="mono small faint">{s.trial_count} records</span>
              {/if}
            </button>
          {/each}
        </div>
      {/if}
    </section>
  </div>
{/if}

{#if runner === "physical"}
  <PhysicalForm onclose={() => (runner = null)} />
{/if}

{#if editSchedule}
  <Modal title="Assessment schedule" subtitle="A schedule makes it appear on Today and enables a window reminder."
    onclose={() => (editSchedule = null)} width="440px">
    <div class="col" style="gap:12px;">
      <label class="row" style="gap:8px; cursor:pointer;">
        <input type="checkbox" checked={editSchedule.enabled === 1 || editSchedule.enabled === true}
          onchange={(e) => (editSchedule.enabled = (e.target as HTMLInputElement).checked)} style="width:auto;" />
        <span>Enabled</span>
      </label>
      <div class="row" style="gap:4px;">
        {#each WEEKDAY_LABELS as wl, i}
          <button class="wd" class:active={(editSchedule.weekdays as number[]).includes(i)}
            onclick={() => toggleScheduleDay(i)}>{wl}</button>
        {/each}
      </div>
      <div class="row">
        <label class="field"><span>Window start</span><input type="time" bind:value={editSchedule.window_start} /></label>
        <label class="field"><span>Window end</span><input type="time" bind:value={editSchedule.window_end} /></label>
      </div>
      <p class="small faint">Outside the window an assessment stays available; the deviation is recorded with the session instead of blocking you.</p>
    </div>
    {#snippet footer()}
      <button class="btn" onclick={() => (editSchedule = null)}>Cancel</button>
      <button class="btn primary" onclick={saveSchedule}>Save</button>
    {/snippet}
  </Modal>
{/if}

{#if detail}
  {@const v = validityPill(detail.validity_state)}
  <Modal title={KINDS.find((k) => k.kind === detail.kind)?.name ?? detail.kind}
    subtitle={`Protocol ${detail.protocol_version} · metrics ${detail.metrics_version ?? "—"} · seed ${detail.session_seed ?? "—"}`}
    onclose={() => (detail = null)} width="720px">
    <div class="col" style="gap:14px;">
      <div class="row wrap">
        <span class="pill {v.cls}">{v.label}</span>
        <span class="pill">{detail.status}</span>
        {#if detail.is_familiarization === 1}<span class="pill">familiarization</span>{/if}
        <span class="pill">{fmtInstant(detail.started_at)}</span>
        <span class="pill">tz {detail.timezone}</span>
        {#if detail.input_method}<span class="pill">{detail.input_method.replaceAll("_", " ")}</span>{/if}
      </div>

      {#if (detail.validity_reasons ?? []).length > 0}
        <div class="inset" style="padding: 10px 14px;">
          <div class="overline" style="margin-bottom:4px;">Validity notes</div>
          <ul class="small dim" style="padding-left:16px;">
            {#each detail.validity_reasons as r}<li>{r.replaceAll("_", " ")}</li>{/each}
          </ul>
        </div>
      {/if}

      {#if detail.derived_metrics}
        <div class="metrics-grid mono small">
          {#each Object.entries(detail.derived_metrics) as [key, val]}
            {#if key !== "metrics_version" && typeof val !== "object"}
              <div class="metric-cell">
                <span class="faint">{key.replaceAll("_", " ")}</span>
                <span>{typeof val === "number" ? +Number(val).toFixed(3) : val}</span>
              </div>
            {/if}
          {/each}
        </div>
      {/if}

      <div class="row wrap">
        <button class="btn sm" onclick={toggleFamiliarization}>
          {detail.is_familiarization === 1 ? "Unmark familiarization" : "Mark as familiarization"}
        </button>
        <button class="btn sm" onclick={() => (showTrials = !showTrials)}>
          {showTrials ? "Hide" : "Show"} raw trials ({detail.trials?.length ?? 0})
        </button>
      </div>

      {#if !detail.self_reported_interruption}
        <div class="row">
          <input class="grow" bind:value={interruptionDraft} placeholder="Anything disturb this session? (kept with the record)" />
          <button class="btn sm" disabled={!interruptionDraft.trim()}
            onclick={() => saveInterruptionNote(interruptionDraft)}>Add</button>
        </div>
      {:else}
        <p class="small dim">Interruption note: {detail.self_reported_interruption}</p>
      {/if}

      {#if showTrials}
        <div class="trials inset mono small">
          <table>
            <thead>
              <tr><th>#</th><th>kind</th><th>onset ms</th><th>RT ms</th><th>flags</th></tr>
            </thead>
            <tbody>
              {#each detail.trials as t}
                <tr>
                  <td>{t.trial_index}</td>
                  <td>{t.stimulus_kind}</td>
                  <td>{t.onset_ms ?? "—"}</td>
                  <td>{t.reaction_time_ms ?? "—"}</td>
                  <td class="faint">
                    {[t.is_false_start && "false-start", t.is_lapse && "lapse", t.is_omission && "omission",
                      t.is_commission_error && "commission", t.visibility_lost && "vis-lost"]
                      .filter(Boolean).join(" ") || (t.payload && Object.keys(t.payload).length ? JSON.stringify(t.payload) : "")}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </div>
  </Modal>
{/if}

<style>
  .page { padding: 28px 32px 48px; max-width: 1100px; margin: 0 auto; }
  .page-head { margin-bottom: 8px; }
  h1 { font-size: 27px; }
  .intro { max-width: 620px; margin-bottom: 20px; }
  .kind-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 12px; }
  .kind-card { display: flex; flex-direction: column; gap: 10px; min-height: 190px; }
  .trend { display: flex; flex-direction: column; gap: 4px; }
  .sessions { overflow: hidden; }
  .session-row {
    display: flex; align-items: center; gap: 12px;
    width: 100%; padding: 10px 16px; text-align: left;
    border-bottom: 1px solid var(--line-soft);
    color: var(--paper); font-size: 13.5px;
  }
  .session-row:last-child { border-bottom: none; }
  .session-row:hover { background: var(--ink-3); }
  .s-kind { font-weight: 600; width: 66px; }
  .metrics-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(160px, 1fr)); gap: 8px; }
  .metric-cell { display: flex; flex-direction: column; background: var(--ink-1); padding: 8px 10px; border-radius: var(--r-sm); }
  .trials { max-height: 300px; overflow: auto; padding: 8px; }
  table { border-collapse: collapse; width: 100%; }
  th, td { text-align: left; padding: 3px 10px 3px 0; }
  th { color: var(--paper-faint); font-weight: 500; position: sticky; top: 0; background: var(--ink-1); }
  .wd {
    padding: 4px 8px; font-size: 12px; border-radius: var(--r-xs);
    background: var(--ink-1); border: 1px solid var(--line); color: var(--paper-dim);
  }
  .wd.active { background: var(--cinnabar-wash); border-color: var(--cinnabar-dim); color: var(--cinnabar-bright); }
</style>
