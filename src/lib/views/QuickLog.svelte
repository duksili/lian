<script lang="ts">
  import Modal from "../components/Modal.svelte";
  import RatingScale from "../components/RatingScale.svelte";
  import { api } from "../api";
  import { app, quickLog, timer, startTimer, stopTimer, toast, reportError, bump, activeTemplates } from "../state.svelte";
  import { todayStr, nowLocalTimeHHMM, toInstant, fmtDuration, fmtDate } from "../format";
  import { onMount } from "svelte";

  let templateId = $state(quickLog.templateId ?? "");
  let date = $state(todayStr());
  let time = $state(nowLocalTimeHHMM());
  let timeKnown = $state(true);
  let durationMin = $state<string>(quickLog.durationSeconds != null ? String(Math.round(quickLog.durationSeconds / 60)) : "");
  let subtype = $state("");
  let intensity = $state<number | null>(null);
  let quality = $state<number | null>(null);
  let bodyBefore = $state("");
  let bodyAfter = $state("");
  let location = $state("");
  let note = $state("");
  let planId = $state<string | null>(quickLog.planId);
  let source = $state<string>(quickLog.source);
  let showMore = $state(false);
  let saving = $state(false);
  let suggestions = $state<any[]>([]);
  let editingId = $state<string | null>(quickLog.eventId);

  const template = $derived(app.templates.find((t) => t.id === templateId));
  const subtypes = $derived((template?.subtypes ?? []) as string[]);

  onMount(async () => {
    // Editing an existing event: load it fully.
    if (quickLog.eventId) {
      try {
        const ev = await api("events.get", { id: quickLog.eventId });
        templateId = ev.template_id;
        date = ev.local_date;
        timeKnown = ev.time_known === 1;
        if (ev.occurred_at) {
          const d = new Date(ev.occurred_at);
          time = `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
        }
        durationMin = ev.duration_seconds != null ? String(Math.round(ev.duration_seconds / 60)) : "";
        subtype = ev.subtype ?? "";
        intensity = ev.intensity ?? null;
        quality = ev.perceived_quality ?? null;
        bodyBefore = ev.body_state_before ?? "";
        bodyAfter = ev.body_state_after ?? "";
        location = ev.location ?? "";
        note = ev.note ?? "";
        planId = ev.plan_id ?? null;
        showMore = true;
      } catch (e) { reportError(e); }
    } else if (quickLog.occurredAt) {
      const d = new Date(quickLog.occurredAt);
      date = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
      time = `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
    }
    if (!templateId && activeTemplates().length > 0) templateId = activeTemplates()[0].id;
  });

  // When a timer is running and the modal opens, offer to finish it.
  const timerActive = $derived(timer.running);

  function finishTimer() {
    const { seconds, startedEpoch } = stopTimer();
    const started = new Date(startedEpoch);
    durationMin = String(Math.max(1, Math.round(seconds / 60)));
    date = `${started.getFullYear()}-${String(started.getMonth() + 1).padStart(2, "0")}-${String(started.getDate()).padStart(2, "0")}`;
    time = `${String(started.getHours()).padStart(2, "0")}:${String(started.getMinutes()).padStart(2, "0")}`;
    source = "timer";
    if (timer.templateId) templateId = timer.templateId;
  }

  function beginTimer() {
    startTimer(templateId || null);
    quickLog.open = false;
    toast("Timer started — press L to finish when done", "ok");
  }

  async function save(thenClose = true) {
    if (!templateId) { toast("Choose an activity", "error"); return; }
    saving = true;
    try {
      const payload: Record<string, unknown> = {
        id: editingId,
        template_id: templateId,
        duration_seconds: durationMin.trim() === "" ? null : Math.round(Number(durationMin) * 60),
        subtype: subtype || null,
        intensity,
        perceived_quality: quality,
        body_state_before: bodyBefore || null,
        body_state_after: bodyAfter || null,
        location: location || null,
        note: note || null,
        plan_id: planId,
        source,
      };
      if (timeKnown) payload.occurred_at = toInstant(date, time);
      else payload.local_date = date;
      const saved = await api("events.save", payload);
      bump();
      toast(editingId ? "Entry updated" : "Practice recorded", "ok");
      if (!editingId) {
        // Offer plan links only after a fresh log, non-blocking.
        try {
          suggestions = await api("plans.suggest_for_event", { event_id: saved.id });
          if (suggestions.length > 0 && !planId) {
            editingId = saved.id;
            saving = false;
            return; // keep open to show suggestions
          }
        } catch { /* suggestions are optional */ }
      }
      if (thenClose) quickLog.open = false;
    } catch (e) { reportError(e); }
    saving = false;
  }

  async function linkTo(plan: any) {
    try {
      await api("plans.link_event", { plan_id: plan.id, event_id: editingId });
      bump();
      toast("Linked to plan", "ok");
      quickLog.open = false;
    } catch (e) { reportError(e); }
  }
</script>

<Modal
  title={editingId && !suggestions.length ? "Edit entry" : suggestions.length ? "Recorded" : "Log practice"}
  subtitle={suggestions.length ? "This may belong to a plan — link it if you like." : "Only the activity is required. Everything else can wait."}
  onclose={() => (quickLog.open = false)}
>
  {#if suggestions.length > 0}
    <div class="col">
      {#each suggestions as s}
        <button class="suggestion inset" onclick={() => linkTo(s)}>
          <span>{s.title}</span>
          <span class="small faint">{fmtDate(s.local_date)}</span>
          <span class="pill accent">link</span>
        </button>
      {/each}
      <button class="btn ghost" onclick={() => (quickLog.open = false)}>Leave unlinked</button>
    </div>
  {:else}
    <div class="col" style="gap: 14px;">
      {#if timerActive}
        <button class="timer-banner" onclick={finishTimer}>
          ● Timer running — {fmtDuration(timer.elapsed)}. Click to use it for this entry.
        </button>
      {/if}

      <div class="templates" role="radiogroup" aria-label="Activity">
        {#each activeTemplates() as t}
          <button
            class="template cat-{t.color}"
            class:active={templateId === t.id}
            role="radio"
            aria-checked={templateId === t.id}
            onclick={() => { templateId = t.id; subtype = ""; }}
          >
            <span class="t-glyph">{t.glyph}</span>
            <span>{t.name}</span>
          </button>
        {/each}
      </div>

      <div class="row wrap">
        <label class="field">
          <span>Date</span>
          <input type="date" bind:value={date} max={todayStr()} />
        </label>
        <label class="field">
          <span>Time</span>
          <div class="row" style="gap:6px;">
            <input type="time" bind:value={time} disabled={!timeKnown} style="width:104px;" />
            <button class="btn ghost sm" onclick={() => (timeKnown = !timeKnown)}
              title="Toggle whether the time of day is known">
              {timeKnown ? "known" : "time unknown"}
            </button>
          </div>
        </label>
        <label class="field">
          <span>Duration (min)</span>
          <input type="number" min="0" step="1" bind:value={durationMin} placeholder="unknown" style="width:104px;" />
        </label>
      </div>

      {#if subtypes.length > 0}
        <div class="row wrap" role="radiogroup" aria-label="Subtype">
          {#each subtypes as st}
            <button class="pill subtype" class:accent={subtype === st}
              onclick={() => (subtype = subtype === st ? "" : st)}>{st}</button>
          {/each}
        </div>
      {/if}

      {#if showMore}
        <div class="col" style="gap:12px;">
          {#if template?.supports_intensity}
            <label class="field">
              <span>Intensity</span>
              <RatingScale value={intensity} anchorLow="very light" anchorHigh="maximal" onchange={(v) => (intensity = v)} />
            </label>
          {/if}
          <label class="field">
            <span>Perceived quality</span>
            <RatingScale value={quality} anchorLow="scattered / poor" anchorHigh="deep / excellent" onchange={(v) => (quality = v)} />
          </label>
          {#if template?.supports_body_state}
            <div class="grid2">
              <label class="field"><span>Body before</span>
                <input bind:value={bodyBefore} placeholder="e.g. stiff shoulders" /></label>
              <label class="field"><span>Body after</span>
                <input bind:value={bodyAfter} placeholder="e.g. loose, warm" /></label>
            </div>
          {/if}
          <div class="grid2">
            <label class="field"><span>Location / context</span>
              <input bind:value={location} placeholder="optional" /></label>
          </div>
          <label class="field"><span>Note</span>
            <textarea bind:value={note} placeholder="private note, optional"></textarea></label>
        </div>
      {:else}
        <button class="btn ghost sm" style="align-self:flex-start;" onclick={() => (showMore = true)}>
          + details (quality, body state, note)
        </button>
      {/if}
    </div>
  {/if}

  {#snippet footer()}
    {#if suggestions.length === 0}
      {#if !editingId && !timerActive}
        <button class="btn ghost" onclick={beginTimer} disabled={!templateId}>Start timer instead</button>
      {/if}
      <div class="grow"></div>
      <button class="btn" onclick={() => (quickLog.open = false)}>Cancel</button>
      <button class="btn primary" onclick={() => save()} disabled={saving}>
        {editingId ? "Save changes" : "Record"}
      </button>
    {/if}
  {/snippet}
</Modal>

<style>
  .templates { display: flex; flex-wrap: wrap; gap: 7px; }
  .template {
    display: inline-flex; align-items: center; gap: 8px;
    padding: 7px 12px; border-radius: var(--r-md);
    background: var(--ink-1); border: 1px solid var(--line);
    font-size: 13px; color: var(--paper-dim);
    transition: all 100ms ease;
  }
  .template:hover { border-color: var(--ink-4); color: var(--paper); }
  .template.active {
    border-color: var(--cat, var(--cinnabar));
    color: var(--paper);
    background: color-mix(in srgb, var(--cat, var(--cinnabar)) 12%, var(--ink-1));
  }
  .t-glyph { color: var(--cat, var(--paper-faint)); }
  .subtype { cursor: pointer; border: none; }
  .timer-banner {
    background: var(--ok-wash); color: var(--ok);
    border-radius: var(--r-md); padding: 10px 14px;
    font-size: 13px; text-align: left;
  }
  .suggestion {
    display: flex; align-items: center; gap: 10px; justify-content: space-between;
    padding: 12px 14px; text-align: left;
  }
  .suggestion:hover { border-color: var(--cinnabar-dim); }
</style>
