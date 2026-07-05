/**
 * PVT v1 runner state machine — pure and clock-injected so the protocol's
 * timing rules are deterministically testable without a DOM or timers.
 *
 * The machine owns the protocol logic: interval consumption against the
 * monotonic 5-minute deadline, false starts, omissions, and trial recording.
 * The Svelte runner is only a driver: it executes the returned actions with
 * real timers and forwards key presses.
 */

export interface PvtConfig {
  durationMs: number;
  timeoutMs: number;
  feedbackMs: number;
  falseStartMs: number;
  /** Seeded interval pool from the core; consumed in order. */
  intervals: number[];
}

export interface PvtTrial {
  trial_index: number;
  stimulus_kind: "stimulus";
  planned_interval_ms: number | null;
  onset_ms: number | null;
  response_ms: number | null;
  reaction_time_ms: number | null;
  is_false_start?: boolean;
  visibility_lost?: boolean;
}

export type PvtAction =
  | { kind: "wait"; ms: number }
  | { kind: "stimulus"; timeoutMs: number }
  | { kind: "feedback"; ms: number; text: string }
  | { kind: "finished"; elapsedMs: number };

export type PvtPhase = "idle" | "waiting" | "stimulus" | "feedback" | "finished";

export class PvtMachine {
  readonly trials: PvtTrial[] = [];
  phase: PvtPhase = "idle";
  keyHoldEvents = 0;

  private t0 = 0;
  private idx = 0;
  private currentIsi = 0;
  private onsetAt = 0;
  private visLost = false;

  constructor(private cfg: PvtConfig) {}

  get startedAt(): number {
    return this.t0;
  }

  elapsed(now: number): number {
    return now - this.t0;
  }

  begin(now: number): PvtAction {
    this.t0 = now;
    return this.schedule(now);
  }

  /** Schedule the next stimulus only if its response window fits the deadline. */
  private schedule(now: number): PvtAction {
    const elapsed = now - this.t0;
    const isi = this.cfg.intervals[this.idx];
    if (isi == null || elapsed + isi + this.cfg.timeoutMs > this.cfg.durationMs) {
      this.phase = "finished";
      return { kind: "finished", elapsedMs: elapsed };
    }
    this.currentIsi = isi;
    this.visLost = false;
    this.phase = "waiting";
    return { kind: "wait", ms: isi };
  }

  markVisibilityLost(): void {
    if (this.phase === "waiting" || this.phase === "stimulus") this.visLost = true;
  }

  /** Host's ISI timer fired: the stimulus appears now. */
  stimulusDue(now: number): PvtAction {
    if (this.phase !== "waiting") return { kind: "finished", elapsedMs: this.elapsed(now) };
    this.onsetAt = now;
    this.phase = "stimulus";
    return { kind: "stimulus", timeoutMs: this.cfg.timeoutMs };
  }

  /**
   * Key press. `repeat`/`heldAtOnset` mark OS auto-repeat or a key already
   * held when the stimulus appeared: those are counted as key-hold events
   * and never attributed to a trial. Returns null when the press is ignored.
   */
  response(now: number, opts: { repeat?: boolean; heldAtOnset?: boolean } = {}): PvtAction | null {
    if (opts.repeat || opts.heldAtOnset) {
      if (this.phase === "waiting" || this.phase === "stimulus") this.keyHoldEvents++;
      return null;
    }
    if (this.phase === "waiting") {
      // False start: response before stimulus onset. Consumes the interval.
      this.trials.push({
        trial_index: this.trials.length,
        stimulus_kind: "stimulus",
        planned_interval_ms: this.currentIsi,
        onset_ms: null,
        response_ms: now - this.t0,
        reaction_time_ms: null,
        is_false_start: true,
        visibility_lost: this.visLost,
      });
      this.idx++;
      this.phase = "feedback";
      return { kind: "feedback", ms: this.cfg.feedbackMs, text: "too soon" };
    }
    if (this.phase === "stimulus") {
      const rt = now - this.onsetAt;
      this.trials.push({
        trial_index: this.trials.length,
        stimulus_kind: "stimulus",
        planned_interval_ms: this.currentIsi,
        onset_ms: this.onsetAt - this.t0,
        response_ms: now - this.t0,
        reaction_time_ms: rt,
        visibility_lost: this.visLost,
      });
      this.idx++;
      this.phase = "feedback";
      const text = rt < this.cfg.falseStartMs ? "too soon" : `${Math.round(rt)} ms`;
      return { kind: "feedback", ms: this.cfg.feedbackMs, text };
    }
    return null;
  }

  /** No response within the timeout: omission. */
  stimulusTimeout(now: number): PvtAction {
    if (this.phase !== "stimulus") return { kind: "finished", elapsedMs: this.elapsed(now) };
    this.trials.push({
      trial_index: this.trials.length,
      stimulus_kind: "stimulus",
      planned_interval_ms: this.currentIsi,
      onset_ms: this.onsetAt - this.t0,
      response_ms: null,
      reaction_time_ms: null,
      visibility_lost: this.visLost,
    });
    this.idx++;
    this.phase = "feedback";
    return { kind: "feedback", ms: this.cfg.feedbackMs, text: "·  ·  ·" };
  }

  feedbackDone(now: number): PvtAction {
    if (this.phase !== "feedback") return { kind: "finished", elapsedMs: this.elapsed(now) };
    return this.schedule(now);
  }

  /**
   * Resume after the host interrupted the flow (e.g. an abort dialog): the
   * in-flight stimulus/interval is discarded without recording a trial and a
   * fresh interval is scheduled against the unchanged deadline.
   */
  resume(now: number): PvtAction {
    if (this.phase === "finished") return { kind: "finished", elapsedMs: this.elapsed(now) };
    return this.schedule(now);
  }
}
