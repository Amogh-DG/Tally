<script lang="ts">
  import type { TimelineBlock } from '../../app/api';
  import {
    formatDuration,
    formatTimeOfDay,
    parseSecondsInDay,
    stringToColor
  } from './utils';

  export let timeline: TimelineBlock[] = [];
  export let reportDate: string = '';

  // SVG dimensions
  const svgWidth = 1000;
  const trackY = 22;
  const trackHeight = 44;
  const axisY = 88;

  // Hover state
  let hoveredBlock: (TimelineBlock & { durationSecs: number; startTime: string; endTime: string }) | null = null;
  let tooltipX = 0;
  let tooltipY = 0;
  let containerRef: HTMLDivElement;

  // Hour markers: every 3 hours (0, 3, 6, 9, 12, 15, 18, 21, 24)
  const hourMarkers = [0, 3, 6, 9, 12, 15, 18, 21, 24].map((h) => ({
    hour: h,
    label: `${String(h).padStart(2, '0')}:00`,
    x: (h / 24) * svgWidth
  }));

  $: processedBlocks = timeline.map((block) => {
    const startSec = parseSecondsInDay(block.start, reportDate);
    const endSec = parseSecondsInDay(block.end, reportDate);
    const durationSecs = Math.max(0, endSec - startSec);
    const x = (startSec / 86400) * svgWidth;
    const rawW = ((endSec - startSec) / 86400) * svgWidth;
    const width = Math.max(2, rawW); // minimum 2px so short bursts are visible
    const isSite = block.kind === 'site';
    const color = stringToColor(block.label || block.app, isSite);
    const startTime = formatTimeOfDay(block.start);
    const endTime = formatTimeOfDay(block.end);

    return {
      ...block,
      startSec,
      endSec,
      durationSecs,
      startTime,
      endTime,
      x,
      width,
      color
    };
  });

  function handleMouseMove(e: MouseEvent, block: any) {
    if (!containerRef) return;
    const rect = containerRef.getBoundingClientRect();
    tooltipX = e.clientX - rect.left;
    tooltipY = e.clientY - rect.top;
    hoveredBlock = block;
  }

  function handleMouseLeave() {
    hoveredBlock = null;
  }
</script>

<div
  class="timeline-container"
  bind:this={containerRef}
  on:mouseleave={handleMouseLeave}
  role="region"
  aria-label="Daily timeline visualization"
>
  <div class="timeline-header">
    <div class="title-group">
      <h3>Day Timeline</h3>
      <span class="subtitle">24-hour activity log</span>
    </div>
    <div class="legend">
      <div class="legend-item">
        <span class="legend-dot app-dot"></span>
        <span>Apps</span>
      </div>
      <div class="legend-item">
        <span class="legend-dot site-dot"></span>
        <span>Sites</span>
      </div>
    </div>
  </div>

  <div class="svg-wrapper">
    <svg
      viewBox="0 0 {svgWidth} 110"
      preserveAspectRatio="none"
      class="timeline-svg"
      role="img"
      aria-label="Timeline of active applications and sites throughout the day"
    >
      <!-- Background track -->
      <rect
        x="0"
        y={trackY}
        width={svgWidth}
        height={trackHeight}
        rx="6"
        class="track-bg"
      />

      <!-- Hour gridlines -->
      {#each hourMarkers as marker}
        <line
          x1={marker.x}
          y1={trackY - 4}
          x2={marker.x}
          y2={axisY - 8}
          class="grid-line"
          class:major-grid={marker.hour === 0 || marker.hour === 12 || marker.hour === 24}
        />
        <text
          x={marker.x}
          y={axisY + 12}
          text-anchor={marker.hour === 0 ? 'start' : marker.hour === 24 ? 'end' : 'middle'}
          class="axis-label"
        >
          {marker.label}
        </text>
      {/each}

      <!-- Timeline activity blocks -->
      {#if processedBlocks.length > 0}
        {#each processedBlocks as block}
          <g
            class="timeline-block-group"
            on:mousemove={(e) => handleMouseMove(e, block)}
            on:mouseenter={(e) => handleMouseMove(e, block)}
            role="img"
            aria-label="{block.kind}: {block.label} ({formatDuration(block.durationSecs)})"
          >
            <rect
              x={block.x}
              y={trackY + 2}
              width={block.width}
              height={trackHeight - 4}
              rx="4"
              fill={block.color}
              class="block-rect"
              class:is-active={hoveredBlock?.label === block.label && hoveredBlock?.start === block.start}
            />

            <!-- Block label if width allows -->
            {#if block.width > 55}
              <text
                x={block.x + 8}
                y={trackY + 25}
                class="block-text"
                clip-path="polygon(0 0, {block.width - 12}px 0, {block.width - 12}px 100%, 0 100%)"
              >
                {block.label}
              </text>
            {/if}
          </g>
        {/each}
      {:else}
        <!-- Empty timeline indicator -->
        <text
          x={svgWidth / 2}
          y={trackY + trackHeight / 2 + 5}
          text-anchor="middle"
          class="empty-track-text"
        >
          No activity recorded in this period
        </text>
      {/if}
    </svg>

    <!-- Floating hover details tooltip -->
    {#if hoveredBlock}
      <div
        class="timeline-tooltip"
        style="left: {Math.min(Math.max(tooltipX, 110), containerRef?.clientWidth ? containerRef.clientWidth - 110 : 200)}px; top: {tooltipY - 10}px;"
      >
        <div class="tooltip-header">
          <span class="tooltip-badge {hoveredBlock.kind}">
            {hoveredBlock.kind.toUpperCase()}
          </span>
          <span class="tooltip-time">
            {hoveredBlock.startTime} – {hoveredBlock.endTime}
          </span>
        </div>
        <div class="tooltip-label">{hoveredBlock.label}</div>
        <div class="tooltip-details">
          <span class="tooltip-app">{hoveredBlock.app}</span>
          <span class="tooltip-duration">{formatDuration(hoveredBlock.durationSecs)}</span>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .timeline-container {
    position: relative;
    background: var(--card-bg, #ffffff);
    border: 1px solid var(--border-color, #e0e0e0);
    border-radius: 8px;
    padding: 1.25rem;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
  }

  .timeline-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 1rem;
  }

  .title-group h3 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--text-color, #1a1a1a);
  }

  .title-group .subtitle {
    font-size: 0.8rem;
    color: var(--text-muted, #71717a);
  }

  .legend {
    display: flex;
    gap: 1rem;
    font-size: 0.85rem;
    color: var(--text-color, #1a1a1a);
  }

  .legend-item {
    display: flex;
    align-items: center;
    gap: 0.35rem;
  }

  .legend-dot {
    width: 10px;
    height: 10px;
    border-radius: 2px;
  }

  .app-dot {
    background: #3b82f6;
  }

  .site-dot {
    background: #10b981;
  }

  .svg-wrapper {
    position: relative;
    width: 100%;
    overflow: hidden;
  }

  .timeline-svg {
    width: 100%;
    height: 110px;
    display: block;
    user-select: none;
  }

  .track-bg {
    fill: var(--timeline-bg, #f1f5f9);
    stroke: var(--border-color, #e2e8f0);
    stroke-width: 1;
  }

  @media (prefers-color-scheme: dark) {
    .track-bg {
      fill: var(--timeline-bg, #2a2a2d);
      stroke: var(--border-color, #383838);
    }
  }

  .grid-line {
    stroke: var(--border-color, #cbd5e1);
    stroke-width: 1;
    stroke-dasharray: 2 3;
    opacity: 0.6;
  }

  .major-grid {
    opacity: 0.9;
    stroke-dasharray: none;
  }

  .axis-label {
    fill: var(--text-muted, #64748b);
    font-size: 11px;
    font-family: inherit;
  }

  .timeline-block-group {
    cursor: pointer;
    outline: none;
  }

  .block-rect {
    transition: opacity 0.15s ease, filter 0.15s ease;
    opacity: 0.92;
  }

  .block-rect:hover,
  .block-rect.is-active {
    opacity: 1;
    filter: brightness(1.15);
    stroke: #ffffff;
    stroke-width: 1.5;
  }

  .block-text {
    fill: #ffffff;
    font-size: 11px;
    font-weight: 500;
    pointer-events: none;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.4);
  }

  .empty-track-text {
    fill: var(--text-muted, #94a3b8);
    font-size: 12px;
    font-style: italic;
  }

  /* Tooltip styling */
  .timeline-tooltip {
    position: absolute;
    transform: translate(-50%, -100%);
    pointer-events: none;
    background: var(--tooltip-bg, #18181b);
    color: var(--tooltip-text, #fafafa);
    padding: 0.6rem 0.8rem;
    border-radius: 6px;
    font-size: 0.8rem;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
    z-index: 100;
    white-space: nowrap;
    border: 1px solid var(--border-color, #3f3f46);
  }

  .tooltip-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.3rem;
  }

  .tooltip-badge {
    font-size: 0.65rem;
    font-weight: 700;
    padding: 1px 4px;
    border-radius: 3px;
    letter-spacing: 0.5px;
  }

  .tooltip-badge.app {
    background: #2563eb;
    color: #ffffff;
  }

  .tooltip-badge.site {
    background: #059669;
    color: #ffffff;
  }

  .tooltip-time {
    color: var(--text-muted, #a1a1aa);
    font-size: 0.75rem;
  }

  .tooltip-label {
    font-weight: 600;
    font-size: 0.85rem;
    margin-bottom: 0.2rem;
  }

  .tooltip-details {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    font-size: 0.75rem;
    color: var(--text-muted, #a1a1aa);
  }

  .tooltip-duration {
    font-weight: 600;
    color: #38bdf8;
  }
</style>
