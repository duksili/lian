/** Formatting helpers shared across views. */

export function fmtDuration(seconds: number | null | undefined): string {
  if (seconds == null) return "—";
  const m = Math.round(seconds / 60);
  if (m < 60) return `${m} min`;
  const h = Math.floor(m / 60);
  const rem = m % 60;
  return rem === 0 ? `${h} h` : `${h} h ${rem} min`;
}

export function fmtDate(date: string | null | undefined): string {
  if (!date) return "—";
  const d = new Date(date + "T12:00:00");
  return d.toLocaleDateString(undefined, { weekday: "short", month: "short", day: "numeric" });
}

export function fmtDateLong(date: string | null | undefined): string {
  if (!date) return "—";
  const d = new Date(date + "T12:00:00");
  return d.toLocaleDateString(undefined, { weekday: "long", year: "numeric", month: "long", day: "numeric" });
}

export function fmtTime(instant: string | null | undefined): string {
  if (!instant) return "—";
  return new Date(instant).toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}

export function fmtInstant(instant: string | null | undefined): string {
  if (!instant) return "—";
  const d = new Date(instant);
  return d.toLocaleDateString(undefined, { month: "short", day: "numeric" }) +
    " " + d.toLocaleTimeString(undefined, { hour: "2-digit", minute: "2-digit" });
}

export function todayStr(): string {
  return localDate(new Date());
}

export function localDate(d: Date): string {
  const y = d.getFullYear();
  const m = String(d.getMonth() + 1).padStart(2, "0");
  const day = String(d.getDate()).padStart(2, "0");
  return `${y}-${m}-${day}`;
}

export function addDays(date: string, days: number): string {
  const d = new Date(date + "T12:00:00");
  d.setDate(d.getDate() + days);
  return localDate(d);
}

export function weekStart(date: string): string {
  const d = new Date(date + "T12:00:00");
  const dow = (d.getDay() + 6) % 7; // Monday=0
  return addDays(date, -dow);
}

export function monthStart(date: string): string {
  return date.slice(0, 8) + "01";
}

export function nowLocalTimeHHMM(): string {
  const d = new Date();
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}

/** Instant (RFC3339 with local offset) for a local date + HH:MM. */
export function toInstant(date: string, hhmm: string): string {
  const d = new Date(`${date}T${hhmm}:00`);
  return rfc3339WithOffset(d);
}

export function rfc3339WithOffset(d: Date): string {
  const pad = (n: number) => String(Math.abs(n)).padStart(2, "0");
  const off = -d.getTimezoneOffset();
  const sign = off >= 0 ? "+" : "-";
  return (
    `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}` +
    `T${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}` +
    `${sign}${pad(Math.floor(off / 60))}:${pad(off % 60)}`
  );
}

export const WEEKDAY_LABELS = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

export const PRECEPT_LABELS: Record<string, string> = {
  non_harming_life: "Non-harming of life",
  not_taking_unoffered: "Not taking what is not offered",
  responsible_sexual_conduct: "Responsible sexual conduct",
  truthful_harmless_speech: "Truthful, harmless speech",
  clarity_regarding_intoxicants: "Clarity regarding intoxicants",
};

export const PRECEPT_KEYS = Object.keys(PRECEPT_LABELS);

export const CONTEXT_KINDS: [string, string][] = [
  ["illness", "Illness"],
  ["injury", "Injury / stiffness"],
  ["travel", "Travel"],
  ["workload", "Unusual workload"],
  ["emotional_stress", "Emotional stress"],
  ["routine_change", "Routine change"],
  ["practice_change", "Practice change"],
  ["sleep_disruption", "Sleep disruption"],
  ["caffeine", "Late caffeine"],
  ["alcohol", "Alcohol"],
  ["custom", "Other"],
];

export const EVIDENCE_LABELS: Record<string, { label: string; hint: string }> = {
  descriptive: { label: "Descriptive", hint: "A summary — no relationship was tested." },
  insufficient_data: { label: "Insufficient data", hint: "Not enough comparable observations yet." },
  observational_signal: { label: "Observational signal", hint: "An apparent association worth monitoring. Not causal." },
  candidate_hypothesis: { label: "Candidate hypothesis", hint: "A repeated signal you chose to investigate with a protocol." },
  protocol_result_inconclusive: { label: "Protocol — inconclusive", hint: "The protocol finished without a clear result." },
  protocol_result_supported: { label: "Protocol — supported", hint: "Consistent with the predefined hypothesis. Not universal proof." },
  protocol_result_not_supported: { label: "Protocol — not supported", hint: "Not consistent with the predefined hypothesis." },
};

export function validityPill(state: string | null | undefined): { cls: string; label: string } {
  switch (state) {
    case "valid": return { cls: "ok", label: "valid" };
    case "caution": return { cls: "caution", label: "caution" };
    case "invalid": return { cls: "invalid", label: "invalid" };
    default: return { cls: "", label: "unreviewed" };
  }
}
