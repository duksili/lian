<script lang="ts">
  import Modal from "../components/Modal.svelte";
  import { api } from "../api";
  import { toast, reportError, bump } from "../state.svelte";

  let { onclose }: { onclose: (finished: boolean) => void } = $props();

  interface StanceAttempt { side: "left" | "right"; duration_s: string; touchdowns: string; }
  let attempts = $state<StanceAttempt[]>([
    { side: "left", duration_s: "", touchdowns: "0" },
    { side: "right", duration_s: "", touchdowns: "0" },
  ]);
  let capSeconds = $state("60");
  let stsTime = $state("");
  let stsConfirmed = $state(false);
  let painOrConcern = $state(false);
  let painNote = $state("");
  let conditionNote = $state("");
  let saving = $state(false);

  function addAttempt(side: "left" | "right") {
    attempts.push({ side, duration_s: "", touchdowns: "0" });
  }

  async function save() {
    saving = true;
    try {
      const started = await api("assessments.start", {
        kind: "physical_weekly_v1",
        input_method: "manual_entry",
        pre_test: { condition_note: conditionNote || null },
      });
      const trials: any[] = [];
      let idx = 0;
      for (const a of attempts) {
        if (a.duration_s.trim() === "") continue;
        trials.push({
          trial_index: idx++,
          stimulus_kind: "single_leg_stance",
          payload: {
            side: a.side,
            duration_seconds: Number(a.duration_s),
            cap_seconds: Number(capSeconds),
            touchdowns: Number(a.touchdowns || 0),
            reached_cap: Number(a.duration_s) >= Number(capSeconds),
          },
        });
      }
      if (stsTime.trim() !== "") {
        trials.push({
          trial_index: idx++,
          stimulus_kind: "sit_to_stand",
          payload: {
            total_seconds: Number(stsTime),
            start_finish_confirmed: stsConfirmed,
            pain_or_balance_concern: painOrConcern,
            concern_note: painOrConcern ? painNote || null : null,
          },
        });
      }
      if (trials.length === 0) {
        toast("Enter at least one attempt", "error");
        saving = false;
        return;
      }
      await api("assessments.finalize", {
        session_id: started.session.id,
        trials,
        context: {},
        note: conditionNote || null,
      });
      bump();
      toast("Physical check recorded", "ok");
      onclose(true);
    } catch (e) { reportError(e); }
    saving = false;
  }
</script>

<Modal title="Weekly physical check" subtitle="Protocol physical-1.0 · manual entry with a stopwatch of your choice."
  onclose={() => onclose(false)} width="620px">
  <div class="col" style="gap:16px;">
    <p class="safety small">
      Stop immediately if you feel pain, dizziness, instability, or any concern.
      This records observations only — it does not assess fall risk or health.
    </p>

    <section>
      <div class="overline" style="margin-bottom:8px;">Single-leg stance</div>
      <p class="small faint" style="margin-bottom:10px;">
        Stand near support. Time each attempt until the raised foot touches down or the cap is reached.
      </p>
      <div class="row" style="margin-bottom:8px;">
        <label class="field"><span>Cap (seconds)</span>
          <input type="number" min="10" bind:value={capSeconds} style="width:90px;" /></label>
      </div>
      <div class="col" style="gap:6px;">
        {#each attempts as a, i}
          <div class="row attempt">
            <span class="pill">{a.side}</span>
            <label class="field"><span>held (s)</span>
              <input type="number" min="0" step="0.1" bind:value={a.duration_s} placeholder="—" style="width:90px;" /></label>
            <label class="field"><span>touchdowns</span>
              <input type="number" min="0" bind:value={a.touchdowns} style="width:80px;" /></label>
            {#if i >= 2}
              <button class="btn ghost sm" onclick={() => attempts.splice(i, 1)}>✕</button>
            {/if}
          </div>
        {/each}
      </div>
      <div class="row" style="margin-top:8px;">
        <button class="btn sm" onclick={() => addAttempt("left")}>+ left attempt</button>
        <button class="btn sm" onclick={() => addAttempt("right")}>+ right attempt</button>
      </div>
    </section>

    <section>
      <div class="overline" style="margin-bottom:8px;">Five-times sit-to-stand</div>
      <p class="small faint" style="margin-bottom:10px;">
        From sitting, stand fully and sit back down five times as quickly as is comfortable. Time the whole sequence.
      </p>
      <div class="row wrap">
        <label class="field"><span>Total time (s)</span>
          <input type="number" min="0" step="0.1" bind:value={stsTime} placeholder="—" style="width:100px;" /></label>
        <label class="row" style="gap:8px; cursor:pointer; align-self:flex-end; padding-bottom:8px;">
          <input type="checkbox" bind:checked={stsConfirmed} style="width:auto;" />
          <span class="small dim">clean start &amp; finish</span>
        </label>
        <label class="row" style="gap:8px; cursor:pointer; align-self:flex-end; padding-bottom:8px;">
          <input type="checkbox" bind:checked={painOrConcern} style="width:auto;" />
          <span class="small dim">pain or balance concern occurred</span>
        </label>
      </div>
      {#if painOrConcern}
        <input bind:value={painNote} placeholder="what happened (optional)" style="margin-top:8px; width:100%;" />
      {/if}
    </section>

    <label class="field"><span>Conditions (chair, footwear, surface…)</span>
      <input bind:value={conditionNote} placeholder="optional" /></label>
  </div>
  {#snippet footer()}
    <button class="btn" onclick={() => onclose(false)}>Cancel</button>
    <button class="btn primary" onclick={save} disabled={saving}>Record session</button>
  {/snippet}
</Modal>

<style>
  .safety {
    background: var(--caution-wash); color: var(--caution);
    padding: 10px 14px; border-radius: var(--r-md);
  }
  .attempt { align-items: flex-end; }
</style>
