<script lang="ts">
  import Modal from "../components/Modal.svelte";
  import { api } from "../api";
  import { toast, reportError, bump } from "../state.svelte";
  import { CONTEXT_KINDS, todayStr } from "../format";

  let { existing = null, onclose }: { existing?: any | null; onclose: () => void } = $props();

  // This modal is mounted fresh per open, so fields deliberately capture the
  // initial prop value; later prop mutation is not a supported flow here.
  // svelte-ignore state_referenced_locally
  const init = existing;

  let kind = $state(init?.kind ?? "illness");
  let label = $state(init?.label ?? "");
  let startDate = $state(init?.start_date ?? todayStr());
  let endDate = $state(init?.end_date ?? "");
  let note = $state(init?.note ?? "");
  let ongoing = $state(init ? !init.end_date : true);
  let saving = $state(false);

  async function save() {
    saving = true;
    try {
      await api("context.save", {
        id: existing?.id ?? null,
        kind,
        label: label || CONTEXT_KINDS.find(([k]) => k === kind)?.[1] || kind,
        start_date: startDate,
        end_date: ongoing ? null : endDate || null,
        note: note || null,
      });
      bump();
      toast("Context noted", "ok");
      onclose();
    } catch (e) { reportError(e); }
    saving = false;
  }

  async function remove() {
    try {
      await api("context.delete", { id: existing.id });
      bump();
      onclose();
    } catch (e) { reportError(e); }
  }
</script>

<Modal title={existing ? "Edit context" : "Context event"}
  subtitle="Life circumstances that help interpret later patterns."
  onclose={onclose} width="480px">
  <div class="col" style="gap:12px;">
    <div class="row wrap">
      {#each CONTEXT_KINDS as [k, l]}
        <button class="pill" class:accent={kind === k} style="cursor:pointer;" onclick={() => (kind = k)}>{l}</button>
      {/each}
    </div>
    <label class="field"><span>Label</span>
      <input bind:value={label} placeholder="short description" /></label>
    <div class="row">
      <label class="field"><span>From</span><input type="date" bind:value={startDate} /></label>
      {#if !ongoing}
        <label class="field"><span>Until</span><input type="date" bind:value={endDate} min={startDate} /></label>
      {/if}
      <label class="field"><span>&nbsp;</span>
        <button class="btn sm" onclick={() => (ongoing = !ongoing)}>
          {ongoing ? "ongoing / single day" : "has an end date"}
        </button>
      </label>
    </div>
    <label class="field"><span>Note</span><textarea bind:value={note} placeholder="optional"></textarea></label>
  </div>
  {#snippet footer()}
    {#if existing}
      <button class="btn danger" onclick={remove}>Delete</button>
    {/if}
    <div class="grow"></div>
    <button class="btn" onclick={onclose}>Cancel</button>
    <button class="btn primary" onclick={save} disabled={saving}>Save</button>
  {/snippet}
</Modal>
