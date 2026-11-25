<script lang="ts">
  import { onMount } from 'svelte';
  import SitesView from '../views/sites/View.svelte';
  import DailyView from '../views/daily/View.svelte';
  import SettingsView from '../views/settings/View.svelte';
  import { isDarkMode } from './theme';
  import { server_status } from './api';

  type Tab = 'daily' | 'sites' | 'settings';
  let activeTab: Tab = 'daily';

  let isMockMode = false;
  let serverReachable: boolean | null = null;

  onMount(async () => {
    isMockMode =
      typeof window !== 'undefined' &&
      (import.meta.env.VITE_MOCK === '1' ||
        import.meta.env.VITE_MOCK === 'true' ||
        !(window as any).__TAURI_INTERNALS__);
    try {
      const status = await server_status();
      serverReachable = status.reachable;
    } catch {
      serverReachable = false;
    }
  });
</script>

<div class="shell" class:dark={$isDarkMode}>
  <aside class="sidebar">
    <div class="brand-row">
      <div class="brand-icon">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10" />
          <polyline points="12 6 12 12 16 14" />
        </svg>
      </div>
      <div class="brand-text">
        <span class="brand-title">Tidemark</span>
        {#if isMockMode}
          <span class="mock-badge" title="Running in fixture mock mode">MOCK</span>
        {/if}
      </div>
    </div>

    <nav class="nav-links" aria-label="Main Navigation">
      <button
        type="button"
        class="nav-item"
        class:active={activeTab === 'daily'}
        on:click={() => (activeTab = 'daily')}
        title="View daily activity and timeline"
      >
        <svg class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="3" y="4" width="18" height="18" rx="2" ry="2" />
          <line x1="16" y1="2" x2="16" y2="6" />
          <line x1="8" y1="2" x2="8" y2="6" />
          <line x1="3" y1="10" x2="21" y2="10" />
        </svg>
        <span>Daily</span>
      </button>

      <button
        type="button"
        class="nav-item"
        class:active={activeTab === 'sites'}
        on:click={() => (activeTab = 'sites')}
        title="Browse web domains and pages"
      >
        <svg class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10" />
          <line x1="2" y1="12" x2="22" y2="12" />
          <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
        </svg>
        <span>Sites</span>
      </button>

      <button
        type="button"
        class="nav-item"
        class:active={activeTab === 'settings'}
        on:click={() => (activeTab = 'settings')}
        title="Configure tracking settings"
      >
        <svg class="nav-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="3" />
          <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z" />
        </svg>
        <span>Settings</span>
      </button>
    </nav>

    <div class="sidebar-footer">
      {#if serverReachable !== null}
        <div class="theme-indicator" title="Local tm-server connection status">
          <span style="width:8px;height:8px;border-radius:50%;display:inline-block;background:{serverReachable ? '#107c41' : '#c50f1f'};"></span>
          <span>{serverReachable ? 'Server connected' : 'Server offline'}</span>
        </div>
      {/if}
      <div class="theme-indicator" title="Theme tracks Windows system setting">
        {#if $isDarkMode}
          <svg class="theme-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <path d="M21 12.79A9 9 0 1 1 11.21 3 7 7 0 0 0 21 12.79z" />
          </svg>
          <span>Dark Mode (OS)</span>
        {:else}
          <svg class="theme-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="12" cy="12" r="5" />
            <line x1="12" y1="1" x2="12" y2="3" />
            <line x1="12" y1="2" x2="12" y2="23" />
            <line x1="4.22" y1="4.22" x2="5.64" y2="5.64" />
            <line x1="18.36" y1="18.36" x2="19.78" y2="19.78" />
            <line x1="1" y1="12" x2="3" y2="12" />
            <line x1="21" y1="12" x2="23" y2="12" />
            <line x1="4.22" y1="19.78" x2="5.64" y2="18.36" />
            <line x1="18.36" y1="5.64" x2="19.78" y2="4.22" />
          </svg>
          <span>Light Mode (OS)</span>
        {/if}
      </div>
    </div>
  </aside>

  <main class="content-area">
    {#if activeTab === 'daily'}
      <DailyView />
    {:else if activeTab === 'sites'}
      <SitesView />
    {:else if activeTab === 'settings'}
      <SettingsView />
    {/if}
  </main>
</div>

<style>
  :global(:root) {
    --font-family: 'Segoe UI Variable', 'Segoe UI', system-ui, -apple-system, BlinkMacSystemFont, sans-serif;
    --bg-app: #f3f3f3;
    --bg-sidebar: #f8f8f8;
    --bg-card: #ffffff;
    --bg-card-hover: #fafafa;
    --bg-card-subtle: #f0f0f0;
    --text-primary: #1c1c1c;
    --text-secondary: #5e5e5e;
    --text-muted: #8a8a8a;
    --border-color: #e2e2e2;
    --border-subtle: #ececec;
    --accent: #0067b8;
    --accent-hover: #005a9e;
    --accent-light: #e1effa;
    --accent-text: #ffffff;
    --item-active-bg: #e6e6e6;
    --item-hover-bg: #eeeeee;
    --input-bg: #ffffff;
    --input-border: #d1d1d1;
    --input-focus-border: #0067b8;
    --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.05);
    --shadow-md: 0 4px 12px rgba(0, 0, 0, 0.08);
    --badge-bg: #e8e8e8;
    --badge-text: #454545;
    --success-bg: #dff6dd;
    --success-border: #107c41;
    --success-text: #107c41;
    /* Semantic aliases for all views */
    --card-bg: var(--bg-card);
    --sidebar-bg: var(--bg-sidebar);
    --text-color: var(--text-primary);
    --active-btn-bg: var(--item-active-bg);
  }

  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --bg-app: #202020;
      --bg-sidebar: #181818;
      --bg-card: #2c2c2c;
      --bg-card-hover: #323232;
      --bg-card-subtle: #242424;
      --text-primary: #f5f5f5;
      --text-secondary: #cccccc;
      --text-muted: #8e8e8e;
      --border-color: #383838;
      --border-subtle: #2e2e2e;
      --accent: #60cdff;
      --accent-hover: #4cc2fc;
      --accent-light: rgba(96, 205, 255, 0.15);
      --accent-text: #000000;
      --item-active-bg: #333333;
      --item-hover-bg: #2b2b2b;
      --input-bg: #2b2b2b;
      --input-border: #444444;
      --input-focus-border: #60cdff;
      --shadow-sm: 0 1px 4px rgba(0, 0, 0, 0.25);
      --shadow-md: 0 4px 16px rgba(0, 0, 0, 0.35);
      --badge-bg: #383838;
      --badge-text: #cccccc;
      --success-bg: rgba(16, 124, 65, 0.2);
      --success-border: #6ccb5f;
      --success-text: #9fe597;
      --bar-bg: #3a3a3a;
      --bar-fill: #60cdff;

      --card-bg: #2c2c2c;
      --sidebar-bg: #202020;
      --text-color: #f5f5f5;
      --active-btn-bg: #383838;
    }
  }

  :global(.dark) {
    --bg-app: #202020;
    --bg-sidebar: #181818;
    --bg-card: #2c2c2c;
    --bg-card-hover: #323232;
    --bg-card-subtle: #242424;
    --text-primary: #f5f5f5;
    --text-secondary: #cccccc;
    --text-muted: #8e8e8e;
    --border-color: #383838;
    --border-subtle: #2e2e2e;
    --accent: #60cdff;
    --accent-hover: #4cc2fc;
    --accent-light: rgba(96, 205, 255, 0.15);
    --accent-text: #000000;
    --item-active-bg: #333333;
    --item-hover-bg: #2b2b2b;
    --input-bg: #2b2b2b;
    --input-border: #444444;
    --input-focus-border: #60cdff;
    --shadow-sm: 0 1px 4px rgba(0, 0, 0, 0.25);
    --shadow-md: 0 4px 16px rgba(0, 0, 0, 0.35);
    --badge-bg: #383838;
    --badge-text: #cccccc;
    --success-bg: rgba(16, 124, 65, 0.2);
    --success-border: #6ccb5f;
    --success-text: #9fe597;
    --bar-bg: #3a3a3a;
    --bar-fill: #60cdff;

    --card-bg: #2c2c2c;
    --sidebar-bg: #202020;
    --text-color: #f5f5f5;
    --active-btn-bg: #383838;
  }

  :global(*),
  :global(*::before),
  :global(*::after) {
    box-sizing: border-box;
  }

  :global(body) {
    margin: 0;
    padding: 0;
    font-family: var(--font-family);
    background-color: var(--bg-app);
    color: var(--text-primary);
    -webkit-font-smoothing: antialiased;
    overflow: hidden;
    user-select: none;
  }

  :global(button) {
    font-family: inherit;
  }

  :global(input, select, textarea) {
    font-family: inherit;
    color: inherit;
  }

  .shell {
    display: flex;
    height: 100vh;
    width: 100vw;
    background-color: var(--bg-app);
    color: var(--text-primary);
  }

  .sidebar {
    width: 220px;
    min-width: 200px;
    background-color: var(--bg-sidebar);
    border-right: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    padding: 1.25rem 0.85rem 0.85rem 0.85rem;
    gap: 0.5rem;
  }

  .brand-row {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    padding: 0 0.5rem 1rem 0.5rem;
    border-bottom: 1px solid var(--border-subtle);
    margin-bottom: 0.5rem;
  }

  .brand-icon {
    width: 28px;
    height: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--accent);
  }

  .brand-icon svg {
    width: 24px;
    height: 24px;
  }

  .brand-text {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .brand-title {
    font-size: 1.15rem;
    font-weight: 700;
    letter-spacing: -0.01em;
  }

  .mock-badge {
    font-size: 0.65rem;
    font-weight: 700;
    padding: 0.15rem 0.4rem;
    border-radius: var(--radius-sm);
    background-color: var(--badge-bg);
    color: var(--badge-text);
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .nav-links {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    flex: 1;
  }

  .nav-item {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    width: 100%;
    padding: 0.65rem 0.85rem;
    background: transparent;
    border: none;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: 0.92rem;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
    transition: background-color 0.15s ease, color 0.15s ease;
  }

  .nav-item:hover {
    background-color: var(--item-hover-bg);
    color: var(--text-primary);
  }

  .nav-item.active {
    background-color: var(--item-active-bg);
    color: var(--text-primary);
    font-weight: 600;
    position: relative;
  }

  .nav-item.active::before {
    content: '';
    position: absolute;
    left: 0;
    top: 20%;
    bottom: 20%;
    width: 3px;
    background-color: var(--accent);
    border-radius: 2px;
  }

  .nav-icon {
    width: 18px;
    height: 18px;
    stroke-width: 2.2;
    flex-shrink: 0;
  }

  .sidebar-footer {
    padding-top: 0.75rem;
    border-top: 1px solid var(--border-subtle);
  }

  .theme-indicator {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.75rem;
    color: var(--text-muted);
    padding: 0.4rem 0.5rem;
  }

  .theme-icon {
    width: 14px;
    height: 14px;
    flex-shrink: 0;
  }

  .content-area {
    flex: 1;
    height: 100vh;
    overflow-y: auto;
    background-color: var(--bg-app);
  }
</style>
