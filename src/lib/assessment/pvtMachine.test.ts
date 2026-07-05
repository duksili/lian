import { describe, expect, it } from "vitest";
import { PvtMachine, type PvtAction, type PvtConfig } from "./pvtMachine";

const CFG: PvtConfig = {
  durationMs: 5 * 60 * 1000,
  timeoutMs: 3000,
  feedbackMs: 550,
  falseStartMs: 100,
  intervals: Array.from({ length: 155 }, (_, i) => 2000 + ((i * 977) % 8001)), // 2000..10000
};

/** Drive the machine with a fake clock; respond after `rtFor(trialIdx)` ms (null = omit). */
function run(cfg: PvtConfig, rtFor: (i: number) => number | null) {
  const m = new PvtMachine(cfg);
  let now = 1_000_000; // arbitrary monotonic origin
  let action = m.begin(now);
  let stimuli = 0;
  while (action.kind !== "finished") {
    if (action.kind === "wait") {
      now += action.ms;
      action = m.stimulusDue(now);
    } else if (action.kind === "stimulus") {
      const rt = rtFor(stimuli++);
      if (rt == null) {
        now += action.timeoutMs;
        action = m.stimulusTimeout(now);
      } else {
        now += rt;
        action = m.response(now)!;
      }
    } else {
      now += action.ms;
      action = m.feedbackDone(now);
    }
  }
  return { m, elapsed: action.elapsedMs };
}

describe("PvtMachine deadline behavior", () => {
  it("never exceeds the 5-minute deadline with normal responses", () => {
    const { m, elapsed } = run(CFG, () => 280);
    expect(elapsed).toBeLessThanOrEqual(CFG.durationMs);
    expect(elapsed).toBeGreaterThan(CFG.durationMs - 20_000); // fills to tolerance
    expect(m.trials.length).toBeGreaterThan(20);
  });

  it("stays within the deadline even when every trial times out", () => {
    const { elapsed, m } = run(CFG, () => null);
    expect(elapsed).toBeLessThanOrEqual(CFG.durationMs);
    expect(m.trials.every((t) => t.response_ms === null)).toBe(true);
  });

  it("does not schedule a stimulus whose response window cannot fit (boundary)", () => {
    // Craft: after one instant trial, remaining time fits exactly one more ISI+timeout.
    const cfg: PvtConfig = { ...CFG, durationMs: 2000 + 0 + 550 + 2000 + 3000, intervals: [2000, 2000, 2000] };
    const { m, elapsed } = run(cfg, () => 0);
    // Trial 1: isi 2000 + rt 0 + feedback 550 = 2550. Next: 2550+2000+3000 = 7550 == duration -> fits.
    // After it: 2550+2000+0+550 = 5100; next would need 5100+2000+3000 = 10100 > 7550 -> stop.
    expect(m.trials.length).toBe(2);
    expect(elapsed).toBeLessThanOrEqual(cfg.durationMs);
  });
});

describe("PvtMachine trial classification data", () => {
  it("records false starts during the waiting phase without an onset", () => {
    const m = new PvtMachine(CFG);
    let now = 0;
    const a = m.begin(now);
    expect(a.kind).toBe("wait");
    now += 500; // press mid-ISI
    const fb = m.response(now);
    expect(fb?.kind).toBe("feedback");
    expect(m.trials[0].is_false_start).toBe(true);
    expect(m.trials[0].onset_ms).toBeNull();
    expect(m.trials[0].response_ms).toBe(500);
  });

  it("records omissions with onset but no response", () => {
    const m = new PvtMachine(CFG);
    let now = 0;
    let a = m.begin(now) as Extract<PvtAction, { kind: "wait" }>;
    now += a.ms;
    m.stimulusDue(now);
    now += CFG.timeoutMs;
    m.stimulusTimeout(now);
    expect(m.trials[0].response_ms).toBeNull();
    expect(m.trials[0].onset_ms).not.toBeNull();
  });

  it("records reaction time relative to onset", () => {
    const m = new PvtMachine(CFG);
    let now = 50_000;
    const a = m.begin(now) as Extract<PvtAction, { kind: "wait" }>;
    now += a.ms;
    m.stimulusDue(now);
    now += 312;
    m.response(now);
    expect(m.trials[0].reaction_time_ms).toBe(312);
    expect(m.trials[0].onset_ms).toBe(a.ms);
  });

  it("counts key repeats / held keys without attributing them to trials", () => {
    const m = new PvtMachine(CFG);
    let now = 0;
    m.begin(now);
    expect(m.response(now + 100, { repeat: true })).toBeNull();
    expect(m.response(now + 120, { heldAtOnset: true })).toBeNull();
    expect(m.keyHoldEvents).toBe(2);
    expect(m.trials.length).toBe(0);
  });
});
