<script lang="ts">
  import { onMount, onDestroy, createEventDispatcher } from 'svelte';
  import Chart from 'chart.js/auto';
  import type { RangeHoursItem } from '../../app/api';
  import {
    formatDuration,
    formatDateDisplay,
    shiftDate
  } from './utils';

  export let selectedDate: string = '';
  export let rangeData: RangeHoursItem[] = [];
  export let timeWindow: 'week' | 'month' = 'week';

  const dispatch = createEventDispatcher<{
    dateSelect: string;
    toggleWindow: 'week' | 'month';
  }>();

  let canvas: HTMLCanvasElement;
  let chart: Chart | null = null;
  let isDarkMode = false;
  let mediaQuery: MediaQueryList | null = null;

  function checkDarkMode() {
    if (typeof window !== 'undefined') {
      isDarkMode = window.matchMedia('(prefers-color-scheme: dark)').matches;
    }
  }

  function getThemeColors() {
    return {
      textColor: isDarkMode ? '#e4e4e7' : '#27272a',
      gridColor: isDarkMode ? 'rgba(255, 255, 255, 0.08)' : 'rgba(0, 0, 0, 0.06)',
      tooltipBg: isDarkMode ? '#18181b' : '#ffffff',
      tooltipText: isDarkMode ? '#f4f4f5' : '#09090b',
      tooltipBorder: isDarkMode ? '#3f3f46' : '#e4e4e7',
      normalBar: isDarkMode ? 'rgba(99, 102, 241, 0.65)' : 'rgba(79, 70, 229, 0.75)',
      normalBorder: isDarkMode ? '#818cf8' : '#4f46e5',
      selectedBar: isDarkMode ? '#38bdf8' : '#0284c7',
      selectedBorder: isDarkMode ? '#7dd3fc' : '#0369a1'
    };
  }

  // Calculate filled date list for week (7 days) or month (30 days)
  $: daysCount = timeWindow === 'week' ? 7 : 30;
  $: fullDateList = (() => {
    const dates: string[] = [];
    for (let i = daysCount - 1; i >= 0; i--) {
      dates.push(shiftDate(selectedDate, -i));
    }
    return dates;
  })();

  // Map range data onto date list
  $: dataMap = new Map(rangeData.map((d) => [d.date, d.active_secs]));
  $: chartItems = fullDateList.map((d) => ({
    date: d,
    secs: dataMap.get(d) || 0,
    hours: Number(((dataMap.get(d) || 0) / 3600).toFixed(2)),
    isSelected: d === selectedDate
  }));

  $: totalSecs = chartItems.reduce((acc, curr) => acc + curr.secs, 0);
  $: avgSecs = daysCount > 0 ? totalSecs / daysCount : 0;

  function formatDateShort(dateStr: string): string {
    const parts = dateStr.split('-');
    if (parts.length < 3) return dateStr;
    const [, m, d] = parts;
    if (timeWindow === 'week') {
      const day = new Date(Number(parts[0]), Number(m) - 1, Number(d));
      const weekday = day.toLocaleDateString(undefined, { weekday: 'short' });
      return `${weekday} ${d}`;
    }
    return `${m}/${d}`;
  }

  function renderChart() {
    if (!canvas || chartItems.length === 0) {
      if (chart) {
        chart.destroy();
        chart = null;
      }
      return;
    }

    if (chart) {
      chart.destroy();
    }

    const theme = getThemeColors();
    const labels = chartItems.map((item) => formatDateShort(item.date));
    const data = chartItems.map((item) => item.hours);
    const backgroundColors = chartItems.map((item) =>
      item.isSelected ? theme.selectedBar : theme.normalBar
    );
    const borderColors = chartItems.map((item) =>
      item.isSelected ? theme.selectedBorder : theme.normalBorder
    );

    chart = new Chart(canvas, {
      type: 'bar',
      data: {
        labels,
        datasets: [
          {
            label: 'Hours Active',
            data,
            backgroundColor: backgroundColors,
            borderColor: borderColors,
            borderWidth: 1.5,
            borderRadius: 4
          }
        ]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        onClick: (_event, elements) => {
          if (elements.length > 0) {
            const index = elements[0].index;
            const clickedDate = chartItems[index].date;
            dispatch('dateSelect', clickedDate);
          }
        },
        plugins: {
          legend: { display: false },
          tooltip: {
            backgroundColor: theme.tooltipBg,
            titleColor: theme.tooltipText,
            bodyColor: theme.tooltipText,
            borderColor: theme.tooltipBorder,
            borderWidth: 1,
            padding: 8,
            callbacks: {
              title: (items) => {
                const item = chartItems[items[0].dataIndex];
                return formatDateDisplay(item.date);
              },
              label: (context) => {
                const item = chartItems[context.dataIndex];
                return ` Active Time: ${formatDuration(item.secs)}`;
              }
            }
          }
        },
        scales: {
          x: {
            ticks: {
              color: theme.textColor,
              font: { size: timeWindow === 'week' ? 12 : 10 },
              maxRotation: timeWindow === 'month' ? 45 : 0
            },
            grid: { display: false }
          },
          y: {
            title: {
              display: true,
              text: 'Hours',
              color: theme.textColor,
              font: { size: 11 }
            },
            ticks: {
              color: theme.textColor,
              font: { size: 11 }
            },
            grid: { color: theme.gridColor }
          }
        }
      }
    });
  }

  $: if (chartItems && canvas) {
    renderChart();
  }

  onMount(() => {
    checkDarkMode();
    if (typeof window !== 'undefined') {
      mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      const listener = () => {
        checkDarkMode();
        renderChart();
      };
      mediaQuery.addEventListener('change', listener);
    }
    renderChart();
  });

  onDestroy(() => {
    if (chart) chart.destroy();
  });

  function setWindow(w: 'week' | 'month') {
    if (timeWindow !== w) {
      timeWindow = w;
      dispatch('toggleWindow', w);
    }
  }
</script>

<div class="range-card">
  <div class="range-header">
    <div class="title-section">
      <div class="header-icon range-icon">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
          <path d="M19 3h-1V1h-2v2H8V1H6v2H5c-1.11 0-2 .9-2 2v14c0 1.1.89 2 2 2h14c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2zm0 16H5V8h14v11z"/>
        </svg>
      </div>
      <div>
        <h3>Hours Per Day</h3>
        <span class="range-summary">
          Total: <strong>{formatDuration(totalSecs)}</strong> &middot; Avg: <strong>{formatDuration(avgSecs)}/day</strong>
        </span>
      </div>
    </div>

    <!-- Week / Month toggle -->
    <div class="toggle-group" role="group" aria-label="Time period selection">
      <button
        class="toggle-btn"
        class:active={timeWindow === 'week'}
        on:click={() => setWindow('week')}
      >
        Past Week
      </button>
      <button
        class="toggle-btn"
        class:active={timeWindow === 'month'}
        on:click={() => setWindow('month')}
      >
        Past Month
      </button>
    </div>
  </div>

  <div class="canvas-container">
    <canvas bind:this={canvas}></canvas>
  </div>
</div>

<style>
  .range-card {
    background: var(--card-bg, #ffffff);
    border: 1px solid var(--border-color, #e0e0e0);
    border-radius: 8px;
    padding: 1.25rem;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
  }

  .range-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 1rem;
    margin-bottom: 1.25rem;
  }

  .title-section {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .header-icon.range-icon {
    width: 32px;
    height: 32px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: #e0e7ff;
    color: #4338ca;
  }

  @media (prefers-color-scheme: dark) {
    .header-icon.range-icon {
      background: #312e81;
      color: #a5b4fc;
    }
  }

  .title-section h3 {
    margin: 0;
    font-size: 1.1rem;
    font-weight: 600;
    color: var(--text-color, #1a1a1a);
  }

  .range-summary {
    font-size: 0.8rem;
    color: var(--text-muted, #71717a);
  }

  .range-summary strong {
    color: var(--text-color, #1a1a1a);
  }

  .toggle-group {
    display: flex;
    background: var(--sidebar-bg, #f4f4f5);
    border: 1px solid var(--border-color, #e4e4e7);
    border-radius: 6px;
    padding: 2px;
  }

  .toggle-btn {
    background: transparent;
    border: none;
    padding: 0.35rem 0.85rem;
    font-size: 0.85rem;
    color: var(--text-muted, #71717a);
    border-radius: 4px;
    cursor: pointer;
    font-weight: 500;
    transition: background-color 0.15s ease, color 0.15s ease;
  }

  .toggle-btn:hover {
    color: var(--text-color, #1a1a1a);
  }

  .toggle-btn.active {
    background: var(--card-bg, #ffffff);
    color: var(--text-color, #1a1a1a);
    font-weight: 600;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
  }

  @media (prefers-color-scheme: dark) {
    .toggle-btn.active {
      background: #3f3f46;
    }
  }

  .canvas-container {
    height: 220px;
    position: relative;
    width: 100%;
  }
</style>
