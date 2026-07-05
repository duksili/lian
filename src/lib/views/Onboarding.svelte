<script lang="ts">
  import { api } from "../api";
  import { app, go, loadGlobals, reportError, toast } from "../state.svelte";
  import { todayStr } from "../format";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";

  let step = $state(0);
  const TOTAL = 5;

  // choices
  let enabledDims = $state<string[]>([]);
  let enabledTemplates = $state<string[]>([]);
  let quietStart = $state("21:30");
  let quietEnd = $state("07:30");
  let eveningCheckin = $state(true);
  let checkinTime = $state("20:30");
  let schedules = $state<Record<string, boolean>>({ pvt_v1: false, go_no_go_v1: false, physical_weekly_v1: false });
  let backupDir = $state<string | null>(null);
  let startBaseline = $state(true);
  let busy = $state(false);

  $effect(() => {
    if (app.loaded && enabledDims.length === 0) {
      enabledDims = app.dimensions.filter((d) => d.is_enabled === 1).map((d) => d.id);
      enabledTemplates = app.templates.filter((t) => !t.is_archived).map((t) => t.id);
    }
  });

  function toggle(list: string[], id: string): string[] {
    return list.includes(id) ? list.filter((x) => x !== id) : [...list, id];
  }

  async function finish() {
    busy = true;
    try {
      await api("dimensions.configure", { enabled_ids: enabledDims });
      for (const t of app.templates) {
        const wantArchived = !enabledTemplates.includes(t.id);
        if ((t.is_archived === 1) !== wantArchived) {
          await api("templates.set_archived", { id: t.id, archived: wantArchived });
        }
      }
      for (const [kind, enabled] of Object.entries(schedules)) {
        if (enabled) {
          const all = await api("assessments.schedules");
          const s = all.find((x: any) => x.kind === kind);
          await api("assessments.save_schedule", {
            kind, enabled: true, weekdays: s.weekdays, window_start: s.window_start, window_end: s.window_end,
          });
        }
      }
      if (eveningCheckin) {
        const rules = await api("reminders.rules");
        const rule = rules.find((r: any) => r.kind === "evening_checkin");
        if (rule) {
          await api("reminders.save_rule", {
            id: rule.id, kind: rule.kind, label: rule.label,
            time_of_day: checkinTime, weekdays: [], enabled: true,
          });
        }
        try {
          if (!(await isPermissionGranted())) await requestPermission();
        } catch { /* notification permission is optional */ }
      }
      await api("settings.set", {
        timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
        quiet_hours_start: quietStart,
        quiet_hours_end: quietEnd,
        backup_dir: backupDir,
        baseline_start: startBaseline ? todayStr() : null,
        onboarding_complete: true,
      });
      await loadGlobals();
      go({ name: "today" });
      toast("Welcome. Everything here stays on this machine.", "ok");
    } catch (e) { reportError(e); }
    busy = false;
  }
</script>

<div class="onboard">
  <div class="panel">
    <div class="steps-indicator" aria-hidden="true">
      {#each Array(TOTAL) as _, i}
        <span class="step-dot" class:active={i === step} class:done={i < step}></span>
      {/each}
    </div>

    {#if step === 0}
      <div class="glyph display">練</div>
      <h1 class="display">LIAN</h1>
      <p class="lede dim">
        A private workstation for practice and personal research —
        meditation, Taiji, movement, sleep, reflection, and repeatable assessments, kept over years.
      </p>
      <div class="promises">
        <div class="promise"><span class="p-mark">⌂</span><div>
          <p>Local-first, yours alone</p>
          <p class="small faint">Everything lives in a SQLite file on this machine. No account, no cloud, no telemetry. Nothing leaves without your explicit export.</p>
        </div></div>
        <div class="promise"><span class="p-mark">◦</span><div>
          <p>Unknown stays unknown</p><p class="small faint">A day you didn't log is a day without data — never “failed”, never a broken streak. There are no streaks.</p>
        </div></div>
        <div class="promise"><span class="p-mark">∴</span><div>
          <p>Honest about evidence</p><p class="small faint">Patterns are labeled as observations or signals, never proof. You decide what deserves a real protocol.</p>
        </div></div>
      </div>
    {:else if step === 1}
      <h2 class="display">What do you want to notice daily?</h2>
      <p class="small faint" style="margin-bottom:16px;">A brief evening check-in on a few 1–5 scales. Three to five works well; change any time.</p>
      <div class="choice-grid">
        {#each app.dimensions as d (d.id)}
          <button class="choice" class:on={enabledDims.includes(d.id)} onclick={() => (enabledDims = toggle(enabledDims, d.id))}>
            <span>{d.label}</span>
            <span class="small faint">{d.anchor_low} → {d.anchor_high}</span>
          </button>
        {/each}
      </div>
    {:else if step === 2}
      <h2 class="display">Which practices will you log?</h2>
      <p class="small faint" style="margin-bottom:16px;">Deselected templates are archived, not deleted; add your own later in Settings.</p>
      <div class="choice-grid">
        {#each app.templates as t (t.id)}
          <button class="choice cat-{t.color}" class:on={enabledTemplates.includes(t.id)} onclick={() => (enabledTemplates = toggle(enabledTemplates, t.id))}>
            <span><span class="e-glyph">{t.glyph}</span> {t.name}</span>
          </button>
        {/each}
      </div>
    {:else if step === 3}
      <h2 class="display">Rhythm and quiet</h2>
      <p class="small faint" style="margin-bottom:16px;">Reminders are gentle, few, and always yours to silence. Quiet hours are absolute.</p>
      <div class="col" style="gap:16px; text-align:left;">
        <div class="row wrap" style="gap:16px;">
          <label class="field"><span>Quiet from</span><input type="time" bind:value={quietStart} /></label>
          <label class="field"><span>until</span><input type="time" bind:value={quietEnd} /></label>
        </div>
        <label class="row" style="gap:10px; cursor:pointer;">
          <input type="checkbox" bind:checked={eveningCheckin} style="width:auto;" />
          <span>Evening check-in reminder at</span>
          <input type="time" bind:value={checkinTime} disabled={!eveningCheckin} style="width:96px;" />
        </label>
        <div>
          <p style="margin-bottom:8px;">Assessment schedules <span class="small faint">(opt-in; you can also run them ad hoc)</span></p>
          <div class="col" style="gap:6px;">
            <label class="row" style="gap:10px; cursor:pointer;">
              <input type="checkbox" bind:checked={schedules.pvt_v1} style="width:auto;" />
              <span class="small">PVT — Mon/Wed/Fri mornings (5 min)</span></label>
            <label class="row" style="gap:10px; cursor:pointer;">
              <input type="checkbox" bind:checked={schedules.go_no_go_v1} style="width:auto;" />
              <span class="small">Go/No-Go — Tue/Thu mornings (4 min)</span></label>
            <label class="row" style="gap:10px; cursor:pointer;">
              <input type="checkbox" bind:checked={schedules.physical_weekly_v1} style="width:auto;" />
              <span class="small">Physical check — Saturdays (2 min)</span></label>
          </div>
        </div>
      </div>
    {:else}
      <h2 class="display">Safekeeping</h2>
      <p class="small faint" style="margin-bottom:16px;">Your data deserves a second copy. You can defer this — LIAN will remind you in Settings, not nag you.</p>
      <div class="col" style="gap:14px; text-align:left;">
        <div class="row">
          <button class="btn" onclick={async () => {
            const d = await openDialog({ directory: true, title: "Choose backup destination" });
            if (typeof d === "string") backupDir = d;
          }}>{backupDir ? "Change destination" : "Choose backup destination…"}</button>
          {#if backupDir}<span class="small mono dim">{backupDir}</span>{:else}<span class="small faint">deferred for now</span>{/if}
        </div>
        <label class="row" style="gap:10px; cursor:pointer;">
          <input type="checkbox" bind:checked={startBaseline} style="width:auto;" />
          <div>
            <span>Begin a 5-week baseline today</span>
            <p class="small faint">During baseline LIAN records and describes but draws no conclusions — a fair starting point for later comparison.</p>
          </div>
        </label>
      </div>
    {/if}

    <div class="nav-row">
      {#if step > 0}
        <button class="btn ghost" onclick={() => step--}>Back</button>
      {:else}
        <span></span>
      {/if}
      {#if step < TOTAL - 1}
        <button class="btn primary" onclick={() => step++}
          disabled={step === 1 && enabledDims.length === 0}>Continue</button>
      {:else}
        <button class="btn primary" onclick={finish} disabled={busy}>Begin</button>
      {/if}
    </div>
  </div>
</div>

<style>
  .onboard {
    height: 100%; overflow-y: auto;
    display: flex; align-items: center; justify-content: center;
    padding: 40px 24px;
  }
  .panel { max-width: 620px; width: 100%; text-align: center; }
  .steps-indicator { display: flex; gap: 8px; justify-content: center; margin-bottom: 36px; }
  .step-dot { width: 6px; height: 6px; border-radius: 99px; background: var(--ink-4); transition: background 200ms ease; }
  .step-dot.active { background: var(--cinnabar); }
  .step-dot.done { background: var(--cinnabar-dim); }
  .glyph { font-size: 54px; color: var(--cinnabar); margin-bottom: 8px; }
  h1 { font-size: 30px; letter-spacing: 0.28em; margin-bottom: 14px; }
  h2 { font-size: 24px; margin-bottom: 8px; }
  .lede { font-size: 15.5px; max-width: 460px; margin: 0 auto 28px; }
  .promises { display: flex; flex-direction: column; gap: 16px; text-align: left; max-width: 480px; margin: 0 auto; }
  .promise { display: flex; gap: 14px; align-items: flex-start; }
  .p-mark { color: var(--cinnabar); width: 22px; text-align: center; font-size: 16px; margin-top: 1px; }
  .choice-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(220px, 1fr)); gap: 8px; text-align: left; }
  .choice {
    display: flex; flex-direction: column; gap: 2px; align-items: flex-start;
    padding: 11px 14px; border-radius: var(--r-md);
    background: var(--ink-1); border: 1px solid var(--line); color: var(--paper-dim);
    transition: all 100ms ease;
  }
  .choice.on { border-color: var(--cinnabar-dim); color: var(--paper); background: var(--cinnabar-wash); }
  .e-glyph { color: var(--cat, var(--paper-faint)); }
  .nav-row { display: flex; justify-content: space-between; margin-top: 36px; }
</style>
