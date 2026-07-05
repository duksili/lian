<script lang="ts">
  import { api, dataLocation, restoreBackup, purgeAllData } from "../api";
  import { app, dataVersion, loadGlobals, toast, reportError, bump } from "../state.svelte";
  import { fmtInstant, todayStr, WEEKDAY_LABELS } from "../format";
  import Modal from "../components/Modal.svelte";
  import { open as openDialog } from "@tauri-apps/plugin-dialog";
  import { revealItemInDir } from "@tauri-apps/plugin-opener";

  let dbPath = $state("");
  let rules = $state<any[]>([]);
  let backups = $state<any[]>([]);
  let templateEditor = $state<any | null>(null);
  let confirmPurge = $state(false);
  let purgeText = $state("");
  let busy = $state(false);

  // template editor fields
  let tName = $state(""); let tSubtypes = $state(""); let tIntensity = $state(false); let tBody = $state(false);

  $effect(() => {
    dataVersion.n;
    load();
  });

  async function load() {
    try {
      const [r, b, loc] = await Promise.all([
        api("reminders.rules"), api("backup.list"), dataLocation().catch(() => ""),
      ]);
      rules = r; backups = b; dbPath = loc;
    } catch (e) { reportError(e); }
  }

  async function set(patch: Record<string, unknown>) {
    try {
      app.settings = await api("settings.set", patch);
    } catch (e) { reportError(e); }
  }

  async function toggleDimension(dim: any) {
    const enabled = app.dimensions.filter((d) => d.is_enabled === 1 && d.id !== dim.id).map((d) => d.id);
    if (dim.is_enabled !== 1) enabled.push(dim.id);
    try {
      await api("dimensions.configure", { enabled_ids: enabled });
      await loadGlobals();
    } catch (e) { reportError(e); }
  }

  function openTemplateEditor(t: any | null) {
    templateEditor = t ?? { id: null };
    tName = t?.name ?? "";
    tSubtypes = (t?.subtypes ?? []).join(", ");
    tIntensity = t?.supports_intensity === 1;
    tBody = t?.supports_body_state === 1;
  }

  async function saveTemplate() {
    try {
      await api("templates.save", {
        id: templateEditor.id,
        name: tName,
        subtypes: tSubtypes.split(",").map((s) => s.trim()).filter(Boolean),
        supports_intensity: tIntensity,
        supports_body_state: tBody,
      });
      templateEditor = null;
      await loadGlobals();
      bump();
      toast("Template saved", "ok");
    } catch (e) { reportError(e); }
  }

  async function archiveTemplate(t: any, archived: boolean) {
    try {
      await api("templates.set_archived", { id: t.id, archived });
      await loadGlobals();
      bump();
    } catch (e) { reportError(e); }
  }

  async function moveTemplate(t: any, dir: number) {
    const active = app.templates.filter((x) => !x.is_archived).map((x) => x.id);
    const i = active.indexOf(t.id);
    const j = i + dir;
    if (j < 0 || j >= active.length) return;
    [active[i], active[j]] = [active[j], active[i]];
    try {
      await api("templates.reorder", { ordered_ids: active });
      await loadGlobals();
    } catch (e) { reportError(e); }
  }

  async function toggleRule(rule: any) {
    try {
      await api("reminders.set_enabled", { id: rule.id, enabled: rule.enabled !== 1 });
      bump();
    } catch (e) { reportError(e); }
  }

  async function updateRuleTime(rule: any, time: string) {
    try {
      await api("reminders.save_rule", {
        id: rule.id, kind: rule.kind, label: rule.label,
        time_of_day: time, weekdays: rule.weekdays, enabled: rule.enabled === 1,
      });
      bump();
    } catch (e) { reportError(e); }
  }

  async function chooseBackupDir(): Promise<string | null> {
    const dir = await openDialog({ directory: true, title: "Choose backup destination" });
    return typeof dir === "string" ? dir : null;
  }

  async function createBackup() {
    busy = true;
    try {
      let dir = app.settings.backup_dir;
      if (!dir) dir = await chooseBackupDir();
      if (!dir) { busy = false; return; }
      const res = await api("backup.create", { dest_dir: dir });
      await loadGlobals();
      bump();
      toast(`Backup written: ${res.path}`, "ok");
    } catch (e) { reportError(e); }
    busy = false;
  }

  async function exportData() {
    busy = true;
    try {
      const dir = await openDialog({ directory: true, title: "Choose export destination" });
      if (typeof dir !== "string") { busy = false; return; }
      const res = await api("export.csv", { dest_dir: dir });
      toast(`Exported ${res.manifest.tables.length} tables + SQLite copy to ${res.path}`, "ok");
    } catch (e) { reportError(e); }
    busy = false;
  }

  async function restore() {
    try {
      const file = await openDialog({
        title: "Choose a LIAN backup (.sqlite3)",
        filters: [{ name: "LIAN backup", extensions: ["sqlite3"] }],
      });
      if (typeof file !== "string") return;
      const check = await api("backup.verify", { path: file });
      const ok = confirm(
        `Restore from backup?\n\nIntegrity: ${check.integrity}\nSchema version: ${check.schema_version}\nManifest checksum match: ${check.manifest_found_and_matches ? "yes" : "no / missing"}\n\nA safety copy of the current data is made first.`
      );
      if (!ok) return;
      busy = true;
      const res = await restoreBackup(file);
      busy = false;
      await loadGlobals();
      bump();
      toast(`Restored. Safety copy of previous data: ${res.safety_copy}`, "ok");
    } catch (e) { busy = false; reportError(e); }
  }

  async function purge() {
    if (purgeText !== "delete everything") return;
    busy = true;
    try {
      await purgeAllData();
      confirmPurge = false; purgeText = "";
      await loadGlobals();
      bump();
      toast("All local data permanently deleted. Starting fresh.", "ok");
    } catch (e) { reportError(e); }
    busy = false;
  }

  const ruleLabels: Record<string, string> = {
    evening_checkin: "Evening check-in",
    weekly_review: "Weekly review (Sunday)",
    monthly_review: "Monthly review (1st of month)",
    determination_review: "Determination review",
    recovery: "Yesterday recovery prompt",
  };
</script>

<div class="page">
  <header class="page-head">
    <div>
      <div class="overline">Configuration</div>
      <h1 class="display">Settings</h1>
    </div>
  </header>

  <div class="sections">
    <!-- ============ general ============ -->
    <section class="card pad">
      <h2 class="sec-title display">General</h2>
      <div class="row wrap" style="gap:16px;">
        <label class="field"><span>Home timezone</span>
          <input value={app.settings.timezone} style="width:220px;"
            onchange={(e) => set({ timezone: (e.target as HTMLInputElement).value })} />
        </label>
        <label class="field"><span>Baseline</span>
          {#if app.settings.baseline_start}
            <div class="row">
              <span class="pill info">started {app.settings.baseline_start} · {app.settings.baseline_weeks} weeks</span>
              <button class="btn ghost sm" onclick={() => set({ baseline_start: null })}>clear</button>
            </div>
          {:else}
            <div class="row">
              <select onchange={(e) => set({ baseline_weeks: Number((e.target as HTMLSelectElement).value) })} value={String(app.settings.baseline_weeks ?? 5)}>
                <option value="4">4 weeks</option><option value="5">5 weeks</option><option value="6">6 weeks</option>
              </select>
              <button class="btn sm" onclick={() => set({ baseline_start: todayStr() })}>Start baseline today</button>
            </div>
          {/if}
        </label>
        <label class="row" style="gap:8px; cursor:pointer; align-self:flex-end; padding-bottom:6px;">
          <input type="checkbox" checked={app.settings.close_to_tray}
            onchange={(e) => set({ close_to_tray: (e.target as HTMLInputElement).checked })} style="width:auto;" />
          <span class="small dim">closing the window keeps LIAN in the tray</span>
        </label>
      </div>
    </section>

    <!-- ============ check-in dimensions ============ -->
    <section class="card pad">
      <h2 class="sec-title display">Daily check-in dimensions</h2>
      <p class="small faint" style="margin-bottom:10px;">A small set works best. Scales are 1–5 with fixed anchors.</p>
      <div class="dim-grid">
        {#each app.dimensions as dim (dim.id)}
          <button class="dim-chip" class:on={dim.is_enabled === 1} onclick={() => toggleDimension(dim)}>
            <span>{dim.label}</span>
            <span class="small faint">{dim.anchor_low} → {dim.anchor_high}</span>
          </button>
        {/each}
      </div>
    </section>

    <!-- ============ activity templates ============ -->
    <section class="card pad">
      <div class="row between" style="margin-bottom:10px;">
        <h2 class="sec-title display" style="margin:0;">Activity templates</h2>
        <button class="btn sm" onclick={() => openTemplateEditor(null)}>+ New template</button>
      </div>
      <div class="col" style="gap:5px;">
        {#each app.templates as t (t.id)}
          <div class="tpl-row cat-{t.color}" class:archived={t.is_archived === 1}>
            <span class="e-glyph">{t.glyph}</span>
            <span class="grow">{t.name}
              {#if t.is_archived === 1}<span class="pill" style="margin-left:8px;">archived — history preserved</span>{/if}
            </span>
            {#if t.is_archived !== 1}
              <button class="btn ghost sm" onclick={() => moveTemplate(t, -1)} title="move up">↑</button>
              <button class="btn ghost sm" onclick={() => moveTemplate(t, 1)} title="move down">↓</button>
              <button class="btn ghost sm" onclick={() => openTemplateEditor(t)}>edit</button>
              <button class="btn ghost sm" onclick={() => archiveTemplate(t, true)}>archive</button>
            {:else}
              <button class="btn ghost sm" onclick={() => archiveTemplate(t, false)}>restore</button>
            {/if}
          </div>
        {/each}
      </div>
    </section>

    <!-- ============ reminders ============ -->
    <section class="card pad">
      <h2 class="sec-title display">Reminders &amp; quiet hours</h2>
      <div class="row wrap" style="gap:16px; margin-bottom:14px;">
        <label class="field"><span>Quiet from</span>
          <input type="time" value={app.settings.quiet_hours_start}
            onchange={(e) => set({ quiet_hours_start: (e.target as HTMLInputElement).value })} /></label>
        <label class="field"><span>until</span>
          <input type="time" value={app.settings.quiet_hours_end}
            onchange={(e) => set({ quiet_hours_end: (e.target as HTMLInputElement).value })} /></label>
        <label class="field"><span>Global pause</span>
          <button class="btn sm" class:primary={app.settings.notifications_paused}
            onclick={() => api("reminders.set_pause", { paused: !app.settings.notifications_paused }).then(loadGlobals)}>
            {app.settings.notifications_paused ? "paused — click to resume" : "active — click to pause all"}
          </button>
        </label>
        <label class="row" style="gap:8px; cursor:pointer; align-self:flex-end; padding-bottom:6px;">
          <input type="checkbox" checked={app.settings.lock_screen_minimal}
            onchange={(e) => set({ lock_screen_minimal: (e.target as HTMLInputElement).checked })} style="width:auto;" />
          <span class="small dim">keep notification text minimal (no private details)</span>
        </label>
      </div>
      <div class="col" style="gap:6px;">
        {#each rules as rule (rule.id)}
          <div class="rule-row">
            <label class="row" style="gap:8px; cursor:pointer;">
              <input type="checkbox" checked={rule.enabled === 1} onchange={() => toggleRule(rule)} style="width:auto;" />
              <span>{ruleLabels[rule.kind] ?? rule.label}</span>
            </label>
            <div class="grow"></div>
            {#if rule.snoozed_until}<span class="pill">snoozed</span>{/if}
            <input type="time" value={rule.time_of_day ?? "20:00"} disabled={rule.enabled !== 1}
              onchange={(e) => updateRuleTime(rule, (e.target as HTMLInputElement).value)} style="width:96px;" />
            <button class="btn ghost sm" title="snooze 1 hour" disabled={rule.enabled !== 1}
              onclick={() => api("reminders.snooze", { id: rule.id, minutes: 60 }).then(bump)}>zᶻ</button>
          </div>
        {/each}
      </div>
      <p class="small faint" style="margin-top:10px;">
        Missed or dismissed reminders never create a failure state, and nothing stacks up after downtime.
      </p>
    </section>

    <!-- ============ data ============ -->
    <section class="card pad">
      <h2 class="sec-title display">Data, backup &amp; export</h2>
      <div class="inset" style="padding:10px 14px; margin-bottom:12px;">
        <div class="overline">Local database (source of truth)</div>
        <div class="row" style="margin-top:4px;">
          <code class="small mono grow" style="word-break:break-all;">{dbPath || "—"}</code>
          {#if dbPath}
            <button class="btn ghost sm" onclick={() => revealItemInDir(dbPath).catch(reportError)}>open location</button>
          {/if}
        </div>
        <p class="small faint" style="margin-top:6px;">
          Everything stays on this machine. Nothing leaves it unless you export or back up to a destination you choose.
        </p>
      </div>

      <div class="row wrap" style="gap:10px; margin-bottom:12px;">
        <button class="btn primary" onclick={createBackup} disabled={busy}>Create backup now</button>
        <button class="btn" onclick={async () => { const d = await chooseBackupDir(); if (d) { await set({ backup_dir: d }); toast("Backup destination set", "ok"); } }}>
          {app.settings.backup_dir ? "Change destination" : "Set destination"}
        </button>
        <button class="btn" onclick={exportData} disabled={busy}>Export CSV + SQLite…</button>
        <button class="btn" onclick={restore} disabled={busy}>Restore from backup…</button>
      </div>
      {#if app.settings.backup_dir}
        <p class="small faint">Destination: <span class="mono">{app.settings.backup_dir}</span>
          {#if app.settings.last_backup_at}· last backup {fmtInstant(app.settings.last_backup_at)}{/if}</p>
      {:else}
        <p class="small" style="color:var(--caution);">No backup destination configured yet.</p>
      {/if}

      {#if backups.length > 0}
        <div class="col" style="gap:4px; margin-top:10px;">
          {#each backups.slice(0, 5) as b}
            <div class="row small">
              <span class="mono dim">{fmtInstant(b.created_at)}</span>
              <span class="mono faint grow" style="overflow:hidden; text-overflow:ellipsis; white-space:nowrap;">{b.path}</span>
              <span class="pill" class:ok={b.file_exists} class:invalid={!b.file_exists}>{b.file_exists ? "present" : "missing"}</span>
            </div>
          {/each}
        </div>
      {/if}

      <div class="danger-zone">
        <div class="row between">
          <div>
            <p style="color:var(--invalid);">Permanently delete all data</p>
            <p class="small faint">Removes the local database entirely. Backups you created elsewhere are untouched.</p>
          </div>
          <button class="btn danger" onclick={() => (confirmPurge = true)}>Delete…</button>
        </div>
      </div>
    </section>
  </div>
</div>

{#if templateEditor}
  <Modal title={templateEditor.id ? "Edit template" : "New activity template"}
    subtitle="Renaming or archiving never touches historical entries."
    onclose={() => (templateEditor = null)} width="480px">
    <div class="col" style="gap:12px;">
      <label class="field"><span>Name</span><input bind:value={tName} /></label>
      <label class="field"><span>Subtypes (comma-separated)</span>
        <input bind:value={tSubtypes} placeholder="e.g. form, standing, class" /></label>
      <div class="row" style="gap:18px;">
        <label class="row" style="gap:8px; cursor:pointer;">
          <input type="checkbox" bind:checked={tIntensity} style="width:auto;" /><span class="small">intensity field</span></label>
        <label class="row" style="gap:8px; cursor:pointer;">
          <input type="checkbox" bind:checked={tBody} style="width:auto;" /><span class="small">body state before/after</span></label>
      </div>
    </div>
    {#snippet footer()}
      <button class="btn" onclick={() => (templateEditor = null)}>Cancel</button>
      <button class="btn primary" onclick={saveTemplate} disabled={!tName.trim()}>Save</button>
    {/snippet}
  </Modal>
{/if}

{#if confirmPurge}
  <Modal title="Delete everything?" subtitle="This cannot be undone. Consider creating a backup first."
    onclose={() => (confirmPurge = false)} width="460px">
    <div class="col" style="gap:12px;">
      <p class="small dim">
        All practices, check-ins, reflections, determinations, plans, assessments (including raw trials),
        protocols, and results will be permanently removed from this machine.
      </p>
      <label class="field"><span>Type <b>delete everything</b> to confirm</span>
        <input bind:value={purgeText} placeholder="delete everything" /></label>
    </div>
    {#snippet footer()}
      <button class="btn" onclick={() => (confirmPurge = false)}>Keep my data</button>
      <button class="btn danger" onclick={purge} disabled={purgeText !== "delete everything" || busy}>Delete permanently</button>
    {/snippet}
  </Modal>
{/if}

<style>
  .page { padding: 28px 32px 60px; max-width: 860px; margin: 0 auto; }
  .page-head { margin-bottom: 18px; }
  h1 { font-size: 27px; }
  .sections { display: flex; flex-direction: column; gap: 14px; }
  .sec-title { font-size: 17px; margin-bottom: 12px; }
  .dim-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(200px, 1fr)); gap: 8px; }
  .dim-chip {
    display: flex; flex-direction: column; align-items: flex-start; gap: 2px;
    padding: 9px 12px; border-radius: var(--r-md); text-align: left;
    background: var(--ink-1); border: 1px solid var(--line); color: var(--paper-dim);
  }
  .dim-chip.on { border-color: var(--cinnabar-dim); color: var(--paper); background: var(--cinnabar-wash); }
  .tpl-row { display: flex; align-items: center; gap: 8px; padding: 7px 10px; border-radius: var(--r-sm); }
  .tpl-row:hover { background: var(--ink-3); }
  .tpl-row.archived { opacity: 0.55; }
  .e-glyph { color: var(--cat, var(--paper-faint)); width: 18px; text-align: center; }
  .rule-row { display: flex; align-items: center; gap: 10px; padding: 5px 0; }
  .danger-zone {
    margin-top: 18px; padding-top: 14px;
    border-top: 1px solid var(--line-soft);
  }
</style>
