<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import Chart from 'chart.js/auto';
  import type { TopApp, TopSite } from '../../app/api';
  import { formatDuration } from './utils';

  export let topApps: TopApp[] = [];
  export let topSites: TopSite[] = [];

  let appsCanvas: HTMLCanvasElement;
  let sitesCanvas: HTMLCanvasElement;
  let appsChart: Chart | null = null;
  let sitesChart: Chart | null = null;

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
      tooltipBorder: isDarkMode ? '#3f3f46' : '#e4e4e7'
    };
  }

  function renderAppsChart() {
    if (!appsCanvas || topApps.length === 0) {
      if (appsChart) {
        appsChart.destroy();
        appsChart = null;
      }
      return;
    }

    if (appsChart) {
      appsChart.destroy();
    }

    const theme = getThemeColors();
    const sortedApps = [...topApps].sort((a, b) => b.secs - a.secs);
    const labels = sortedApps.map((a) => a.app);
    const dataInHours = sortedApps.map((a) => Number((a.secs / 3600).toFixed(2)));
    const rawSecs = sortedApps.map((a) => a.secs);

    appsChart = new Chart(appsCanvas, {
      type: 'bar',
      data: {
        labels,
        datasets: [
          {
            label: 'Hours',
            data: dataInHours,
            backgroundColor: isDarkMode ? '#3b82f6' : '#2563eb',
            borderColor: isDarkMode ? '#60a5fa' : '#1d4ed8',
            borderWidth: 1,
            borderRadius: 4
          }
        ]
      },
      options: {
        indexAxis: 'y',
        responsive: true,
        maintainAspectRatio: false,
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
              label: (context) => {
                const secs = rawSecs[context.dataIndex];
                return ` Active Time: ${formatDuration(secs)}`;
              }
            }
          }
        },
        scales: {
          x: {
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
          },
          y: {
            ticks: {
              color: theme.textColor,
              font: { size: 12, weight: 500 }
            },
            grid: { display: false }
          }
        }
      }
    });
  }

  function renderSitesChart() {
    if (!sitesCanvas || topSites.length === 0) {
      if (sitesChart) {
        sitesChart.destroy();
        sitesChart = null;
      }
      return;
    }

    if (sitesChart) {
      sitesChart.destroy();
    }

    const theme = getThemeColors();
    const sortedSites = [...topSites].sort((a, b) => b.secs - a.secs);
    const labels = sortedSites.map((s) => s.domain);
    const dataInHours = sortedSites.map((s) => Number((s.secs / 3600).toFixed(2)));
    const rawSecs = sortedSites.map((s) => s.secs);

    sitesChart = new Chart(sitesCanvas, {
      type: 'bar',
      data: {
        labels,
        datasets: [
          {
            label: 'Hours',
            data: dataInHours,
            backgroundColor: isDarkMode ? '#10b981' : '#059669',
            borderColor: isDarkMode ? '#34d399' : '#047857',
            borderWidth: 1,
            borderRadius: 4
          }
        ]
      },
      options: {
        indexAxis: 'y',
        responsive: true,
        maintainAspectRatio: false,
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
              label: (context) => {
                const secs = rawSecs[context.dataIndex];
                return ` Active Time: ${formatDuration(secs)}`;
              }
            }
          }
        },
        scales: {
          x: {
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
          },
          y: {
            ticks: {
              color: theme.textColor,
              font: { size: 12, weight: 500 }
            },
            grid: { display: false }
          }
        }
      }
    });
  }

  function updateCharts() {
    renderAppsChart();
    renderSitesChart();
  }

  $: if (topApps || topSites) {
    updateCharts();
  }

  onMount(() => {
    checkDarkMode();
    if (typeof window !== 'undefined') {
      mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
      const listener = () => {
        checkDarkMode();
        updateCharts();
      };
      mediaQuery.addEventListener('change', listener);
    }
    updateCharts();
  });

  onDestroy(() => {
    if (appsChart) appsChart.destroy();
    if (sitesChart) sitesChart.destroy();
  });
</script>

<div class="top-charts-grid">
  <!-- Top Apps Section -->
  <div class="chart-card">
    <div class="card-header">
      <div class="header-icon app-icon">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
          <path d="M4 4h7v7H4V4zm9 0h7v7h-7V4zm-9 9h7v7H4v-7zm9 0h7v7h-7v-7z"/>
        </svg>
      </div>
      <div class="header-title">
        <h4>Top Applications</h4>
        <span class="header-count">{topApps.length} active</span>
      </div>
    </div>

    {#if topApps.length > 0}
      <div class="canvas-container">
        <canvas bind:this={appsCanvas}></canvas>
      </div>
      <div class="items-list">
        {#each [...topApps].sort((a, b) => b.secs - a.secs) as app, i}
          <div class="item-row">
            <span class="rank-badge">#{i + 1}</span>
            <span class="item-name" title={app.app}>{app.app}</span>
            <span class="item-duration">{formatDuration(app.secs)}</span>
          </div>
        {/each}
      </div>
    {:else}
      <div class="empty-state">
        <p>No application activity recorded</p>
      </div>
    {/if}
  </div>

  <!-- Top Sites Section -->
  <div class="chart-card">
    <div class="card-header">
      <div class="header-icon site-icon">
        <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
          <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm-1 17.93c-3.95-.49-7-3.85-7-7.93 0-.62.08-1.21.21-1.79L9 15v1c0 1.1.9 2 2 2v1.93zm6.9-2.54c-.26-.81-1-1.39-1.9-1.39h-1v-3c0-.55-.45-1-1-1H8v-2h2c.55 0 1-.45 1-1V7h2c1.1 0 2-.9 2-2v-.41c2.93 1.19 5 4.06 5 7.41 0 2.08-.8 3.97-2.1 5.39z"/>
        </svg>
      </div>
      <div class="header-title">
        <h4>Top Websites</h4>
        <span class="header-count">{topSites.length} visited</span>
      </div>
    </div>

    {#if topSites.length > 0}
      <div class="canvas-container">
        <canvas bind:this={sitesCanvas}></canvas>
      </div>
      <div class="items-list">
        {#each [...topSites].sort((a, b) => b.secs - a.secs) as site, i}
          <div class="item-row">
            <span class="rank-badge">#{i + 1}</span>
            <span class="item-name" title={site.domain}>{site.domain}</span>
            <span class="item-duration">{formatDuration(site.secs)}</span>
          </div>
        {/each}
      </div>
    {:else}
      <div class="empty-state">
        <p>No website visits recorded</p>
      </div>
    {/if}
  </div>
</div>

<style>
  .top-charts-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 1.25rem;
  }

  .chart-card {
    background: var(--card-bg, #ffffff);
    border: 1px solid var(--border-color, #e0e0e0);
    border-radius: 8px;
    padding: 1.25rem;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
    display: flex;
    flex-direction: column;
  }

  .card-header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 1rem;
  }

  .header-icon {
    width: 32px;
    height: 32px;
    border-radius: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .app-icon {
    background: #eff6ff;
    color: #2563eb;
  }

  .site-icon {
    background: #ecfdf5;
    color: #059669;
  }

  @media (prefers-color-scheme: dark) {
    .app-icon {
      background: #1e3a5f;
      color: #60a5fa;
    }
    .site-icon {
      background: #064e3b;
      color: #34d399;
    }
  }

  .header-title h4 {
    margin: 0;
    font-size: 1rem;
    font-weight: 600;
    color: var(--text-color, #1a1a1a);
  }

  .header-count {
    font-size: 0.75rem;
    color: var(--text-muted, #71717a);
  }

  .canvas-container {
    height: 160px;
    position: relative;
    margin-bottom: 1rem;
  }

  .items-list {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    border-top: 1px solid var(--border-color, #e0e0e0);
    padding-top: 0.75rem;
  }

  .item-row {
    display: flex;
    align-items: center;
    font-size: 0.85rem;
    padding: 0.25rem 0.5rem;
    border-radius: 4px;
    background: var(--item-row-bg, rgba(0, 0, 0, 0.02));
  }

  @media (prefers-color-scheme: dark) {
    .item-row {
      background: rgba(255, 255, 255, 0.04);
    }
  }

  .rank-badge {
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted, #71717a);
    width: 28px;
  }

  .item-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text-color, #1a1a1a);
    font-weight: 500;
  }

  .item-duration {
    font-weight: 600;
    color: var(--text-color, #1a1a1a);
    font-size: 0.8rem;
  }

  .empty-state {
    padding: 2.5rem 1rem;
    text-align: center;
    color: var(--text-muted, #71717a);
    font-style: italic;
    font-size: 0.9rem;
  }
</style>
