<script lang="ts">
  import { onMount } from 'svelte';
  import type { DayReport, RangeHours } from '../../app/api';
  import DatePicker from './DatePicker.svelte';
  import Timeline from './Timeline.svelte';
  import TopCharts from './TopCharts.svelte';
  import RangeChart from './RangeChart.svelte';
  import EmptyState from './EmptyState.svelte';
  import { formatDateISO, isMockMode, loadDayReport, loadRangeHours } from './utils';

  let selectedDate = isMockMode() ? '2026-11-23' : formatDateISO(new Date());
  let timeWindow: 'week' | 'month' = 'week';

  let dayReport: DayReport | null = null;
  let rangeHours: RangeHours = [];
  let loading = false;
  let error: string | null = null;

  async function fetchDayData(date: string) {
    loading = true;
    error = null;
    try {
      dayReport = await loadDayReport(date);
    } catch (e: any) {
      error = e?.message || 'Failed to load day data';
    } finally {
      loading = false;
    }
  }

  async function fetchRangeData(date: string, windowType: 'week' | 'month') {
    try {
      const days = windowType === 'week' ? 7 : 30;
      rangeHours = await loadRangeHours(date, days);
    } catch (e: any) {
      console.error('Failed to load range hours:', e);
    }
  }

  async function handleDateChange(date: string) {
    if (selectedDate !== date) {
      selectedDate = date;
      await Promise.all([
        fetchDayData(selectedDate),
        fetchRangeData(selectedDate, timeWindow)
      ]);
    }
  }

  async function handleToggleWindow(w: 'week' | 'month') {
    timeWindow = w;
    await fetchRangeData(selectedDate, timeWindow);
  }

  $: isEmptyDay =
    !dayReport ||
    (dayReport.active_secs === 0 &&
      (!dayReport.timeline || dayReport.timeline.length === 0) &&
      (!dayReport.top_apps || dayReport.top_apps.length === 0));

  onMount(async () => {
    await Promise.all([
      fetchDayData(selectedDate),
      fetchRangeData(selectedDate, timeWindow)
    ]);
  });
</script>

<div class="daily-view-container">
  <!-- Page Header -->
  <header class="page-header">
    <div class="header-titles">
      <h2>Daily Overview</h2>
      <p class="header-desc">Track and analyze your daily activity, application usage, and browser time.</p>
    </div>
  </header>

  <!-- Section 1: Date Navigation Bar -->
  <section class="section-date-picker">
    <DatePicker
      currentDate={selectedDate}
      activeSecs={dayReport?.active_secs ?? 0}
      on:dateChange={(e) => handleDateChange(e.detail)}
    />
  </section>

  {#if error}
    <div class="error-banner">
      <span>Error: {error}</span>
    </div>
  {/if}

  <!-- Empty state message if no activity -->
  {#if isEmptyDay && !loading}
    <section class="section-empty">
      <EmptyState
        date={selectedDate}
        on:jumpToDate={(e) => handleDateChange(e.detail)}
      />
    </section>
  {/if}

  <!-- Section 2: SVG Timeline of the Day -->
  <section class="section-timeline">
    <Timeline
      timeline={dayReport?.timeline ?? []}
      reportDate={selectedDate}
    />
  </section>

  <!-- Section 3: Top Sites & Top Apps Bar Charts -->
  <section class="section-top-charts">
    <TopCharts
      topApps={dayReport?.top_apps ?? []}
      topSites={dayReport?.top_sites ?? []}
    />
  </section>

  <!-- Section 4: Hours Per Day (Week / Month Range Bar Chart) -->
  <section class="section-range">
    <RangeChart
      {selectedDate}
      rangeData={rangeHours}
      {timeWindow}
      on:dateSelect={(e) => handleDateChange(e.detail)}
      on:toggleWindow={(e) => handleToggleWindow(e.detail)}
    />
  </section>
</div>

<style>
  :global(:root) {
    --card-bg: #ffffff;
    --border-color: #e5e7eb;
    --text-color: #111827;
    --text-muted: #6b7280;
    --timeline-bg: #f8fafc;
    --tooltip-bg: #18181b;
    --tooltip-text: #ffffff;
    --empty-bg: #f9fafb;
    --item-row-bg: rgba(0, 0, 0, 0.025);
  }

  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --card-bg: #242427;
      --border-color: #38383c;
      --text-color: #f3f4f6;
      --text-muted: #9ca3af;
      --timeline-bg: #1e1e21;
      --tooltip-bg: #111113;
      --tooltip-text: #ffffff;
      --empty-bg: #1c1c1f;
      --item-row-bg: rgba(255, 255, 255, 0.04);
    }
  }

  .daily-view-container {
    max-width: 1200px;
    margin: 0 auto;
    padding: 1.5rem 2rem 3rem 2rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
    color: var(--text-color, #111827);
  }

  .page-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    border-bottom: 1px solid var(--border-color, #e5e7eb);
    padding-bottom: 1rem;
  }

  .header-titles h2 {
    margin: 0 0 0.25rem 0;
    font-size: 1.6rem;
    font-weight: 700;
    letter-spacing: -0.02em;
    color: var(--text-color, #111827);
  }

  .header-desc {
    margin: 0;
    font-size: 0.9rem;
    color: var(--text-muted, #6b7280);
  }

  .error-banner {
    background: #fef2f2;
    border: 1px solid #fecaca;
    color: #b91c1c;
    padding: 0.75rem 1rem;
    border-radius: 6px;
    font-size: 0.9rem;
  }

  @media (prefers-color-scheme: dark) {
    .error-banner {
      background: #450a0a;
      border-color: #7f1d1d;
      color: #fca5a5;
    }
  }

  section {
    width: 100%;
  }

  @media (max-width: 768px) {
    .daily-view-container {
      padding: 1rem;
      gap: 1.25rem;
    }
  }
</style>
