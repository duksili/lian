<script lang="ts">
  import Modal from "../components/Modal.svelte";
  import { api } from "../api";
  import { activeTemplates, toast, reportError, bump } from "../state.svelte";
  import { todayStr, WEEKDAY_LABELS } from "../format";

  let { existing = null, initialDate = todayStr(), onclose }: {
    existing?: any | null;
    initialDate?: string;
    onclose: () => void;
  } = $props();

  const KINDS = [
    ["activity", "Practice / activity"],
    ["assessment", "Assessment"],
    ["recovery", "Recovery / rest"],
    ["commitment", "Commitment"],
    ["custom", "Custom"],
  ] as const;

  let kind = $state(existing?.kind ?? "activity");
  let title = $state(existing?.title ?? "");
  let templateId = $state(existing?.activity_template_id ?? "");
  let assessmentKind = $state(existing?.assessment_kind ?? "pvt_v1");
  let date = $state(existing?.local_date ?? initialDate);
  let hasTime = $state(existing ? !existing.date_only : true);
  let time = $state("07:30");
  let durationMin = $state<string>(existing?.target_duration_seconds ? String(existing.target_duration_seconds / 60) : "");
  let note = $state(existing?.note ?? "");
  let reminderOffset = $state<string>(existing?.reminder_offset_minutes != null ? String(existing.reminder_offset_minutes) : "");
  let determinationId = $state(existing?.determination_id ?? "");

  // recurrence (only for new plans)
  let recurring = $state(false);
  let frequency = $state<"daily" | "weekly" | "monthly">("weekly");
  let weekdays = $state<number[]>([]);
  let until = $state("");

  let determinations = $state<any[]>([]);
  let saving = $state(false);

  $effect(() => {
    if (existing?.scheduled_start) {
      const d = new Date(existing.scheduled_start);
      time = `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
    }
  });

  $effect(() => {
    api("determinations.list", {}).then((d) => (determinations = d)).catch(() => {});
  });

  const effectiveTitle = $derived.by(() => {
    if (title.trim()) return title.trim();
    if (kind === "activity" && templateId) {
      return activeTemplates().find((t) => t.id === templateId)?.name ?? "";
    }
    if (kind === "assessment") {
      return assessmentKind === "pvt_v1" ? "PVT" : assessmentKind === "go_no_go_v1" ? "Go / No-Go" : "Weekly physical check";
    }
    return "";
  });

  function toggleWeekday(i: number) {
    weekdays = weekdays.includes(i) ? weekdays.filter((w) => w !== i) : [...weekdays, i].sort();
  }

  async function save() {
    if (!effectiveTitle) { toast("Give the plan a title", "error"); return; }
    saving = true;
    try {
      const common = {
        title: effectiveTitle,
        kind,
        activity_template_id: kind === "activity" ? templateId || null : null,
        assessment_kind: kind === "assessment" ? assessmentKind : null,
        time_of_day: hasTime ? time : null,
        target_duration_seconds: durationMin.trim() === "" ? null : Number(durationMin) * 60,
        note: note || null,
        determination_id: determinationId || null,
        reminder_offset_minutes: reminderOffset.trim() === "" ? null : Number(reminderOffset),
      };
      if (recurring && !existing) {
        await api("series.save", {
          ...common,
          frequency,
          weekdays: frequency === "weekly" ? weekdays : [],
          starts_on: date,
          until: until || null,
          duration_minutes: durationMin.trim() === "" ? null : Number(durationMin),
        });
      } else {
        await api("plans.save", { ...common, id: existing?.id ?? null, local_date: date });
      }
      bump();
      toast(existing ? "Plan updated" : "Planned", "ok");
      onclose();
    } catch (e) { reportError(e); }
    saving = false;
  }

  async function remove() {
    try {
      await api("plans.delete", { id: existing.id });
      bump(); onclose();
    } catch (e) { reportError(e); }
  }
</script>

<Modal title={existing ? "Edit plan" : "Plan"}
  subtitle="A plan records intention. Completing it later is a separate, explicit act."
  onclose={onclose}>
  <div class="col" style="gap:13px;">
    <div class="row wrap" role="radiogroup" aria-label="Kind">
      {#each KINDS as [k, label]}
        <button class="pill" class:accent={kind === k} style="cursor:pointer;" onclick={() => (kind = k)}>{label}</button>
      {/each}
    </div>

    {#if kind === "activity"}
      <label class="field"><span>Activity</span>
        <select bind:value={templateId}>
          <option value="">— choose —</option>
          {#each activeTemplates() as t}<option value={t.id}>{t.name}</option>{/each}
        </select>
      </label>
    {:else if kind === "assessment"}
      <label class="field"><span>Assessment</span>
        <select bind:value={assessmentKind}>
          <option value="pvt_v1">PVT</option>
          <option value="go_no_go_v1">Go / No-Go</option>
          <option value="physical_weekly_v1">Weekly physical check</option>
        </select>
      </label>
    {/if}

    <label class="field"><span>Title {kind === "activity" || kind === "assessment" ? "(optional — defaults to the activity)" : ""}</span>
      <input bind:value={title} placeholder={effectiveTitle || "e.g. Evening restraint: no screens after 22:00"} />
    </label>

    <div class="row wrap">
      <label class="field"><span>{recurring ? "First date" : "Date"}</span>
        <input type="date" bind:value={date} /></label>
      <label class="field"><span>Time</span>
        <div class="row" style="gap:6px;">
          <input type="time" bind:value={time} disabled={!hasTime} style="width:100px;" />
          <button class="btn ghost sm" onclick={() => (hasTime = !hasTime)}>{hasTime ? "timed" : "any time"}</button>
        </div>
      </label>
      <label class="field"><span>Target (min)</span>
        <input type="number" min="0" bind:value={durationMin} placeholder="—" style="width:90px;" /></label>
    </div>

    {#if !existing}
      <div class="inset" style="padding:12px 14px;">
        <label class="row" style="cursor:pointer; gap:8px;">
          <input type="checkbox" bind:checked={recurring} style="width:auto;" />
          <span class="small">Repeats</span>
        </label>
        {#if recurring}
          <div class="col" style="gap:10px; margin-top:10px;">
            <div class="row">
              {#each ["daily", "weekly", "monthly"] as f}
                <button class="btn sm" class:primary={frequency === f} onclick={() => (frequency = f as any)}>{f}</button>
              {/each}
            </div>
            {#if frequency === "weekly"}
              <div class="row" style="gap:4px;">
                {#each WEEKDAY_LABELS as wl, i}
                  <button class="wd" class:active={weekdays.includes(i)} onclick={() => toggleWeekday(i)}>{wl}</button>
                {/each}
              </div>
            {/if}
            <label class="field"><span>Until (optional)</span>
              <input type="date" bind:value={until} min={date} style="width:160px;" /></label>
            <p class="small faint">Changing the series later never rewrites past occurrences.</p>
          </div>
        {/if}
      </div>
    {/if}

    <div class="grid2">
      <label class="field"><span>Reminder (minutes before)</span>
        <input type="number" min="0" bind:value={reminderOffset} placeholder="no reminder" /></label>
      <label class="field"><span>Linked determination (optional)</span>
        <select bind:value={determinationId}>
          <option value="">none</option>
          {#each determinations as d}<option value={d.id}>{d.title}</option>{/each}
        </select>
      </label>
    </div>
    <label class="field"><span>Note</span><textarea bind:value={note} placeholder="optional"></textarea></label>
  </div>
  {#snippet footer()}
    {#if existing}
      <button class="btn danger" onclick={remove}>Delete plan</button>
    {/if}
    <div class="grow"></div>
    <button class="btn" onclick={onclose}>Cancel</button>
    <button class="btn primary" onclick={save} disabled={saving}>{existing ? "Save" : recurring ? "Create series" : "Plan it"}</button>
  {/snippet}
</Modal>

<style>
  .wd {
    padding: 4px 8px; font-size: 12px; border-radius: var(--r-xs);
    background: var(--ink-1); border: 1px solid var(--line); color: var(--paper-dim);
  }
  .wd.active { background: var(--cinnabar-wash); border-color: var(--cinnabar-dim); color: var(--cinnabar-bright); }
</style>
