<script lang="ts">
  import RatingScale from "../components/RatingScale.svelte";
  import { api } from "../api";
  import { enabledDimensions, toast, reportError, bump } from "../state.svelte";
  import { toInstant } from "../format";

  let { date, existing = null, onsaved }: {
    date: string;
    existing?: any | null;
    onsaved?: () => void;
  } = $props();

  let ratings = $state<Record<string, number | null>>({});
  let note = $state("");
  let sleepStart = $state("");
  let sleepEnd = $state("");
  let sleepQuality = $state<number | null>(null);
  let awakenings = $state<string>("");
  let showSleep = $state(false);
  let saving = $state(false);

  $effect(() => {
    // Re-seed local fields when the target check-in changes.
    const e = existing;
    ratings = {};
    if (e) {
      for (const r of e.ratings ?? []) ratings[r.dimension_id] = r.value;
      note = e.note ?? "";
      sleepQuality = e.sleep_quality ?? null;
      awakenings = e.awakenings != null ? String(e.awakenings) : "";
      sleepStart = e.sleep_start ? hhmm(e.sleep_start) : "";
      sleepEnd = e.sleep_end ? hhmm(e.sleep_end) : "";
      showSleep = !!(e.sleep_start || e.sleep_quality);
    } else {
      note = ""; sleepStart = ""; sleepEnd = ""; sleepQuality = null; awakenings = "";
    }
  });

  function hhmm(instant: string): string {
    const d = new Date(instant);
    return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
  }

  function sleepDurationMinutes(): number | null {
    if (!sleepStart || !sleepEnd) return null;
    const [sh, sm] = sleepStart.split(":").map(Number);
    const [eh, em] = sleepEnd.split(":").map(Number);
    let mins = eh * 60 + em - (sh * 60 + sm);
    if (mins <= 0) mins += 24 * 60; // slept across midnight
    return mins;
  }

  async function save() {
    saving = true;
    try {
      const cleaned: Record<string, number> = {};
      for (const [k, v] of Object.entries(ratings)) if (v != null) cleaned[k] = v;
      const prevDay = new Date(date + "T12:00:00");
      prevDay.setDate(prevDay.getDate() - 1);
      const prev = `${prevDay.getFullYear()}-${String(prevDay.getMonth() + 1).padStart(2, "0")}-${String(prevDay.getDate()).padStart(2, "0")}`;
      await api("checkins.save", {
        id: existing?.id ?? null,
        local_date: date,
        ratings: cleaned,
        note: note || null,
        // Sleep start belongs to the prior evening when it is later than noon.
        sleep_start: sleepStart ? toInstant(Number(sleepStart.split(":")[0]) >= 12 ? prev : date, sleepStart) : null,
        sleep_end: sleepEnd ? toInstant(date, sleepEnd) : null,
        sleep_duration_minutes: sleepDurationMinutes(),
        sleep_quality: sleepQuality,
        awakenings: awakenings.trim() === "" ? null : Number(awakenings),
      });
      bump();
      toast("Check-in saved", "ok");
      onsaved?.();
    } catch (e) { reportError(e); }
    saving = false;
  }
</script>

<div class="col" style="gap:14px;">
  {#each enabledDimensions() as dim (dim.id)}
    <label class="field">
      <span>{dim.label}</span>
      <RatingScale
        value={ratings[dim.id] ?? null}
        anchorLow={dim.anchor_low}
        anchorHigh={dim.anchor_high}
        onchange={(v) => (ratings[dim.id] = v)}
      />
    </label>
  {/each}

  {#if showSleep}
    <div class="inset" style="padding: 12px 14px;">
      <div class="overline" style="margin-bottom: 10px;">Sleep (self-reported)</div>
      <div class="row wrap">
        <label class="field"><span>Fell asleep</span><input type="time" bind:value={sleepStart} /></label>
        <label class="field"><span>Woke</span><input type="time" bind:value={sleepEnd} /></label>
        <label class="field"><span>Awakenings</span>
          <input type="number" min="0" bind:value={awakenings} placeholder="—" style="width:74px;" /></label>
      </div>
      <label class="field" style="margin-top:10px;"><span>Sleep quality</span>
        <RatingScale value={sleepQuality} anchorLow="poor" anchorHigh="excellent" onchange={(v) => (sleepQuality = v)} />
      </label>
    </div>
  {:else}
    <button class="btn ghost sm" style="align-self:flex-start;" onclick={() => (showSleep = true)}>+ sleep</button>
  {/if}

  <label class="field"><span>Note</span>
    <textarea bind:value={note} placeholder="anything worth remembering about today (optional)"></textarea>
  </label>

  <div class="row">
    <span class="small faint grow">Skipped ratings stay unknown — that's fine.</span>
    <button class="btn primary" onclick={save} disabled={saving}>
      {existing ? "Update check-in" : "Save check-in"}
    </button>
  </div>
</div>
