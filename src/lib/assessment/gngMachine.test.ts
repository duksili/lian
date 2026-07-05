import { describe, expect, it } from "vitest";
import { GngMachine, type GngConfig } from "./gngMachine";

const CFG: GngConfig = {
  stimulusMs: 600,
  responseWindowMs: 1000,
  trials: [
    { stimulus: "go", isi_ms: 1000 },
    { stimulus: "no_go", isi_ms: 1200 },
    { stimulus: "go", isi_ms: 900 },
  ],
};

function begin(cfg = CFG) {
  const m = new GngMachine(cfg);
  let now = 10_000;
  const a = m.begin(now);
  return { m, now, a };
}

describe("GngMachine response window", () => {
  it("accepts a response inside the window and stamps the correct trial", () => {
    const { m } = begin();
    let now = 10_000 + 1000;
    m.stimulusDue(now);
    now += 350;
    const next = m.response(now);
    expect(next?.kind).toBe("isi");
    expect(m.trials[0].trial_index).toBe(0);
    expect(m.trials[0].reaction_time_ms).toBe(350);
    expect(m.trials[0].onset_ms).toBe(1000);
  });

  it("accepts a late-but-in-window response (after 600 ms, before 1000 ms)", () => {
    const { m } = begin();
    let now = 10_000 + 1000;
    m.stimulusDue(now);
    now += 850; // stimulus already hidden, window still open
    expect(m.response(now)).not.toBeNull();
    expect(m.trials[0].reaction_time_ms).toBe(850);
  });

  it("rejects a response after the window closes", () => {
    const { m } = begin();
    let now = 10_000 + 1000;
    m.stimulusDue(now);
    now += 1001;
    expect(m.response(now)).toBeNull();
    // Window then closes as an omission for the go trial.
    m.windowClosed(now);
    expect(m.trials[0].response_ms).toBeNull();
  });

  it("never attributes a press during the next ISI to the upcoming trial", () => {
    const { m } = begin();
    let now = 10_000 + 1000;
    m.stimulusDue(now);
    now += 300;
    m.response(now); // trial 0 done, now in trial 1's ISI
    expect(m.phase).toBe("isi");
    // Press during ISI: previous onset must not leak; nothing recorded.
    expect(m.response(now + 100)).toBeNull();
    expect(m.trials.length).toBe(1);
    // Trial 1 then proceeds cleanly from its own onset.
    now += 1200;
    m.stimulusDue(now);
    now += 200;
    m.response(now);
    expect(m.trials[1].trial_index).toBe(1);
    expect(m.trials[1].onset_ms).toBe(now - 200 - 10_000);
  });

  it("counts key repeats and held-at-onset keys as key-hold events, not responses", () => {
    const { m } = begin();
    let now = 10_000 + 1000;
    m.stimulusDue(now);
    expect(m.response(now + 50, { repeat: true })).toBeNull();
    expect(m.response(now + 60, { heldAtOnset: true })).toBeNull();
    expect(m.keyHoldEvents).toBe(2);
    expect(m.trials.length).toBe(0);
    // A genuine press afterwards still works.
    expect(m.response(now + 400)).not.toBeNull();
    expect(m.trials.length).toBe(1);
  });

  it("commits no-go responses without a reaction time and finishes the sequence", () => {
    const { m } = begin();
    let now = 10_000 + 1000;
    m.stimulusDue(now); // go
    m.response(now + 300);
    now += 300 + 1200;
    m.stimulusDue(now); // no_go, commission
    m.response(now + 250);
    now += 250 + 900;
    m.stimulusDue(now); // go, omission
    const done = m.windowClosed(now + 1000);
    expect(done.kind).toBe("finished");
    expect(m.trials.length).toBe(3);
    expect(m.trials[1].stimulus_kind).toBe("no_go");
    expect(m.trials[1].reaction_time_ms).toBeNull();
    expect(m.trials[1].response_ms).not.toBeNull();
    expect(m.trials[2].response_ms).toBeNull();
  });
});
