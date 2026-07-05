<script lang="ts">
  import { api } from "../api";
  import { reportError, bump } from "../state.svelte";
  import { onMount } from "svelte";
  import { GngMachine, type GngAction } from "../assessment/gngMachine";

  let { onclose }: { onclose: (finished: boolean) => void } = $props();

  type UiPhase = "prep" | "countdown" | "isi" | "stimulus" | "blank" | "done" | "confirm_abort";
  let phase = $state<UiPhase>("prep");

  const RESPONSE_WINDOW_MS = 1000; // fixed for protocol gng-1.0
  const STIMULUS_MS = 600;

  let familiarization = $state(false);
  let session = $state<any | null>(null);
  let machine: GngMachine | null = null;
  let totalTrials = $state(160);
  let trialIndex = $state(0);
  let currentStimulus = $state<"go" | "no_go">("go");
  let visibilityLost = 0;
  let result = $state<any | null>(null);
  let feedbackText = $state("");
  let spaceDown = false;
  let heldAtOnset = false;

  let timeouts: ReturnType<typeof setTimeout>[] = [];
  function later(fn: () => void, ms: number) {
    timeouts.push(setTimeout(fn, ms));
  }
  function clearAll() {
    for (const t of timeouts) clearTimeout(t);
    timeouts = [];
  }

  function onVisibility() {
    if (document.hidden && machine) {
      visibilityLost++;
      machine.markVisibilityLost();
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
        input_method: "keyboard_spacebar",
        device_metadata: {
          platform: navigator.platform,
          screen: `${screen.width}x${screen.height}`,
          device_pixel_ratio: window.devicePixelRatio,
        },
        is_familiarization: familiarization,
      });
      session = started.session;
      const seq = started.sequence.trials as { stimulus: "go" | "no_go"; isi_ms: number }[];
      totalTrials = seq.length;
      machine = new GngMachine({
        stimulusMs: STIMULUS_MS,
        responseWindowMs: RESPONSE_WINDOW_MS,
        trials: seq,
      });
      visibilityLost = 0;
      phase = "countdown";
      let count = 3;
      feedbackText = String(count);
      const tick = () => {
        count--;
        if (count > 0) {
          feedbackText = String(count);
          later(tick, 1000);
        } else {
          exec(machine!.begin(performance.now()));
        }
      };
      later(tick, 1000);
    } catch (e) {
      reportError(e);
      onclose(false);
    }
  }

  /** Execute a machine action with real timers; the machine owns the protocol. */
  function exec(action: GngAction) {
    if (!machine) return;
    trialIndex = machine.trialIndex;
    switch (action.kind) {
      case "isi":
        phase = "isi";
        later(() => {
          heldAtOnset = spaceDown; // key already down when the stimulus appears
          exec(machine!.stimulusDue(performance.now()));
        }, action.ms);
        break;
      case "stimulus":
        currentStimulus = action.stimulus;
        phase = "stimulus";
        // Stimulus hides after visibleMs; the response window runs to windowMs.
        later(() => {
          if (phase === "stimulus") phase = "blank";
        }, action.visibleMs);
        later(() => {
          exec(machine!.windowClosed(performance.now()));
        }, action.windowMs);
        break;
      case "finished":
        finish(false);
        break;
    }
  }

  function respond(repeat: boolean) {
    if (!machine) return;
    const action = machine.response(performance.now(), { repeat });
    if (action) {
      clearAll();
      exec(action);
    }
  }

  async function finish(abortedEarly: boolean) {
    if (!machine) return;
    phase = "done";
    clearAll();
    try {
      result = await api("assessments.finalize", {
        session_id: session.id,
        trials: machine.trials,
        context: {
          visibility_lost_count: visibilityLost,
          key_hold_events: machine.keyHoldEvents,
          input_method: "keyboard_spacebar",
          aborted: abortedEarly,
        },
      });
      bump();
    } catch (e) {
      reportError(e);
    }
  }

  async function abandonWithoutData() {
    try {
      await api("assessments.abort", { session_id: session.id, reason: "user cancelled before completion" });
      bump();
    } catch (e) {
      reportError(e);
    }
    onclose(false);
  }

  function onkeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      if (["isi", "stimulus", "blank"].includes(phase)) {
        clearAll();
        phase = "confirm_abort";
      } else if (phase === "prep") onclose(false);
      return;
    }
    if (e.code === "Space") {
      e.preventDefault();
      // A held key produces repeats; a key held since before onset must be
      // released before a press can count. Both are recorded as key-hold events.
      if (e.repeat || heldAtOnset) {
        machine?.response(performance.now(), { repeat: true });
      } else {
        respond(false);
      }
      spaceDown = true;
    }
  }
  function onkeyup(e: KeyboardEvent) {
    if (e.code === "Space") {
      spaceDown = false;
      heldAtOnset = false;
    }
  }
</script>

<svelte:window on:keydown={onkeydown} on:keyup={onkeyup} />

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
        <li>Press and release — a held key does not register.</li>
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
  {:else if phase === "isi" || phase === "blank"}
    <div class="fix mono faint">+</div>
    <div class="progress"><div class="bar" style:width="{(trialIndex / totalTrials) * 100}%"></div></div>
  {:else if phase === "stimulus"}
    <div class="stim big" class:go={currentStimulus === "go"} class:nogo={currentStimulus === "no_go"}></div>
    <div class="progress"><div class="bar" style:width="{(trialIndex / totalTrials) * 100}%"></div></div>
  {:else if phase === "confirm_abort"}
    <div class="prep card pad">
      <h2 class="display">Stop the test?</h2>
      <p class="small dim" style="margin: 10px 0 16px;">
        {trialIndex} of {totalTrials} trials done. Keep the partial data (marked incomplete) or discard the attempt.
      </p>
      <div class="row" style="justify-content:flex-end;">
        <button class="btn" onclick={() => exec(machine!.resume())}>Continue</button>
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
