/**
 * Go/No-Go v1 runner state machine — pure and clock-injected.
 *
 * Protocol rules enforced here (see references/GO_NO_GO_PROTOCOL_V1.md and
 * LIAN-02): a response is accepted only during the current stimulus's
 * response window; onset state resets at each new ISI so nothing carries
 * into the next trial; OS key-repeat and keys already held at stimulus onset
 * are counted as key-hold events, never attributed to a trial.
 */

export type GngStimulus = "go" | "no_go";

export interface GngConfig {
  stimulusMs: number; // stimulus visible
  responseWindowMs: number; // responses accepted from onset up to this
  trials: { stimulus: GngStimulus; isi_ms: number }[];
}

export interface GngTrial {
  trial_index: number;
  stimulus_kind: GngStimulus;
  onset_ms: number;
  response_ms: number | null;
  reaction_time_ms: number | null;
  visibility_lost?: boolean;
}

export type GngAction =
  | { kind: "isi"; ms: number }
  | { kind: "stimulus"; stimulus: GngStimulus; visibleMs: number; windowMs: number }
  | { kind: "finished" };

export type GngPhase = "idle" | "isi" | "window" | "finished";

export class GngMachine {
  readonly trials: GngTrial[] = [];
  phase: GngPhase = "idle";
  keyHoldEvents = 0;

  private t0 = 0;
  private idx = 0;
  private onsetAt = 0;
  private visLost = false;

  constructor(private cfg: GngConfig) {}

  get trialIndex(): number {
    return this.idx;
  }

  begin(now: number): GngAction {
    this.t0 = now;
    return this.nextTrial();
  }

  private nextTrial(): GngAction {
    // Reset per-trial state so nothing leaks from the previous stimulus.
    this.onsetAt = 0;
    this.visLost = false;
    if (this.idx >= this.cfg.trials.length) {
      this.phase = "finished";
      return { kind: "finished" };
    }
    this.phase = "isi";
    return { kind: "isi", ms: this.cfg.trials[this.idx].isi_ms };
  }

  markVisibilityLost(): void {
    if (this.phase === "isi" || this.phase === "window") this.visLost = true;
  }

  stimulusDue(now: number): GngAction {
    if (this.phase !== "isi") return { kind: "finished" };
    this.onsetAt = now;
    this.phase = "window";
    const t = this.cfg.trials[this.idx];
    return {
      kind: "stimulus",
      stimulus: t.stimulus,
      visibleMs: this.cfg.stimulusMs,
      windowMs: this.cfg.responseWindowMs,
    };
  }

  /**
   * Key press. Ignored (returns null) unless the current trial's response
   * window is open. Repeats/held keys are counted, never recorded as trials.
   */
  response(now: number, opts: { repeat?: boolean; heldAtOnset?: boolean } = {}): GngAction | null {
    if (opts.repeat || opts.heldAtOnset) {
      if (this.phase === "isi" || this.phase === "window") this.keyHoldEvents++;
      return null;
    }
    if (this.phase !== "window") return null; // presses during ISI never count
    const rt = now - this.onsetAt;
    if (rt > this.cfg.responseWindowMs) return null;
    const t = this.cfg.trials[this.idx];
    this.trials.push({
      trial_index: this.idx,
      stimulus_kind: t.stimulus,
      onset_ms: this.onsetAt - this.t0,
      response_ms: now - this.t0,
      reaction_time_ms: t.stimulus === "go" ? Math.round(rt) : null,
      visibility_lost: this.visLost,
    });
    this.idx++;
    return this.nextTrial();
  }

  /**
   * Resume after a host interruption (abort dialog): the in-flight trial is
   * re-run from a fresh ISI; nothing was recorded for it.
   */
  resume(): GngAction {
    if (this.phase === "finished") return { kind: "finished" };
    return this.nextTrial();
  }

  /** The response window elapsed without a response: commit the trial. */
  windowClosed(now: number): GngAction {
    if (this.phase !== "window") return { kind: "finished" };
    const t = this.cfg.trials[this.idx];
    this.trials.push({
      trial_index: this.idx,
      stimulus_kind: t.stimulus,
      onset_ms: this.onsetAt - this.t0,
      response_ms: null,
      reaction_time_ms: null,
      visibility_lost: this.visLost,
    });
    this.idx++;
    return this.nextTrial();
  }
}
