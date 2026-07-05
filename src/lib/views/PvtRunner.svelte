<script lang="ts">
  import { api } from "../api";
  import { app, toast, reportError, bump } from "../state.svelte";
  import { onMount } from "svelte";

  let { onclose }: { onclose: (finished: boolean) => void } = $props();

  type Phase = "prep" | "countdown" | "waiting" | "stimulus" | "feedback" | "done" | "confirm_abort";
  let phase = $state<Phase>("prep");

  // pre-test
  let familiarization = $state(false);
  let caffeineToday = $state<"no" | "yes" | "">("");
  let restedFeeling = $state<"" | "rested" | "average" | "tired">("");

  let session = $state<any | null>(null);
  let intervals: number[] = [];
  let trials: any[] = [];
  let trialIndex = 0;
  let counterMs = $state(0);
  let feedbackText = $state("");
  let visibilityLost = 0;
  let currentTrialVisibilityLost = false;
  let progress = $state(0);
  let result = $state<any | null>(null);
  let sessionStart = 0;
  let stimulusOnset = 0;

  let rafId = 0;
  let timeoutId: ReturnType<typeof setTimeout> | null = null;

  function onVisibility() {
    if (document.hidden && ["waiting", "stimulus"].includes(phase)) {
      visibilityLost++;
      currentTrialVisibilityLost = true;
    }
  }

  onMount(() => {
    document.addEventListener("visibilitychange", onVisibility);
    return () => {
      document.removeEventListener("visibilitychange", onVisibility);
      if (timeoutId) clearTimeout(timeoutId);
      cancelAnimationFrame(rafId);
    };
  });

  async function begin() {
    try {
      const started = await api("assessments.start", {
        kind: "pvt_v1",
        input_method: app.settings.assessment_input_method ?? "keyboard_spacebar",
        device_metadata: {
          platform: navigator.platform,
          user_agent: navigator.userAgent,
          screen: `${screen.width}x${screen.height}`,
          device_pixel_ratio: window.devicePixelRatio,
        },
        pre_test: {
          caffeine_today: caffeineToday || null,
          rested_feeling: restedFeeling || null,
        },
        is_familiarization: familiarization,
      });
      session = started.session;
      intervals = started.sequence.intervals_ms;
      trials = [];
      trialIndex = 0;
      visibilityLost = 0;
      phase = "countdown";
      let count = 3;
      feedbackText = String(count);
      const tick = () => {
        count--;
        if (count > 0) { feedbackText = String(count); timeoutId = setTimeout(tick, 1000); }
        else {
          sessionStart = performance.now();
          nextTrial();
        }
      };
      timeoutId = setTimeout(tick, 1000);
    } catch (e) { reportError(e); onclose(false); }
  }

  function nowMs(): number { return Math.round(performance.now() - sessionStart); }

  function nextTrial() {
    if (trialIndex >= intervals.length) { finish(false); return; }
    currentTrialVisibilityLost = false;
    phase = "waiting";
    const isi = intervals[trialIndex];
    timeoutId = setTimeout(() => {
      stimulusOnset = performance.now();
      phase = "stimulus";
      counterMs = 0;
      const loop = () => {
        if (phase !== "stimulus") return;
        counterMs = Math.round(performance.now() - stimulusOnset);
        if (counterMs >= 3000) {
          // timeout — omission
          trials.push({
            trial_index: trialIndex,
            stimulus_kind: "stimulus",
            planned_interval_ms: isi,
            onset_ms: Math.round(stimulusOnset - sessionStart),
            response_ms: null,
            reaction_time_ms: null,
            visibility_lost: currentTrialVisibilityLost,
          });
          trialIndex++;
          showFeedback("·  ·  ·");
          return;
        }
        rafId = requestAnimationFrame(loop);
      };
      rafId = requestAnimationFrame(loop);
    }, isi);
  }

  function showFeedback(text: string) {
    feedbackText = text;
    phase = "feedback";
    progress = trialIndex / intervals.length;
    timeoutId = setTimeout(() => nextTrial(), 550);
  }

  function respond() {
    if (phase === "waiting") {
      // False start: response before stimulus onset.
      trials.push({
        trial_index: trialIndex,
        stimulus_kind: "stimulus",
        planned_interval_ms: intervals[trialIndex],
        onset_ms: null,
        response_ms: nowMs(),
        reaction_time_ms: null,
        is_false_start: true,
        visibility_lost: currentTrialVisibilityLost,
      });
      trialIndex++;
      if (timeoutId) clearTimeout(timeoutId);
      showFeedback("too soon");
      return;
    }
    if (phase === "stimulus") {
      const rt = Math.round(performance.now() - stimulusOnset);
      cancelAnimationFrame(rafId);
      trials.push({
        trial_index: trialIndex,
        stimulus_kind: "stimulus",
        planned_interval_ms: intervals[trialIndex],
        onset_ms: Math.round(stimulusOnset - sessionStart),
        response_ms: nowMs(),
        reaction_time_ms: rt,
        visibility_lost: currentTrialVisibilityLost,
      });
      trialIndex++;
      showFeedback(`${rt} ms`);
    }
  }

  async function finish(abortedEarly: boolean) {
    phase = "done";
    if (timeoutId) clearTimeout(timeoutId);
    cancelAnimationFrame(rafId);
    try {
      result = await api("assessments.finalize", {
        session_id: session.id,
        trials,
        context: {
          visibility_lost_count: visibilityLost,
          elapsed_ms: nowMs(),
          aborted: abortedEarly,
        },
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
      if (["waiting", "stimulus", "feedback"].includes(phase)) {
        if (timeoutId) clearTimeout(timeoutId);
        cancelAnimationFrame(rafId);
        phase = "confirm_abort";
      } else if (phase === "prep") onclose(false);
      return;
    }
    if (e.code === "Space") {
      e.preventDefault();
      respond();
    }
  }
</script>

<svelte:window on:keydown={onkeydown} />

<div class="stage">
  {#if phase === "prep"}
    <div class="prep card pad">
      <div class="overline">Psychomotor Vigilance · protocol pvt-1.0</div>
      <h2 class="display" style="margin: 6px 0 14px;">Five quiet minutes</h2>
      <ul class="small dim rules">
        <li>Watch the dim ring. When the millisecond counter appears, press <kbd>space</kbd> as fast as you can.</li>
        <li>Pressing before the counter appears counts as a false start — just wait for the next one.</li>
        <li>Sit as you usually do for this test. Keep this window focused and undisturbed.</li>
        <li>Responses under 100 ms are false starts; 500 ms or slower counts as a lapse. This is normal variation, not a grade.</li>
      </ul>
      <div class="row wrap" style="margin: 16px 0;">
        <label class="field"><span>Caffeine today?</span>
          <select bind:value={caffeineToday}><option value="">—</option><option value="no">not yet</option><option value="yes">yes</option></select>
        </label>
        <label class="field"><span>Feeling</span>
          <select bind:value={restedFeeling}><option value="">—</option><option value="rested">rested</option><option value="average">average</option><option value="tired">tired</option></select>
        </label>
        <label class="row" style="gap:8px; cursor:pointer; align-self:flex-end; padding-bottom:8px;">
          <input type="checkbox" bind:checked={familiarization} style="width:auto;" />
          <span class="small dim">familiarization run (excluded from trends by default)</span>
        </label>
      </div>
      <div class="row" style="justify-content:flex-end;">
        <button class="btn" onclick={() => onclose(false)}>Not now</button>
        <button class="btn primary" onclick={begin}>Begin</button>
      </div>
    </div>
  {:else if phase === "countdown"}
    <div class="big-counter mono faint">{feedbackText}</div>
  {:else if phase === "waiting"}
    <div class="ring" aria-label="waiting for stimulus"></div>
    <p class="hint small faint">wait…</p>
  {:else if phase === "stimulus"}
    <div class="big-counter mono">{counterMs}</div>
  {:else if phase === "feedback"}
    <div class="big-counter mono dim">{feedbackText}</div>
    <div class="progress"><div class="bar" style:width="{progress * 100}%"></div></div>
  {:else if phase === "confirm_abort"}
    <div class="prep card pad">
      <h2 class="display">Stop the test?</h2>
      <p class="small dim" style="margin: 10px 0 16px;">
        Data so far can be kept (marked incomplete/invalid) or the attempt can be discarded entirely.
      </p>
      <div class="row" style="justify-content:flex-end;">
        <button class="btn" onclick={() => { phase = "waiting"; nextTrial(); }}>Continue test</button>
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
          <div><span class="faint">median RT</span> {result.derived_metrics.median_rt_ms ?? "—"} ms</div>
          <div><span class="faint">lapses (≥500 ms)</span> {result.derived_metrics.lapse_count}</div>
          <div><span class="faint">false starts</span> {result.derived_metrics.false_start_count}</div>
          <div><span class="faint">trials</span> {result.trial_count}</div>
        </div>
        {#if result.validity_state !== "valid"}
          <p class="small" style="margin-top:12px; color: var(--caution);">
            Marked “{result.validity_state}”: {result.validity_reasons.join(", ").replaceAll("_", " ")}.
            The raw data is kept either way.
          </p>
        {:else}
          <p class="small faint" style="margin-top:12px;">One observation among many — single sessions mean little on their own.</p>
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
  .big-counter {
    font-size: 88px; font-weight: 500; letter-spacing: 0.02em;
    color: var(--paper);
    font-variant-numeric: tabular-nums;
  }
  .ring {
    width: 88px; height: 88px; border-radius: 99px;
    border: 2px solid var(--ink-4);
  }
  .hint { position: absolute; bottom: 15vh; }
  .progress {
    position: absolute; bottom: 10vh; width: 200px; height: 2px;
    background: var(--ink-3); border-radius: 2px; overflow: hidden;
  }
  .bar { height: 100%; background: var(--cinnabar-dim); transition: width 300ms ease; }
  .metrics { display: grid; grid-template-columns: 1fr 1fr; gap: 8px 20px; }
</style>
