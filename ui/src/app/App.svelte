<script lang="ts">
  import SitesView from '../views/sites/View.svelte';
  import DailyView from '../views/daily/View.svelte';
  import SettingsView from '../views/settings/View.svelte';

  type Tab = 'daily' | 'sites' | 'settings';
  let activeTab: Tab = 'daily';
</script>

<div class="shell">
  <nav class="sidebar">
    <div class="brand">Tally</div>
    <button class:active={activeTab === 'daily'} on:click={() => activeTab = 'daily'}>
      Daily
    </button>
    <button class:active={activeTab === 'sites'} on:click={() => activeTab = 'sites'}>
      Sites
    </button>
    <button class:active={activeTab === 'settings'} on:click={() => activeTab = 'settings'}>
      Settings
    </button>
  </nav>

  <main class="content">
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
    --bg-color: #ffffff;
    --text-color: #1a1a1a;
    --sidebar-bg: #f5f5f5;
    --border-color: #e0e0e0;
    --active-btn-bg: #e5e5e5;
  }

  @media (prefers-color-scheme: dark) {
    :global(:root) {
      --bg-color: #1e1e1e;
      --text-color: #f0f0f0;
      --sidebar-bg: #252526;
      --border-color: #383838;
      --active-btn-bg: #37373d;
    }
  }

  :global(body) {
    margin: 0;
    padding: 0;
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;
    background-color: var(--bg-color);
    color: var(--text-color);
  }

  .shell {
    display: flex;
    height: 100vh;
    width: 100vw;
    overflow: hidden;
  }

  .sidebar {
    width: 200px;
    background-color: var(--sidebar-bg);
    border-right: 1px solid var(--border-color);
    display: flex;
    flex-direction: column;
    padding: 1rem;
    gap: 0.5rem;
  }

  .brand {
    font-size: 1.25rem;
    font-weight: bold;
    margin-bottom: 1rem;
  }

  button {
    background: transparent;
    border: none;
    padding: 0.6rem 1rem;
    text-align: left;
    font-size: 0.95rem;
    color: var(--text-color);
    border-radius: 4px;
    cursor: pointer;
  }

  button:hover {
    background-color: var(--active-btn-bg);
  }

  button.active {
    background-color: var(--active-btn-bg);
    font-weight: 600;
  }

  .content {
    flex: 1;
    overflow-y: auto;
  }
</style>
