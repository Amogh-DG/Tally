<script lang="ts">
  import { createEventDispatcher } from 'svelte';
  import { formatDateDisplay } from './utils';

  export let date: string = '';

  const dispatch = createEventDispatcher<{
    jumpToDate: string;
  }>();

  function handleToday() {
    dispatch('jumpToDate', '2026-11-23');
  }
</script>

<div class="empty-day-banner">
  <div class="empty-icon-wrap">
    <svg viewBox="0 0 24 24" width="40" height="40" fill="none" stroke="currentColor" stroke-width="1.75">
      <circle cx="12" cy="12" r="10" />
      <line x1="12" y1="6" x2="12" y2="12" />
      <line x1="12" y1="12" x2="16" y2="14" />
    </svg>
  </div>
  <div class="empty-content">
    <h3>No activity recorded</h3>
    <p>
      There were no active applications or websites tracked on
      <strong>{formatDateDisplay(date)}</strong>.
    </p>
    <div class="empty-actions">
      <button class="action-btn" on:click={handleToday}>
        Jump to Fixture Day (2026-11-23)
      </button>
    </div>
  </div>
</div>

<style>
  .empty-day-banner {
    display: flex;
    align-items: center;
    gap: 1.5rem;
    background: var(--empty-bg, var(--card-bg, #ffffff));
    border: 1px dashed var(--border-color, #d4d4d8);
    border-radius: 8px;
    padding: 1.75rem 2rem;
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.03);
  }

  .empty-icon-wrap {
    color: var(--text-muted, #a1a1aa);
    flex-shrink: 0;
  }

  .empty-content h3 {
    margin: 0 0 0.35rem 0;
    font-size: 1.15rem;
    font-weight: 600;
    color: var(--text-color, #1a1a1a);
  }

  .empty-content p {
    margin: 0 0 0.85rem 0;
    font-size: 0.9rem;
    color: var(--text-muted, #71717a);
  }

  .empty-content strong {
    color: var(--text-color, #1a1a1a);
  }

  .action-btn {
    background: var(--sidebar-bg, #f4f4f5);
    border: 1px solid var(--border-color, #e4e4e7);
    color: #2563eb;
    font-weight: 600;
    padding: 0.45rem 0.9rem;
    border-radius: 6px;
    font-size: 0.85rem;
    cursor: pointer;
    transition: background-color 0.15s ease;
  }

  .action-btn:hover {
    background: var(--active-btn-bg, #e5e5e5);
  }

  @media (prefers-color-scheme: dark) {
    .action-btn {
      color: #60a5fa;
    }
  }
</style>
