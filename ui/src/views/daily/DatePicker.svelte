<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import {
    formatDateDisplay,
    formatDuration,
    shiftDate,
    formatDateISO,
    isMockMode
  } from './utils';

  export let currentDate: string = '';
  export let activeSecs: number = 0;

  const dispatch = createEventDispatcher<{
    dateChange: string;
  }>();

  const todayStr = isMockMode() ? '2026-11-23' : formatDateISO(new Date());

  function handlePrev() {
    const prev = shiftDate(currentDate, -1);
    dispatch('dateChange', prev);
  }

  function handleNext() {
    const next = shiftDate(currentDate, 1);
    dispatch('dateChange', next);
  }

  function handleDateInput(e: Event) {
    const input = e.target as HTMLInputElement;
    if (input.value) {
      dispatch('dateChange', input.value);
    }
  }

  function handleToday() {
    dispatch('dateChange', todayStr);
  }
</script>

<div class="date-picker-bar">
  <div class="nav-controls">
    <button class="nav-btn prev-btn" on:click={handlePrev} title="Previous Day">
      <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
        <path d="M15.41 7.41L14 6l-6 6 6 6 1.41-1.41L10.83 12z"/>
      </svg>
      <span>Prev</span>
    </button>

    <div class="date-display-wrapper">
      <input
        type="date"
        class="date-input"
        value={currentDate}
        on:change={handleDateInput}
        aria-label="Select Date"
      />
      <div class="date-label">
        <span class="date-text">{formatDateDisplay(currentDate)}</span>
        {#if currentDate === todayStr}
          <span class="today-badge">Today</span>
        {/if}
      </div>
    </div>

    <button class="nav-btn next-btn" on:click={handleNext} title="Next Day">
      <span>Next</span>
      <svg viewBox="0 0 24 24" width="16" height="16" fill="currentColor">
        <path d="M10 6L8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6z"/>
      </svg>
    </button>

    {#if currentDate !== todayStr}
      <button class="today-btn" on:click={handleToday} title="Jump to Today">
        Jump to Today
      </button>
    {/if}
  </div>

  <div class="active-summary">
    <span class="summary-label">Total Active Time:</span>
    <span class="summary-value" class:has-activity={activeSecs > 0}>
      {formatDuration(activeSecs)}
    </span>
  </div>
</div>

<style>
  .date-picker-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    flex-wrap: wrap;
    gap: 1rem;
    background: var(--bg-card, var(--card-bg, #ffffff));
    border: 1px solid var(--border-color, #e0e0e0);
    border-radius: 8px;
    padding: 0.85rem 1.25rem;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.05);
  }

  .nav-controls {
    display: flex;
    align-items: center;
    gap: 0.6rem;
  }

  .nav-btn {
    display: flex;
    align-items: center;
    gap: 0.25rem;
    background: var(--bg-card-subtle, var(--sidebar-bg, #f4f4f5));
    border: 1px solid var(--border-color, #e4e4e7);
    color: var(--text-primary, var(--text-color, #1a1a1a));
    padding: 0.4rem 0.75rem;
    border-radius: 6px;
    font-size: 0.85rem;
    cursor: pointer;
    font-weight: 500;
    transition: background-color 0.15s ease, border-color 0.15s ease;
  }

  .nav-btn:hover {
    background: var(--item-active-bg, var(--active-btn-bg, #e5e5e5));
    border-color: var(--border-color, #d4d4d8);
  }

  .date-display-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .date-input {
    position: absolute;
    top: 0;
    left: 0;
    width: 100%;
    height: 100%;
    opacity: 0;
    cursor: pointer;
    z-index: 2;
  }

  .date-label {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.4rem 0.85rem;
    background: var(--bg-card-subtle, var(--sidebar-bg, #f4f4f5));
    border: 1px solid var(--border-color, #e4e4e7);
    border-radius: 6px;
    cursor: pointer;
    transition: background-color 0.15s ease;
  }

  .date-display-wrapper:hover .date-label {
    background: var(--item-active-bg, var(--active-btn-bg, #e5e5e5));
  }

  .date-text {
    font-weight: 600;
    font-size: 0.95rem;
    color: var(--text-primary, var(--text-color, #1a1a1a));
  }

  .today-badge {
    background: #3b82f6;
    color: #ffffff;
    font-size: 0.65rem;
    font-weight: 700;
    padding: 1px 6px;
    border-radius: 10px;
    text-transform: uppercase;
  }

  .today-btn {
    background: transparent;
    border: 1px dashed var(--border-color, #d4d4d8);
    color: #3b82f6;
    padding: 0.4rem 0.75rem;
    border-radius: 6px;
    font-size: 0.8rem;
    cursor: pointer;
    font-weight: 500;
  }

  .today-btn:hover {
    background: rgba(59, 130, 246, 0.08);
  }

  .active-summary {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.9rem;
  }

  .summary-label {
    color: var(--text-secondary, var(--text-muted, #71717a));
  }

  .summary-value {
    font-weight: 700;
    font-size: 1.1rem;
    color: var(--text-secondary, var(--text-muted, #71717a));
  }

  .summary-value.has-activity {
    color: var(--accent, #2563eb);
  }

  :global(.dark) .summary-value.has-activity {
    color: var(--accent, #60a5fa);
  }

  @media (prefers-color-scheme: dark) {
    .summary-value.has-activity {
      color: var(--accent, #60a5fa);
    }
  }
</style>
