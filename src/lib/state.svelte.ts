import { api } from "./api";

export type Route =
  | { name: "today" }
  | { name: "timeline" }
  | { name: "calendar" }
  | { name: "assessments" }
  | { name: "determinations" }
  | { name: "review" }
  | { name: "research" }
  | { name: "settings" }
  | { name: "onboarding" };

export const nav = $state({ route: { name: "today" } as Route });

export function go(route: Route) {
  nav.route = route;
}

/** Global app data loaded once and refreshed after mutations. */
export const app = $state({
  loaded: false,
  settings: {} as any,
  templates: [] as any[],
  dimensions: [] as any[],
  status: {} as any,
});

export async function loadGlobals() {
  const [settings, templates, dimensions, status] = await Promise.all([
    api("settings.get"),
    api("templates.list", { include_archived: true }),
    api("dimensions.list"),
    api("meta.status"),
  ]);
  app.settings = settings;
  app.templates = templates;
  app.dimensions = dimensions;
  app.status = status;
  app.loaded = true;
  // Adopt the device timezone silently only at first run (before onboarding).
  const deviceTz = Intl.DateTimeFormat().resolvedOptions().timeZone;
  if (!settings.onboarding_complete && deviceTz && settings.timezone === "UTC") {
    app.settings = await api("settings.set", { timezone: deviceTz });
  }
}

export const activeTemplates = () => app.templates.filter((t) => !t.is_archived);
export const enabledDimensions = () => app.dimensions.filter((d) => d.is_enabled);

/* ---------------- toasts ---------------- */

export interface Toast {
  id: number;
  text: string;
  kind: "info" | "ok" | "error";
}

export const toasts = $state<{ items: Toast[] }>({ items: [] });
let toastId = 0;

export function toast(text: string, kind: Toast["kind"] = "info") {
  const id = ++toastId;
  toasts.items.push({ id, text, kind });
  setTimeout(() => {
    const i = toasts.items.findIndex((t) => t.id === id);
    if (i >= 0) toasts.items.splice(i, 1);
  }, kind === "error" ? 6000 : 3200);
}

export function reportError(e: unknown) {
  console.error(e);
  toast(String(e), "error");
}

/* ---------------- practice timer ---------------- */

export const timer = $state({
  running: false,
  startedAt: null as string | null,
  startedEpoch: 0,
  templateId: null as string | null,
  elapsed: 0,
});

let timerInterval: ReturnType<typeof setInterval> | null = null;

export function startTimer(templateId: string | null) {
  timer.running = true;
  timer.templateId = templateId;
  timer.startedEpoch = Date.now();
  timer.startedAt = new Date().toISOString();
  timer.elapsed = 0;
  timerInterval = setInterval(() => {
    timer.elapsed = Math.floor((Date.now() - timer.startedEpoch) / 1000);
  }, 1000);
}

export function stopTimer(): { seconds: number; startedEpoch: number } {
  const out = { seconds: Math.floor((Date.now() - timer.startedEpoch) / 1000), startedEpoch: timer.startedEpoch };
  timer.running = false;
  timer.templateId = null;
  if (timerInterval) clearInterval(timerInterval);
  timerInterval = null;
  return out;
}

/* ---------------- quick log modal (global) ---------------- */

export const quickLog = $state({
  open: false,
  /** optional prefill */
  templateId: null as string | null,
  planId: null as string | null,
  durationSeconds: null as number | null,
  occurredAt: null as string | null,
  source: "manual" as "manual" | "timer",
  /** editing an existing event */
  eventId: null as string | null,
});

export function openQuickLog(prefill: Partial<typeof quickLog> = {}) {
  quickLog.templateId = prefill.templateId ?? null;
  quickLog.planId = prefill.planId ?? null;
  quickLog.durationSeconds = prefill.durationSeconds ?? null;
  quickLog.occurredAt = prefill.occurredAt ?? null;
  quickLog.source = prefill.source ?? "manual";
  quickLog.eventId = prefill.eventId ?? null;
  quickLog.open = true;
}

/** Bumped after any data mutation so views can refresh. */
export const dataVersion = $state({ n: 0 });
export function bump() {
  dataVersion.n++;
}
