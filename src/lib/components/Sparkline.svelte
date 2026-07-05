<script lang="ts">
  /** Minimal line/dot chart for personal-range trends. Dots are inspectable. */
  let {
    points,
    height = 44,
    color = "var(--cinnabar)",
    yMin = null,
    yMax = null,
    showDots = true,
  }: {
    points: { x: string; y: number; flag?: string }[];
    height?: number;
    color?: string;
    yMin?: number | null;
    yMax?: number | null;
    showDots?: boolean;
  } = $props();

  const W = 100;
  const pad = 4;

  const computed = $derived.by(() => {
    if (points.length === 0) return { path: "", dots: [] as { cx: number; cy: number; p: (typeof points)[0] }[] };
    const ys = points.map((p) => p.y);
    const lo = yMin ?? Math.min(...ys);
    const hi = yMax ?? Math.max(...ys);
    const span = hi - lo || 1;
    const dots = points.map((p, i) => ({
      cx: points.length === 1 ? W / 2 : pad + (i / (points.length - 1)) * (W - pad * 2),
      cy: height - pad - ((p.y - lo) / span) * (height - pad * 2),
      p,
    }));
    const path = dots.map((d, i) => `${i === 0 ? "M" : "L"}${d.cx.toFixed(1)},${d.cy.toFixed(1)}`).join(" ");
    return { path, dots };
  });
</script>

<svg viewBox="0 0 {W} {height}" preserveAspectRatio="none" style:height="{height}px" role="img">
  {#if computed.path}
    <path d={computed.path} fill="none" stroke={color} stroke-width="1.3" opacity="0.75"
      vector-effect="non-scaling-stroke" stroke-linejoin="round" />
  {/if}
  {#if showDots}
    {#each computed.dots as d}
      <circle cx={d.cx} cy={d.cy} r="1.8"
        fill={d.p.flag === "caution" ? "var(--caution)" : d.p.flag === "invalid" ? "var(--invalid)" : d.p.flag === "familiarization" ? "var(--paper-ghost)" : color}>
        <title>{d.p.x}: {typeof d.p.y === "number" ? +d.p.y.toFixed(3) : d.p.y}{d.p.flag ? ` (${d.p.flag})` : ""}</title>
      </circle>
    {/each}
  {/if}
</svg>

<style>
  svg { width: 100%; display: block; overflow: visible; }
</style>
