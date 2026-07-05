/**
 * Dev-only smoke autopilot: when the settings row `ui_autopilot` is true
 * (set externally, e.g. by the seed_demo utility), the app walks through all
 * primary views on a fixed schedule so an external tool can capture each one.
 * Never bundled into release builds; self-clears its trigger when done.
 */
import { api } from "./api";
import { go, quickLog } from "./state.svelte";

const ROUTES = [
  "today", "timeline", "calendar", "assessments",
  "determinations", "review", "research", "settings",
] as const;

export async function maybeAutopilot(settings: any) {
  if (!import.meta.env.DEV || !settings?.ui_autopilot) return;
  let i = 0;
  const step = async () => {
    if (i < ROUTES.length) {
      go({ name: ROUTES[i] } as any);
      i++;
      setTimeout(step, 6000);
    } else if (i === ROUTES.length) {
      go({ name: "today" } as any);
      quickLog.open = true;
      i++;
      setTimeout(step, 6000);
    } else {
      quickLog.open = false;
      try { await api("settings.set", { ui_autopilot: false }); } catch { /* dev only */ }
    }
  };
  setTimeout(step, 3000);
}
