<script lang="ts">
  import { api } from "../api";
  import { dataVersion, toast, reportError, bump } from "../state.svelte";
  import { fmtDate, todayStr, fmtInstant, addDays } from "../format";
  import Modal from "../components/Modal.svelte";
  import EmptyState from "../components/EmptyState.svelte";

  let determinations = $state<any[]>([]);
  let includeClosed = $state(false);
  let editorOpen = $state(false);
  let editing = $state<any | null>(null);
  let detail = $state<any | null>(null);
  let reviewing = $state<any | null>(null);
  let superseding = $state(false);

  // editor fields
  let title = $state("");
  let description = $state("");
  let startedOn = $state(todayStr());
  let endsOn = $state("");
  let cadence = $state("");
  let saving = $state(false);

  // review fields
  let reviewStatus = $state("kept");
  let reviewNote = $state("");

  // linking
  let linking = $state(false);
  let linkCandidates = $state<{ type: string; id: string; label: string }[]>([]);

  async function openLinker() {
    try {
      const from = addDays(todayStr(), -14);
      const to = addDays(todayStr(), 14);
      const [events, checkins, plans] = await Promise.all([
        api("events.list", { from, to: todayStr() }),
        api("checkins.list", { from, to: todayStr() }),
        api("plans.list", { from: todayStr(), to }),
      ]);
      const existing = new Set((detail.links ?? []).map((l: any) => `${l.linked_type}:${l.linked_id}`));
      linkCandidates = [
        ...events.map((e: any) => ({ type: "activity_event", id: e.id, label: `${e.template_name} · ${fmtDate(e.local_date)}` })),
        ...checkins.map((c: any) => ({ type: "checkin", id: c.id, label: `Check-in · ${fmtDate(c.local_date)}` })),
        ...plans.map((p: any) => ({ type: "plan", id: p.id, label: `Plan: ${p.title} · ${fmtDate(p.local_date)}` })),
      ].filter((c) => !existing.has(`${c.type}:${c.id}`));
      linking = true;
    } catch (e) { reportError(e); }
  }

  async function addLink(c: { type: string; id: string }) {
    try {
      await api("determinations.add_link", {
        determination_id: detail.id, linked_type: c.type, linked_id: c.id,
      });
      linking = false;
      detail = await api("determinations.get", { id: detail.id });
      bump();
    } catch (e) { reportError(e); }
  }

  async function removeLink(l: any) {
    try {
      await api("determinations.remove_link", { link_id: l.id });
      detail = await api("determinations.get", { id: detail.id });
      bump();
    } catch (e) { reportError(e); }
  }

  $effect(() => {
    dataVersion.n; includeClosed;
    load();
  });

  async function load() {
    try {
      determinations = await api("determinations.list", { include_closed: includeClosed });
      if (detail) detail = await api("determinations.get", { id: detail.id });
    } catch (e) { reportError(e); }
  }

  function openEditor(d: any | null, supersede = false) {
    editing = d; superseding = supersede;
    title = supersede ? d?.title ?? "" : d?.title ?? "";
    description = d?.description ?? "";
    startedOn = supersede ? todayStr() : d?.started_on ?? todayStr();
    endsOn = d?.ends_on ?? "";
    cadence = d?.review_cadence ?? "";
    editorOpen = true;
  }

  async function save() {
    saving = true;
    try {
      const payload = {
        title, description: description || null,
        started_on: startedOn, ends_on: endsOn || null,
        review_cadence: cadence || null,
      };
      if (superseding && editing) {
        await api("determinations.supersede", { id: editing.id, replacement: payload });
        toast("Superseded — the earlier wording is preserved", "ok");
      } else {
        await api("determinations.save", { ...payload, id: editing?.id ?? null });
        toast(editing ? "Updated — prior wording kept in history" : "Determination made", "ok");
      }
      editorOpen = false; bump();
    } catch (e) { reportError(e); }
    saving = false;
  }

  async function setLifecycle(d: any, state: string) {
    try {
      await api("determinations.set_lifecycle", { id: d.id, state });
      bump();
    } catch (e) { reportError(e); }
  }

  async function openDetail(d: any) {
    try {
      detail = await api("determinations.get", { id: d.id });
    } catch (e) { reportError(e); }
  }

  async function saveReview() {
    try {
      await api("determinations.review", {
        determination_id: reviewing.id,
        local_date: todayStr(),
        status: reviewStatus,
        note: reviewNote || null,
      });
      reviewing = null; reviewNote = "";
      bump();
      toast("Review recorded privately", "ok");
    } catch (e) { reportError(e); }
  }

  const stateLabel: Record<string, string> = {
    active: "active", paused: "paused", completed: "completed",
    discontinued: "discontinued", superseded: "superseded",
  };
</script>

<div class="page">
  <header class="page-head">
    <div>
      <div class="overline">Voluntary commitments</div>
      <h1 class="display">Determinations</h1>
    </div>
    <div class="row">
      <label class="row small dim" style="gap:6px; cursor:pointer;">
        <input type="checkbox" bind:checked={includeClosed} style="width:auto;" /> show closed
      </label>
      <button class="btn primary" onclick={() => openEditor(null)}>+ Determination</button>
    </div>
  </header>

  <p class="small faint intro">
    A determination is an intention you hold — distinct from a scheduled plan and from the Five Precepts.
    Nothing here is scored, streaked, or judged. Reviews exist only if you give a determination a review rhythm.
  </p>

  {#if determinations.length === 0}
    <EmptyState glyph="❖" title="No determinations yet"
      body="When you decide to hold an intention over time — a practice emphasis, a restraint, a commitment — record it here and revisit it on your own terms.">
      <button class="btn primary" onclick={() => openEditor(null)}>Make one</button>
    </EmptyState>
  {:else}
    <div class="det-grid">
      {#each determinations as d (d.id)}
        <button class="card pad det-card" class:muted={!["active","paused"].includes(d.lifecycle_state)}
          onclick={() => openDetail(d)}>
          <div class="row between">
            <span class="det-mark" class:pausedm={d.lifecycle_state === "paused"}>❖</span>
            <span class="pill" class:ok={d.lifecycle_state === "active"}>{stateLabel[d.lifecycle_state]}</span>
          </div>
          <p class="det-title display">{d.title}</p>
          <p class="small faint">
            since {fmtDate(d.started_on)}{d.ends_on ? ` · until ${fmtDate(d.ends_on)}` : " · open-ended"}
            {#if d.review_cadence}· reviewed {d.review_cadence}{/if}
          </p>
          {#if d.last_review}
            <p class="small dim">last review: {d.last_review.status.replace("_", " ")} · {fmtDate(d.last_review.local_date)}</p>
          {/if}
        </button>
      {/each}
    </div>
  {/if}
</div>

{#if detail}
  <Modal title={detail.title} subtitle="Private record — links are context, never proof." onclose={() => (detail = null)} width="640px">
    <div class="col" style="gap:14px;">
      {#if detail.description}<p class="dim">{detail.description}</p>{/if}
      <div class="row wrap">
        <span class="pill" class:ok={detail.lifecycle_state === "active"}>{stateLabel[detail.lifecycle_state]}</span>
        <span class="pill">since {fmtDate(detail.started_on)}</span>
        {#if detail.ends_on}<span class="pill">until {fmtDate(detail.ends_on)}</span>{/if}
        {#if detail.review_cadence}<span class="pill">review: {detail.review_cadence}</span>{/if}
      </div>

      <div class="row wrap">
        {#if detail.lifecycle_state === "active"}
          {#if detail.review_cadence}
            <button class="btn sm primary" onclick={() => { reviewing = detail; }}>Review now</button>
          {/if}
          <button class="btn sm" onclick={() => openEditor(detail)}>Revise wording</button>
          <button class="btn sm" onclick={() => setLifecycle(detail, "paused")}>Pause</button>
          <button class="btn sm" onclick={() => setLifecycle(detail, "completed")}>Complete</button>
          <button class="btn sm" onclick={() => openEditor(detail, true)}>Supersede</button>
          <button class="btn sm" onclick={() => setLifecycle(detail, "discontinued")}>Discontinue</button>
        {:else if detail.lifecycle_state === "paused"}
          <button class="btn sm primary" onclick={() => setLifecycle(detail, "active")}>Resume</button>
          <button class="btn sm" onclick={() => setLifecycle(detail, "discontinued")}>Discontinue</button>
        {:else}
          <button class="btn sm" onclick={() => setLifecycle(detail, "active")}>Reactivate</button>
        {/if}
      </div>

      {#if detail.reviews?.length > 0}
        <section>
          <div class="overline" style="margin-bottom:6px;">Reviews</div>
          <div class="col" style="gap:4px;">
            {#each detail.reviews as r}
              <div class="row hist-row">
                <span class="pill" class:ok={r.status === "kept"} class:caution={r.status === "uncertain"}>{r.status.replace("_", " ")}</span>
                <span class="small dim">{fmtDate(r.local_date)}</span>
                {#if r.note}<span class="small faint grow">{r.note}</span>{/if}
              </div>
            {/each}
          </div>
        </section>
      {/if}

      <section>
        <div class="row between" style="margin-bottom:6px;">
          <span class="overline">Linked records</span>
          <button class="btn ghost sm" onclick={openLinker}>+ link</button>
        </div>
        {#if (detail.links ?? []).length === 0 && !linking}
          <p class="small faint">Nothing linked. Links are context for your own review — they never prove or disprove anything.</p>
        {:else}
          <div class="col" style="gap:4px;">
            {#each detail.links as l (l.id)}
              <div class="row hist-row">
                <span class="pill">{l.linked_type.replace("_", " ")}</span>
                <span class="small faint mono grow" style="overflow:hidden; text-overflow:ellipsis;">{l.linked_id.slice(0, 8)}…</span>
                <button class="btn ghost sm" onclick={() => removeLink(l)}>unlink</button>
              </div>
            {/each}
          </div>
        {/if}
        {#if linking}
          <div class="inset" style="margin-top:8px; padding:8px; max-height:200px; overflow-y:auto;">
            {#if linkCandidates.length === 0}
              <p class="small faint" style="padding:6px;">No recent records to link.</p>
            {/if}
            {#each linkCandidates as c (c.type + c.id)}
              <button class="link-cand" onclick={() => addLink(c)}>
                <span class="pill">{c.type.replace("_", " ")}</span>
                <span class="small">{c.label}</span>
              </button>
            {/each}
          </div>
        {/if}
      </section>

      {#if detail.revisions?.length > 0}
        <section>
          <div class="overline" style="margin-bottom:6px;">Earlier wording</div>
          <div class="col" style="gap:4px;">
            {#each detail.revisions as r}
              <div class="hist-row">
                <p class="small dim">“{r.prior_title}”</p>
                <p class="small faint">until {fmtInstant(r.revised_at)}</p>
              </div>
            {/each}
          </div>
        </section>
      {/if}
    </div>
  </Modal>
{/if}

{#if reviewing}
  <Modal title="Private review" subtitle={reviewing.title} onclose={() => (reviewing = null)} width="460px">
    <div class="col" style="gap:12px;">
      <div class="row wrap">
        {#each [["kept", "kept"], ["not_kept", "not kept"], ["uncertain", "uncertain"], ["not_reviewed", "prefer not to review"]] as [k, label]}
          <button class="pill" class:accent={reviewStatus === k} style="cursor:pointer;"
            onclick={() => (reviewStatus = k)}>{label}</button>
        {/each}
      </div>
      <label class="field"><span>Note (private)</span>
        <textarea bind:value={reviewNote} placeholder="optional"></textarea></label>
      <p class="small faint">Whatever you record here stays a reflection — it is never turned into a score.</p>
    </div>
    {#snippet footer()}
      <button class="btn" onclick={() => (reviewing = null)}>Cancel</button>
      <button class="btn primary" onclick={saveReview}>Record</button>
    {/snippet}
  </Modal>
{/if}

{#if editorOpen}
  <Modal title={superseding ? "Supersede determination" : editing ? "Revise determination" : "New determination"}
    subtitle={superseding ? "The current determination is closed and preserved; this becomes its successor." :
      editing ? "The previous wording is kept in history." :
      "Voluntary, private, and yours to change."}
    onclose={() => (editorOpen = false)} width="520px">
    <div class="col" style="gap:12px;">
      <label class="field"><span>Wording</span>
        <input bind:value={title} placeholder="e.g. For one month, I practice standing before breakfast" /></label>
      <label class="field"><span>Why (optional, private)</span>
        <textarea bind:value={description}></textarea></label>
      <div class="row wrap">
        <label class="field"><span>Starts</span><input type="date" bind:value={startedOn} /></label>
        <label class="field"><span>Ends (optional)</span><input type="date" bind:value={endsOn} min={startedOn} /></label>
        <label class="field"><span>Review rhythm</span>
          <select bind:value={cadence}>
            <option value="">none — narrative only</option>
            <option value="daily">daily</option>
            <option value="weekly">weekly</option>
            <option value="monthly">monthly</option>
          </select>
        </label>
      </div>
      {#if !cadence}
        <p class="small faint">Without a review rhythm this stays a narrative commitment — no review prompts, no statuses.</p>
      {/if}
    </div>
    {#snippet footer()}
      <button class="btn" onclick={() => (editorOpen = false)}>Cancel</button>
      <button class="btn primary" onclick={save} disabled={saving || !title.trim()}>
        {superseding ? "Supersede" : editing ? "Save revision" : "Make determination"}
      </button>
    {/snippet}
  </Modal>
{/if}

<style>
  .page { padding: 28px 32px 48px; max-width: 1000px; margin: 0 auto; }
  .page-head { display: flex; justify-content: space-between; align-items: flex-end; margin-bottom: 10px; flex-wrap: wrap; gap: 12px; }
  h1 { font-size: 27px; }
  .intro { max-width: 640px; margin-bottom: 20px; }
  .det-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 12px; }
  .det-card { text-align: left; display: flex; flex-direction: column; gap: 8px; transition: border-color 120ms ease; }
  .det-card:hover { border-color: var(--ink-4); }
  .det-card.muted { opacity: 0.6; }
  .det-mark { color: var(--cinnabar); }
  .det-mark.pausedm { color: var(--paper-ghost); }
  .det-title { font-size: 16.5px; line-height: 1.4; }
  .hist-row { padding: 6px 8px; border-radius: var(--r-xs); background: var(--ink-1); }
  .link-cand {
    display: flex; align-items: center; gap: 8px; width: 100%;
    padding: 6px 8px; border-radius: var(--r-xs); text-align: left; color: var(--paper);
  }
  .link-cand:hover { background: var(--ink-3); }
</style>
