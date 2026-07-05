<script lang="ts">
  import { api } from "../api";
  import { app, dataVersion, activeTemplates, enabledDimensions, toast, reportError, bump } from "../state.svelte";
  import { todayStr, addDays, fmtInstant, fmtDate, EVIDENCE_LABELS, CONTEXT_KINDS } from "../format";
  import Modal from "../components/Modal.svelte";
  import EmptyState from "../components/EmptyState.svelte";

  let tab = $state<"explore" | "results" | "protocols">("explore");

  // ---------- explorer spec ----------
  let expKind = $state<"activity_duration" | "activity_count" | "sleep_duration" | "checkin_dimension">("activity_duration");
  let expTemplate = $state("");
  let expDimension = $state("");
  let outKind = $state<"checkin_dimension" | "assessment_metric" | "sleep_duration">("checkin_dimension");
  let outDimension = $state("");
  let outAssessment = $state("pvt_v1");
  let outMetric = $state("lapse_rate");
  let lagDays = $state(1);
  let rangeDays = $state(56);
  let excludeContext = $state<string[]>(["illness"]);
  let running = $state(false);
  let result = $state<any | null>(null);
  let showPoints = $state(false);

  let results = $state<any[]>([]);
  let protocols = $state<any[]>([]);
  let protocolEditor = $state(false);
  let protoDetail = $state<any | null>(null);
  let promoting = $state<any | null>(null);
  let promoteNote = $state("");

  const METRICS: Record<string, [string, string][]> = {
    pvt_v1: [["lapse_rate", "lapse rate"], ["median_rt_ms", "median RT (ms)"], ["mean_rt_ms", "mean RT (ms)"], ["false_start_count", "false starts"]],
    go_no_go_v1: [["commission_error_rate", "commission rate"], ["omission_rate", "omission rate"], ["go_rt_median_ms", "go RT median (ms)"]],
  };

  $effect(() => {
    dataVersion.n;
    loadLists();
  });

  async function loadLists() {
    try {
      const [r, p] = await Promise.all([api("analysis.results", { limit: 100 }), api("protocols.list")]);
      results = r; protocols = p;
    } catch (e) { reportError(e); }
  }

  function expLabel(): string {
    if (expKind === "sleep_duration") return "Sleep duration (min)";
    if (expKind === "checkin_dimension") return app.dimensions.find((d) => d.id === expDimension)?.label ?? "dimension";
    const t = app.templates.find((t) => t.id === expTemplate);
    return `${t?.name ?? "activity"} ${expKind === "activity_count" ? "(sessions/day)" : "(min/day)"}`;
  }
  function outLabel(): string {
    if (outKind === "sleep_duration") return "Sleep duration (min)";
    if (outKind === "checkin_dimension") return app.dimensions.find((d) => d.id === outDimension)?.label ?? "dimension";
    const m = METRICS[outAssessment]?.find(([k]) => k === outMetric)?.[1] ?? outMetric;
    return `${outAssessment === "pvt_v1" ? "PVT" : "Go/No-Go"} ${m}`;
  }

  async function run(persist: boolean) {
    running = true;
    try {
      result = await api("analysis.run", {
        exposure: {
          kind: expKind,
          template_id: expKind.startsWith("activity") ? expTemplate || null : null,
          dimension_id: expKind === "checkin_dimension" ? expDimension || null : null,
          label: expLabel(),
        },
        outcome: {
          kind: outKind,
          dimension_id: outKind === "checkin_dimension" ? outDimension || null : null,
          assessment_kind: outKind === "assessment_metric" ? outAssessment : null,
          metric: outKind === "assessment_metric" ? outMetric : null,
          label: outLabel(),
        },
        lag_days: lagDays,
        from: addDays(todayStr(), -rangeDays + 1),
        to: todayStr(),
        exclude_context_kinds: excludeContext,
        persist,
      });
      if (persist) { toast("Result saved", "ok"); loadLists(); }
    } catch (e) { reportError(e); }
    running = false;
  }

  async function promote() {
    try {
      await api("analysis.promote", { result_id: promoting.id, note: promoteNote || null });
      promoting = null; promoteNote = "";
      toast("Recorded as a candidate hypothesis", "ok");
      loadLists();
    } catch (e) { reportError(e); }
  }

  // ---------- protocol editor ----------
  let pId = $state<string | null>(null);
  let pTitle = $state(""); let pQuestion = $state(""); let pHypothesis = $state("");
  let pIntervention = $state(""); let pAnalysisPlan = $state("");
  let pStart = $state(todayStr()); let pEnd = $state(addDays(todayStr(), 42));
  let pAdherence = $state(""); let pContextVars = $state(""); let pStop = $state("");
  let pOutcomeSummary = $state("");
  let pSpec = $state<any | null>(null);

  /** Prefill from a candidate-hypothesis result (r), an existing protocol (proto), or blank. */
  function openProtocolFrom(r: any | null, proto: any | null = null) {
    protocolEditor = true;
    pId = null; pSpec = null;
    if (proto) {
      pId = proto.id;
      pTitle = proto.title; pQuestion = proto.question; pHypothesis = proto.hypothesis;
      pIntervention = proto.intervention_definition; pAnalysisPlan = proto.analysis_plan;
      pStart = proto.start_date ?? todayStr(); pEnd = proto.end_date ?? addDays(todayStr(), 42);
      pAdherence = proto.adherence_requirements ?? ""; pContextVars = proto.context_variables ?? "";
      pStop = proto.stop_criteria ?? "";
      pOutcomeSummary = JSON.stringify(proto.primary_outcome_definition);
      pSpec = proto.analysis_spec ?? null;
    } else if (r) {
      const e = r.exposure_definition; const o = r.outcome_definition;
      pTitle = `${e.label ?? "exposure"} → ${o.label ?? "outcome"}`;
      pQuestion = `Does ${e.label ?? "the exposure"} relate to ${o.label ?? "the outcome"} ${r.time_window.lag_days === 0 ? "the same day" : `${r.time_window.lag_days} day(s) later`}?`;
      pHypothesis = "";
      pAnalysisPlan = `Compare ${o.label} across ${e.label} levels with lag ${r.time_window.lag_days}, excluding invalid and familiarization sessions; median-split comparison and rank correlation as in analysis ${r.analysis_version}.`;
      pIntervention = "";
      pOutcomeSummary = JSON.stringify(o);
      // Machine-readable pre-registration: exactly what analysis.run accepts.
      const scope = r.source_data_scope ?? {};
      pSpec = {
        exposure: e, outcome: o, lag_days: r.time_window.lag_days,
        from: r.time_window.from, to: r.time_window.to,
        exclude_invalid: scope.exclude_invalid ?? true,
        exclude_familiarization: scope.exclude_familiarization ?? true,
        exclude_context_kinds: scope.exclude_context_kinds ?? [],
      };
    } else {
      pTitle = ""; pQuestion = ""; pHypothesis = ""; pIntervention = ""; pAnalysisPlan = "";
      pOutcomeSummary = "";
    }
  }

  async function saveProtocol() {
    try {
      const saved = await api("protocols.save", {
        id: pId,
        title: pTitle, question: pQuestion, hypothesis: pHypothesis,
        primary_outcome_definition: pOutcomeSummary ? JSON.parse(pOutcomeSummary) : { description: pQuestion, version: "analysis-1.0" },
        intervention_definition: pIntervention,
        analysis_plan: pAnalysisPlan,
        start_date: pStart, end_date: pEnd,
        adherence_requirements: pAdherence || null,
        context_variables: pContextVars || null,
        stop_criteria: pStop || null,
        analysis_spec: pSpec,
      });
      protocolEditor = false;
      if (pId && saved.id !== pId) {
        toast(`Amendment created protocol version ${saved.version}`, "ok");
      } else {
        toast(pId ? "Protocol updated" : "Protocol saved as draft", "ok");
      }
      protoDetail = null;
      loadLists();
    } catch (e) { reportError(e); }
  }

  async function openProtocol(p: any) {
    try {
      const full = await api("protocols.get", { id: p.id });
      protoDetail = full;
      // Viewing linked results is the moment outcome/plan edits must start
      // forking a new version. A protocol without results stays unlocked.
      if ((full.results ?? []).length > 0 && full.results_locked !== 1
          && ["active", "paused", "completed"].includes(full.status)) {
        await api("protocols.lock_results", { id: full.id });
        full.results_locked = 1;
      }
    } catch (e) { reportError(e); }
  }

  let runningProtoAnalysis = $state(false);

  /** Run the protocol's pre-registered analysis and persist the linked result. */
  async function runProtocolAnalysis(p: any) {
    if (!p.analysis_spec) return;
    runningProtoAnalysis = true;
    try {
      const spec = { ...p.analysis_spec };
      spec.from = p.start_date ?? spec.from;
      const end = p.end_date && p.end_date < todayStr() ? p.end_date : todayStr();
      spec.to = end;
      spec.protocol_id = p.id;
      spec.persist = true;
      await api("analysis.run", spec);
      await api("protocols.lock_results", { id: p.id });
      protoDetail = await api("protocols.get", { id: p.id });
      toast("Linked result recorded", "ok");
      loadLists();
    } catch (e) { reportError(e); }
    runningProtoAnalysis = false;
  }

  async function protocolAction(p: any, action: string, arg?: string) {
    try {
      if (action === "status") await api("protocols.set_status", { id: p.id, status: arg });
      else if (action === "conclude") await api("protocols.conclude", { id: p.id, conclusion: arg, note: null });
      protoDetail = null;
      loadLists();
    } catch (e) { reportError(e); }
  }

  const baselineActive = $derived(app.settings.baseline_start != null &&
    todayStr() <= addDays(app.settings.baseline_start, (app.settings.baseline_weeks ?? 5) * 7));
</script>

<div class="page">
  <header class="page-head">
    <div>
      <div class="overline">Personal research</div>
      <h1 class="display">Research</h1>
    </div>
    <div class="row">
      <button class="btn sm" class:primary={tab === "explore"} onclick={() => (tab = "explore")}>Explore</button>
      <button class="btn sm" class:primary={tab === "results"} onclick={() => (tab = "results")}>Results ({results.length})</button>
      <button class="btn sm" class:primary={tab === "protocols"} onclick={() => (tab = "protocols")}>Protocols ({protocols.length})</button>
    </div>
  </header>

  {#if baselineActive}
    <p class="baseline-note small">
      Baseline period is running — analyses stay descriptive until it ends. Collect first, interpret later.
    </p>
  {/if}

  {#if tab === "explore"}
    <section class="card pad">
      <div class="spec">
        <div class="spec-block">
          <div class="overline">Exposure</div>
          <select bind:value={expKind}>
            <option value="activity_duration">Activity duration</option>
            <option value="activity_count">Activity count</option>
            <option value="sleep_duration">Sleep duration</option>
            <option value="checkin_dimension">Check-in dimension</option>
          </select>
          {#if expKind.startsWith("activity")}
            <select bind:value={expTemplate}>
              <option value="">— choose activity —</option>
              {#each activeTemplates() as t}<option value={t.id}>{t.name}</option>{/each}
            </select>
          {:else if expKind === "checkin_dimension"}
            <select bind:value={expDimension}>
              <option value="">— choose dimension —</option>
              {#each enabledDimensions() as d}<option value={d.id}>{d.label}</option>{/each}
            </select>
          {/if}
        </div>

        <div class="spec-arrow">
          <span class="mono small faint">lag</span>
          <div class="row" style="gap:3px;">
            {#each [0, 1, 2] as l}
              <button class="btn sm" class:primary={lagDays === l} onclick={() => (lagDays = l)}>{l === 0 ? "same day" : `+${l}d`}</button>
            {/each}
            <input type="number" min="0" max="14" value={lagDays}
              onchange={(e) => (lagDays = Number((e.target as HTMLInputElement).value))}
              style="width:54px;" title="custom lag (days)" />
          </div>
          <span class="arrow display">→</span>
        </div>

        <div class="spec-block">
          <div class="overline">Outcome</div>
          <select bind:value={outKind}>
            <option value="checkin_dimension">Check-in dimension</option>
            <option value="assessment_metric">Assessment metric</option>
            <option value="sleep_duration">Sleep duration</option>
          </select>
          {#if outKind === "checkin_dimension"}
            <select bind:value={outDimension}>
              <option value="">— choose dimension —</option>
              {#each enabledDimensions() as d}<option value={d.id}>{d.label}</option>{/each}
            </select>
          {:else if outKind === "assessment_metric"}
            <select bind:value={outAssessment} onchange={() => (outMetric = METRICS[outAssessment][0][0])}>
              <option value="pvt_v1">PVT</option>
              <option value="go_no_go_v1">Go / No-Go</option>
            </select>
            <select bind:value={outMetric}>
              {#each METRICS[outAssessment] as [k, label]}<option value={k}>{label}</option>{/each}
            </select>
          {/if}
        </div>
      </div>

      <div class="row wrap" style="margin-top:14px; gap:14px;">
        <label class="field"><span>Window</span>
          <div class="row" style="gap:3px;">
            {#each [[28, "4w"], [56, "8w"], [84, "12w"], [180, "6m"]] as [d, label]}
              <button class="btn sm" class:primary={rangeDays === d} onclick={() => (rangeDays = d as number)}>{label}</button>
            {/each}
          </div>
        </label>
        <label class="field"><span>Exclude days with context</span>
          <div class="row wrap" style="gap:4px;">
            {#each CONTEXT_KINDS.slice(0, 8) as [k, label]}
              <button class="pill" class:accent={excludeContext.includes(k)} style="cursor:pointer;"
                onclick={() => (excludeContext = excludeContext.includes(k) ? excludeContext.filter((x) => x !== k) : [...excludeContext, k])}>
                {label}
              </button>
            {/each}
          </div>
        </label>
        <div class="grow"></div>
        <button class="btn primary" style="align-self:flex-end;" onclick={() => run(false)} disabled={running}>Inspect</button>
      </div>
    </section>

    {#if result}
      {@const el = EVIDENCE_LABELS[result.evidence_label]}
      <section class="card pad" style="margin-top:14px;">
        <div class="row between wrap" style="margin-bottom:10px;">
          <div>
            <h2 class="display" style="font-size:18px;">{result.exposure_definition.label} → {result.outcome_definition.label}</h2>
            <p class="small faint">lag {result.time_window.lag_days}d · {result.time_window.from} to {result.time_window.to} · analysis {result.analysis_version}</p>
          </div>
          <span class="pill" class:accent={result.evidence_label === "observational_signal"}
            class:info={result.evidence_label === "descriptive"} title={el?.hint}>{el?.label ?? result.evidence_label}</span>
        </div>

        <div class="counts mono small">
          <div><span class="faint">paired days</span> {result.included_count}</div>
          <div><span class="faint">excluded</span> {result.excluded_count}</div>
          <div><span class="faint">unknown / missing</span> {result.missing_count}</div>
          <div><span class="faint">rank correlation ρ</span> {result.values_json.spearman_rho != null ? (+result.values_json.spearman_rho).toFixed(2) : "—"}</div>
          <div><span class="faint">threshold</span> ≥{result.values_json.min_pairs_threshold} pairs</div>
        </div>

        {#if result.values_json.groups}
          {@const g = result.values_json.groups}
          <div class="groups inset">
            <div class="group">
              <span class="overline">exposure ≤ {(+g.split_value).toFixed(0)}</span>
              <span class="mono">{g.low_exposure_outcome ? (+g.low_exposure_outcome.mean).toFixed(2) : "—"}</span>
              <span class="small faint">mean outcome · n={g.low_exposure_outcome?.n ?? 0}</span>
            </div>
            <div class="group">
              <span class="overline">exposure &gt; {(+g.split_value).toFixed(0)}</span>
              <span class="mono">{g.high_exposure_outcome ? (+g.high_exposure_outcome.mean).toFixed(2) : "—"}</span>
              <span class="small faint">mean outcome · n={g.high_exposure_outcome?.n ?? 0}</span>
            </div>
          </div>
        {/if}

        <!-- scatter -->
        {#if result.values_json.points.length > 0}
          {@const pts = result.values_json.points}
          {@const xs = pts.map((p: any) => p.exposure)}
          {@const ys = pts.map((p: any) => p.outcome)}
          {@const xmin = Math.min(...xs)} {@const xmax = Math.max(...xs)}
          {@const ymin = Math.min(...ys)} {@const ymax = Math.max(...ys)}
          <svg viewBox="0 0 100 46" class="scatter" role="img" aria-label="scatter of paired observations">
            {#each pts as p}
              <circle
                cx={4 + ((p.exposure - xmin) / (xmax - xmin || 1)) * 92}
                cy={42 - ((p.outcome - ymin) / (ymax - ymin || 1)) * 38}
                r="1.6" fill="var(--cat-teal)" opacity="0.8">
                <title>{p.exposure_date}: exposure {p.exposure} → outcome {p.outcome} ({p.outcome_date})</title>
              </circle>
            {/each}
          </svg>
          <div class="row between small faint">
            <span>{result.exposure_definition.label} →</span>
            <span>↑ {result.outcome_definition.label}</span>
          </div>
        {/if}

        <div class="caveats">
          {#each result.caveats as c}<p class="small faint">• {c}</p>{/each}
        </div>

        <div class="row wrap" style="margin-top:12px;">
          <button class="btn sm" onclick={() => (showPoints = !showPoints)}>{showPoints ? "Hide" : "Inspect"} raw pairs</button>
          <button class="btn sm" onclick={() => run(true)}>Save result</button>
          {#if result.evidence_label === "observational_signal"}
            <button class="btn sm primary" onclick={() => { run(true).then(() => { promoting = results[0] ?? result; }); }}>
              Mark as candidate hypothesis…
            </button>
          {/if}
        </div>

        {#if showPoints}
          <div class="inset mono small" style="margin-top:10px; max-height:220px; overflow:auto; padding:8px 12px;">
            <table style="width:100%; border-collapse:collapse;">
              <thead><tr><th style="text-align:left;">exposure day</th><th style="text-align:left;">value</th><th style="text-align:left;">outcome day</th><th style="text-align:left;">value</th></tr></thead>
              <tbody>
                {#each result.values_json.points as p}
                  <tr><td>{p.exposure_date}</td><td>{p.exposure}</td><td>{p.outcome_date}</td><td>{p.outcome}</td></tr>
                {/each}
              </tbody>
            </table>
          </div>
        {/if}
      </section>
    {:else}
      <EmptyState glyph="∴" title="Choose an exposure and an outcome"
        body="LIAN shows you counts, raw pairs, and honest labels. It will tell you when there isn't enough data — and it will never claim causation." />
    {/if}
  {/if}

  {#if tab === "results"}
    {#if results.length === 0}
      <EmptyState glyph="∴" title="No saved results"
        body="Run an analysis in Explore and save it. Saved results keep their data scope, counts, caveats, and version — including null findings." />
    {:else}
      <div class="col" style="gap:8px;">
        {#each results as r (r.id)}
          {@const el = EVIDENCE_LABELS[r.evidence_label]}
          <div class="card pad result-row" class:stale={r.is_stale === 1}>
            <div class="row between wrap">
              <div>
                <span>{r.exposure_definition.label ?? r.exposure_definition.kind} → {r.outcome_definition.label ?? r.outcome_definition.kind}</span>
                <p class="small faint">
                  {fmtInstant(r.generated_at)} · lag {r.time_window.lag_days}d · n={r.included_count} · missing {r.missing_count}
                  {#if r.is_stale === 1}· <span style="color:var(--caution);">stale — source data changed since generation</span>{/if}
                </p>
              </div>
              <div class="row">
                <span class="pill" class:accent={["observational_signal", "candidate_hypothesis"].includes(r.evidence_label)} title={el?.hint}>{el?.label}</span>
                {#if r.evidence_label === "observational_signal"}
                  <button class="btn sm" onclick={() => (promoting = r)}>promote…</button>
                {/if}
                {#if r.evidence_label === "candidate_hypothesis"}
                  <button class="btn sm primary" onclick={() => openProtocolFrom(r)}>design protocol</button>
                {/if}
              </div>
            </div>
          </div>
        {/each}
      </div>
    {/if}
  {/if}

  {#if tab === "protocols"}
    <div class="row" style="margin-bottom:12px;">
      <button class="btn primary sm" onclick={() => openProtocolFrom(null)}>+ New protocol</button>
    </div>
    {#if protocols.length === 0}
      <EmptyState glyph="⧉" title="No protocols yet"
        body="A protocol pre-registers a question, hypothesis, outcome, and analysis window before you look at results. Candidate hypotheses from Results can seed one." />
    {:else}
      <div class="col" style="gap:8px;">
        {#each protocols as p (p.id)}
          <button class="card pad result-row" style="text-align:left;" onclick={() => openProtocol(p)}>
            <div class="row between wrap">
              <div>
                <span>{p.title} <span class="small faint mono">v{p.version}</span></span>
                <p class="small faint">{p.question}</p>
              </div>
              <div class="row">
                {#if p.conclusion}<span class="pill" title={EVIDENCE_LABELS[p.conclusion]?.hint}>{EVIDENCE_LABELS[p.conclusion]?.label}</span>{/if}
                <span class="pill" class:ok={p.status === "active"} class:info={p.status === "draft"}>{p.status}</span>
              </div>
            </div>
          </button>
        {/each}
      </div>
    {/if}
  {/if}
</div>

{#if promoting}
  <Modal title="Candidate hypothesis" subtitle="An explicit, human decision — LIAN never promotes automatically."
    onclose={() => (promoting = null)} width="480px">
    <div class="col" style="gap:10px;">
      <p class="small dim">
        Confirm this only if the signal has repeated across periods and you have considered obvious
        confounders (sleep, illness, workload, testing time).
      </p>
      <label class="field"><span>Why do you believe this deserves a protocol?</span>
        <textarea bind:value={promoteNote} placeholder="e.g. seen in June and July windows; holds when illness days are excluded"></textarea>
      </label>
    </div>
    {#snippet footer()}
      <button class="btn" onclick={() => (promoting = null)}>Not yet</button>
      <button class="btn primary" onclick={promote}>Confirm candidate</button>
    {/snippet}
  </Modal>
{/if}

{#if protocolEditor}
  <Modal title="Research protocol" subtitle="Everything below is fixed before results are viewed; later changes create a new version."
    onclose={() => (protocolEditor = false)} width="640px">
    <div class="col" style="gap:11px;">
      <label class="field"><span>Title</span><input bind:value={pTitle} /></label>
      <label class="field"><span>Question</span><input bind:value={pQuestion} placeholder="What are you asking?" /></label>
      <label class="field"><span>Hypothesis (expected relationship)</span>
        <textarea bind:value={pHypothesis} placeholder="e.g. On days with ≥30 min Taiji, next-morning PVT lapse rate is lower than on days without."></textarea></label>
      <label class="field"><span>Intervention / exposure schedule</span>
        <textarea bind:value={pIntervention} placeholder="What will you actually do, and when?"></textarea></label>
      <label class="field"><span>Analysis plan (comparison and lag)</span>
        <textarea bind:value={pAnalysisPlan}></textarea></label>
      <div class="row">
        <label class="field"><span>Start</span><input type="date" bind:value={pStart} /></label>
        <label class="field"><span>End</span><input type="date" bind:value={pEnd} min={pStart} /></label>
      </div>
      <label class="field"><span>Adherence requirements (optional)</span>
        <input bind:value={pAdherence} placeholder="e.g. at least 5 practice days per week" /></label>
      <label class="field"><span>Context variables to record (optional)</span>
        <input bind:value={pContextVars} placeholder="e.g. sleep duration, caffeine, illness" /></label>
      <label class="field"><span>Stop / pause criteria (optional)</span>
        <input bind:value={pStop} placeholder="e.g. pause during illness or travel" /></label>
    </div>
    {#snippet footer()}
      <button class="btn" onclick={() => (protocolEditor = false)}>Cancel</button>
      <button class="btn primary" onclick={saveProtocol}
        disabled={!pTitle.trim() || !pQuestion.trim() || !pHypothesis.trim() || !pIntervention.trim() || !pAnalysisPlan.trim()}>
        Save draft
      </button>
    {/snippet}
  </Modal>
{/if}

{#if protoDetail}
  <Modal title={protoDetail.title} subtitle={`Version ${protoDetail.version} · ${protoDetail.status}`} onclose={() => (protoDetail = null)} width="640px">
    <div class="col" style="gap:12px;">
      <div><div class="overline">Question</div><p class="dim">{protoDetail.question}</p></div>
      <div><div class="overline">Hypothesis</div><p class="dim">{protoDetail.hypothesis}</p></div>
      <div><div class="overline">Intervention</div><p class="dim">{protoDetail.intervention_definition}</p></div>
      <div><div class="overline">Analysis plan</div><p class="dim">{protoDetail.analysis_plan}</p></div>
      <div class="row wrap">
        {#if protoDetail.start_date}<span class="pill">{fmtDate(protoDetail.start_date)} → {fmtDate(protoDetail.end_date)}</span>{/if}
        {#if protoDetail.adherence_requirements}<span class="pill">adherence: {protoDetail.adherence_requirements}</span>{/if}
        {#if protoDetail.stop_criteria}<span class="pill">stop: {protoDetail.stop_criteria}</span>{/if}
      </div>
      {#if protoDetail.conclusion}
        <div class="inset" style="padding:10px 14px;">
          <div class="overline">Conclusion</div>
          <p class="dim">{EVIDENCE_LABELS[protoDetail.conclusion]?.label}{protoDetail.conclusion_note ? ` — ${protoDetail.conclusion_note}` : ""}</p>
        </div>
      {/if}

      {#if (protoDetail.results ?? []).length > 0}
        <section>
          <div class="overline" style="margin-bottom:6px;">Linked results</div>
          <div class="col" style="gap:4px;">
            {#each protoDetail.results as r (r.id)}
              <div class="inset" style="padding:8px 12px;">
                <div class="row between wrap">
                  <span class="small">{fmtInstant(r.generated_at)} · n={r.included_count} · excluded {r.excluded_count} · missing {r.missing_count}</span>
                  <span class="pill" title={EVIDENCE_LABELS[r.evidence_label]?.hint}>{EVIDENCE_LABELS[r.evidence_label]?.label ?? r.evidence_label}</span>
                </div>
                <p class="small faint">
                  {r.exposure_definition.label ?? r.exposure_definition.kind} → {r.outcome_definition.label ?? r.outcome_definition.kind}
                  · lag {r.time_window.lag_days}d · window {r.time_window.from}…{r.time_window.to} · {r.analysis_version}
                  {#if r.is_stale === 1}· <span style="color:var(--caution);">stale</span>{/if}
                </p>
              </div>
            {/each}
          </div>
        </section>
      {/if}

      <div class="row wrap">
        {#if protoDetail.status === "draft"}
          <button class="btn sm primary" onclick={() => protocolAction(protoDetail, "status", "active")}>Activate</button>
          <button class="btn sm" onclick={() => openProtocolFrom(null, protoDetail)}>Edit draft</button>
        {:else if protoDetail.status === "active"}
          {#if protoDetail.analysis_spec}
            <button class="btn sm primary" onclick={() => runProtocolAnalysis(protoDetail)} disabled={runningProtoAnalysis}>
              Run predefined analysis
            </button>
          {/if}
          <button class="btn sm" onclick={() => openProtocolFrom(null, protoDetail)}
            title={protoDetail.results_locked === 1 ? "Results have been viewed — outcome/plan changes will create a new version" : "Edit protocol"}>
            {protoDetail.results_locked === 1 ? "Amend (new version)" : "Edit"}
          </button>
          <button class="btn sm" onclick={() => protocolAction(protoDetail, "status", "paused")}>Pause</button>
          <button class="btn sm" onclick={() => protocolAction(protoDetail, "conclude", "protocol_result_supported")}>Conclude: supported</button>
          <button class="btn sm" onclick={() => protocolAction(protoDetail, "conclude", "protocol_result_not_supported")}>not supported</button>
          <button class="btn sm" onclick={() => protocolAction(protoDetail, "conclude", "protocol_result_inconclusive")}>inconclusive</button>
        {:else if protoDetail.status === "paused"}
          <button class="btn sm primary" onclick={() => protocolAction(protoDetail, "status", "active")}>Resume</button>
          <button class="btn sm" onclick={() => protocolAction(protoDetail, "status", "cancelled")}>Cancel protocol</button>
        {/if}
      </div>
      <p class="small faint">A supported result means “consistent with the predefined hypothesis for me, in this period” — nothing more. Null and negative results are kept.</p>
    </div>
  </Modal>
{/if}

<style>
  .page { padding: 28px 32px 48px; max-width: 1000px; margin: 0 auto; }
  .page-head { display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 16px; flex-wrap: wrap; gap: 12px; }
  h1 { font-size: 27px; }
  .baseline-note {
    background: var(--info-wash); color: #9db1c5;
    padding: 8px 14px; border-radius: var(--r-md); margin-bottom: 14px;
  }
  .spec { display: grid; grid-template-columns: 1fr auto 1fr; gap: 16px; align-items: start; }
  @media (max-width: 860px) { .spec { grid-template-columns: 1fr; } }
  .spec-block { display: flex; flex-direction: column; gap: 8px; }
  .spec-arrow { display: flex; flex-direction: column; align-items: center; gap: 6px; padding-top: 18px; }
  .arrow { font-size: 24px; color: var(--paper-faint); }
  .counts { display: flex; flex-wrap: wrap; gap: 8px 24px; margin-bottom: 12px; }
  .counts div { display: flex; flex-direction: column; }
  .groups { display: grid; grid-template-columns: 1fr 1fr; gap: 1px; margin-bottom: 12px; overflow: hidden; }
  .group { padding: 12px 16px; display: flex; flex-direction: column; gap: 2px; background: var(--ink-1); }
  .group .mono { font-size: 22px; }
  .scatter { width: 100%; height: 130px; background: var(--ink-1); border-radius: var(--r-md); margin-bottom: 4px; }
  .caveats { margin-top: 10px; display: flex; flex-direction: column; gap: 3px; }
  .result-row { transition: border-color 120ms ease; }
  .result-row:hover { border-color: var(--ink-4); }
  .result-row.stale { opacity: 0.7; border-style: dashed; }
  th { color: var(--paper-faint); font-weight: 500; }
</style>
