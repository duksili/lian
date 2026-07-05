<script lang="ts">
  import { api } from "../api";
  import { app, reportError, bump } from "../state.svelte";
  import { onMount } from "svelte";

  let { onclose }: { onclose: (finished: boolean) => void } = $props();

  type Phase = "prep" | "countdown" | "isi" | "stimulus" | "done" | "confirm_abort";
  let phase = $state<Phase>("prep");

  const RESPONSE_WINDOW_MS = 1000; // fixed for protocol gng-1.0
  const STIMULUS_MS = 600;

  let familiarization = $state(false);
  let session = $state<any | null>(null);
  let sequence: { stimulus: string; isi_ms: number }[] = [];
  let trials: any[] = [];
  let trialIndex = $state(0);
  let currentStimulus = $state<"go" | "no_go">("go");
  let visibilityLost = 0;
  let currentTrialVisibilityLost = false;
  let result = $state<any | null>(null);
  let feedbackText = $state("");
  let sessionStart = 0;
  let stimulusOnset = 0;
  let responded = false;

  let timeouts: ReturnType<typeof setTimeout>[] = [];
  function later(fn: () => void, ms: number) {
    timeouts.push(setTimeout(fn, ms));
  }
  function clearAll() {
    for (const t of timeouts) clearTimeout(t);
    timeouts = [];
  }

  function onVisibility() {
    if (document.hidden && ["isi", "stimulus"].includes(phase)) {
      visibilityLost++;
      currentTrialVisibilityLost = true;
    }
  }

  onMount(() => {
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      document.removeEventListener("visibilitychange", onVisibility);
      clearAll();
    };
  });

  async function begin() {
    try {
      const started = await api("assessments.start", {
        kind: "go_no_go_v1",
        input_method: app.settings.assessment_input_method ?? "keyboard_spacebar",
        device_metadata: {
          platform: navigator.platform,
          screen: `${screen.width}x${screen.height}`,
          device_pixel_ratio: window.devicePixelRatio,
        },
        is_familiarization: familiarization,
      });
      session = started.session;
      sequence = started.sequence.trials;
      trials = [];
      trialIndex = 0;
      phase = "countdown";
      let count = 3;
      feedbackText = String(count);
      const tick = () => {
        count--;
        if (count > 0) { feedbackText = String(count); later(tick, 1000); }
        else { sessionStart = performance.now(); nextTrial(); }
      };
      later(tick, 1000);
    } catch (e) { reportError(e); onclose(false); }
  }

  function nextTrial() {
    if (trialIndex >= sequence.length) { finish(false); return; }
    currentTrialVisibilityLost = false;
    responded = false;
    phase = "isi";
    const t = sequence[trialIndex];
    later(() => {
      currentStimulus = t.stimulus as "go" | "no_go";
      stimulusOnset = performance.now();
      phase = "stimulus";
      // Stimulus disappears after STIMULUS_MS but the response window runs to
      // RESPONSE_WINDOW_MS; then the trial is committed.
      later(() => { if (phase === "stimulus" && !responded) phase = "isi"; }, STIMULUS_MS);
      later(() => commitTrial(), RESPONSE_WINDOW_MS);
    }, t.isi_ms);
  }

  function commitTrial() {
    if (trialIndex >= sequence.length) return;
    const t = sequence[trialIndex];
    if (!responded) {
      trials.push({
        trial_index: trialIndex,
        stimulus_kind: t.stimulus,
        onset_ms: Math.round(stimulusOnset - sessionStart),
        response_ms: null,
        reaction_time_ms: null,
        visibility_lost: currentTrialVisibilityLost,
      });
      trialIndex++;
      nextTrial();
    }
  }

  function respond() {
    if (phase !== "stimulus" && phase !== "isi") return;
    if (responded || trialIndex >= sequence.length) return;
    // Only count as a response when a stimulus is in its window.
    const sinceOnset = performance.now() - stimulusOnset;
    if (phase === "isi" && (sinceOnset > RESPONSE_WINDOW_MS || stimulusOnset === 0)) return;
    responded = true;
    clearAll();
    const t = sequence[trialIndex];
    const rt = Math.round(sinceOnset);
    trials.push({
      trial_index: trialIndex,
      stimulus_kind: t.stimulus,
      onset_ms: Math.round(stimulusOnset - sessionStart),
      response_ms: Math.round(performance.now() - sessionStart),
      reaction_time_ms: t.stimulus === "go" ? rt : null,
      visibility_lost: currentTrialVisibilityLost,
    });
    trialIndex++;
    nextTrial();
  }

  async function finish(abortedEarly: boolean) {
    phase = "done";
    clearAll();
    try {
      result = await api("assessments.finalize", {
        session_id: session.id,
        trials,
        context: { visibility_lost_count: visibilityLost, aborted: abortedEarly },
      });
      bump();
    } catch (e) { reportError(e); }
  }

  async function abandonWithoutData() {
    try {
      await api("assessments.abort", { session_id: session.id, reason: "user cancelled before completion" });
      bump();
    } catch (e) { reportError(e); }
    onclose(false);
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (["isi", "stimulus"].includes(phase)) { clearAll(); phase = "confirm_abort"; }
      else if (phase === "prep") onclose(false);
      return;
    }
    if (e.code === "Space") { e.preventDefault(); respond(); }
  }
</script>

<svelte:window on:keydown={onkeydown} />

<div class="stage">
  {#if phase === "prep"}
    <div class="prep card pad">
      <div class="overline">Go / No-Go · protocol gng-1.0</div>
      <h2 class="display" style="margin: 6px 0 14px;">160 brief decisions</h2>
      <div class="demo row" style="gap:40px; justify-content:center; margin: 10px 0 16px;">
        <div class="col" style="align-items:center; gap:8px;">
          <div class="stim go"></div>
          <span class="small dim">filled circle → press <kbd>space</kbd></span>
        </div>
        <div class="col" style="align-items:center; gap:8px;">
          <div class="stim nogo"></div>
          <span class="small dim">open square → do nothing</span>
        </div>
      </div>
      <ul class="small dim rules">
        <li>Respond as quickly as you can to circles; hold back for squares.</li>
        <li>Each stimulus appears briefly. About four minutes total.</li>
        <li>Occasional wrong presses are part of the task, not a failure.</li>
      </ul>
      <label class="row" style="gap:8px; cursor:pointer; margin: 14px 0;">
        <input type="checkbox" bind:checked={familiarization} style="width:auto;" />
        <span class="small dim">familiarization run (excluded from trends by default)</span>
      </label>
      <div class="row" style="justify-content:flex-end;">
        <button class="btn" onclick={() => onclose(false)}>Not now</button>
        <button class="btn primary" onclick={begin}>Begin</button>
      </div>
    </div>
  {:else if phase === "countdown"}
    <div class="big-counter mono faint">{feedbackText}</div>
  {:else if phase === "isi"}
    <div class="fix mono faint">+</div>
    <div class="progress"><div class="bar" style:width="{(trialIndex / 160) * 100}%"></div></div>
  {:else if phase === "stimulus"}
    <div class="stim big" class:go={currentStimulus === "go"} class:nogo={currentStimulus === "no_go"}></div>
    <div class="progress"><div class="bar" style:width="{(trialIndex / 160) * 100}%"></div></div>
  {:else if phase === "confirm_abort"}
    <div class="prep card pad">
      <h2 class="display">Stop the test?</h2>
      <p class="small dim" style="margin: 10px 0 16px;">
        {trialIndex} of 160 trials done. Keep the partial data (marked incomplete) or discard the attempt.
      </p>
      <div class="row" style="justify-content:flex-end;">
        <button class="btn" onclick={() => nextTrial()}>Continue</button>
        <button class="btn" onclick={() => finish(true)}>Stop, keep data</button>
        <button class="btn danger" onclick={abandonWithoutData}>Discard attempt</button>
      </div>
    </div>
  {:else if phase === "done"}
    <div class="prep card pad">
      {#if result}
        <div class="overline">Complete</div>
        <h2 class="display" style="margin: 6px 0 14px;">Recorded</h2>
        <div class="metrics mono small">
          <div><span class="faint">go RT median</span> {result.derived_metrics.go_rt_median_ms ?? "—"} ms</div>
          <div><span class="faint">commissions</span> {result.derived_metrics.commission_error_count} / {result.derived_metrics.no_go_trial_count}</div>
          <div><span class="faint">omissions</span> {result.derived_metrics.omission_count}</div>
          <div><span class="faint">trials</span> {result.trial_count}</div>
        </div>
        {#if result.validity_state !== "valid"}
          <p class="small" style="margin-top:12px; color: var(--caution);">
            Marked “{result.validity_state}”: {result.validity_reasons.join(", ").replaceAll("_", " ")}. Raw data kept.
          </p>
        {:else}
          <p class="small faint" style="margin-top:12px;">Repeated tests improve with familiarity — trends matter, single runs don't.</p>
        {/if}
        <div class="row" style="justify-content:flex-end; margin-top:16px;">
          <button class="btn primary" onclick={() => onclose(true)}>Done</button>
        </div>
      {:else}
        <p class="dim">Saving…</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .prep { max-width: 520px; width: 100%; }
  .rules { padding-left: 18px; display: flex; flex-direction: column; gap: 6px; }
  .big-counter { font-size: 88px; color: var(--paper); font-variant-numeric: tabular-nums; }
  .fix { font-size: 40px; }
  .stim { width: 64px; height: 64px; }
  .stim.big { width: 110px; height: 110px; }
  .stim.go { border-radius: 99px; background: var(--paper); }
  .stim.nogo { border-radius: 8px; border: 4px solid var(--paper); background: transparent; }
  .progress {
    position: absolute; bottom: 10vh; width: 200px; height: 2px;
    background: var(--ink-3); border-radius: 2px; overflow: hidden;
  }
  .bar { height: 100%; background: var(--cinnabar-dim); }
  .metrics { display: grid; grid-template-columns: 1fr 1fr; gap: 8px 20px; }
</style>
