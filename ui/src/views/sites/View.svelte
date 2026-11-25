<script lang="ts">
  import { onMount } from 'svelte';
  import {
    get_sites,
    search,
    type SiteItem,
    type Hit
  } from '../../app/api';
  import { formatDuration, formatPercent } from '../../app/formatters';

  type SearchMode = 'sites' | 'titles' | 'both';

  let sites: SiteItem[] = [];
  let loadingSites = true;
  let sitesError: string | null = null;

  // Search state
  let searchQuery = '';
  let searchMode: SearchMode = 'both';
  let selectedSiteDomain: string | null = null;
  let isSearching = false;
  let searchHits: Hit[] = [];
  let searchError: string | null = null;
  let hasSearched = false;

  // Expansion tracking
  let expandedDomains = new Set<string>();

  // Date filters (local YYYY-MM-DD)
  function localDateISO(d: Date): string {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }
  const today = localDateISO(new Date());
  let fromDate = today;
  let toDate = today;

  // Debounce handle for search input
  let searchDebounceTimer: any = null;

  onMount(() => {
    loadSites();
  });

  async function loadSites() {
    loadingSites = true;
    sitesError = null;
    try {
      const data = await get_sites(fromDate, toDate);
      // Sort sites by secs descending and sort their pages by secs descending
      sites = (data || [])
        .map(site => ({
          ...site,
          pages: [...(site.pages || [])].sort((a, b) => b.secs - a.secs)
        }))
        .sort((a, b) => b.secs - a.secs);
    } catch (err: any) {
      sitesError = err?.message || String(err) || 'Failed to load sites';
    } finally {
      loadingSites = false;
    }
  }

  function handleSearchInput() {
    clearTimeout(searchDebounceTimer);
    searchDebounceTimer = setTimeout(() => {
      runSearch();
    }, 250);
  }

  function handleModeChange(mode: SearchMode) {
    if (searchMode !== mode) {
      searchMode = mode;
      if (searchQuery.trim()) {
        runSearch();
      }
    }
  }

  async function runSearch() {
    const q = searchQuery.trim();
    if (!q) {
      hasSearched = false;
      searchHits = [];
      searchError = null;
      return;
    }

    isSearching = true;
    searchError = null;
    hasSearched = true;

    try {
      // If a site is selected, title search is scoped to that site
      // Passing selectedSiteDomain when a site is selected
      const hits = await search(
        q,
        searchMode,
        selectedSiteDomain,
        fromDate,
        toDate
      );
      searchHits = hits || [];
    } catch (err: any) {
      searchError = err?.message || String(err) || 'Search failed';
      searchHits = [];
    } finally {
      isSearching = false;
    }
  }

  function clearSearch() {
    searchQuery = '';
    hasSearched = false;
    searchHits = [];
    searchError = null;
  }

  function toggleExpand(domain: string) {
    const next = new Set(expandedDomains);
    if (next.has(domain)) {
      next.delete(domain);
    } else {
      next.add(domain);
    }
    expandedDomains = next;
  }

  function expandAll() {
    expandedDomains = new Set(sites.map(s => s.domain));
  }

  function collapseAll() {
    expandedDomains = new Set();
  }

  function toggleScopeSite(domain: string) {
    if (selectedSiteDomain === domain) {
      selectedSiteDomain = null;
    } else {
      selectedSiteDomain = domain;
    }

    if (searchQuery.trim()) {
      runSearch();
    }
  }

  function clearScope() {
    selectedSiteDomain = null;
    if (searchQuery.trim()) {
      runSearch();
    }
  }

  // Derived metrics
  $: totalSiteSecs = sites.reduce((sum, s) => sum + s.secs, 0);
  $: maxSiteSecs = sites.length > 0 ? Math.max(...sites.map(s => s.secs)) : 1;
</script>

<div class="sites-container">
  <!-- Top Header -->
  <header class="view-header">
    <div class="title-section">
      <h1 class="view-title">Sites</h1>
      <p class="view-subtitle">
        Web time breakdown by domain and page.
      </p>
    </div>

    {#if !loadingSites && sites.length > 0 && !hasSearched}
      <div class="header-stats">
        <span class="stat-pill">
          <strong>{sites.length}</strong> {sites.length === 1 ? 'site' : 'sites'}
        </span>
        <span class="stat-pill highlight">
          <strong>{formatDuration(totalSiteSecs)}</strong> total
        </span>
      </div>
    {/if}
  </header>

  <!-- Search Control Bar -->
  <section class="search-panel" aria-label="Search controls">
    <div class="search-row">
      <!-- Search Input Box -->
      <div class="search-input-wrapper">
        <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="11" cy="11" r="8" />
          <line x1="21" y1="21" x2="16.65" y2="16.65" />
        </svg>

        <input
          type="text"
          class="search-input"
          placeholder={selectedSiteDomain
            ? `Search pages on ${selectedSiteDomain}...`
            : 'Search sites or page titles...'}
          bind:value={searchQuery}
          on:input={handleSearchInput}
          on:keydown={(e) => e.key === 'Enter' && runSearch()}
          aria-label="Search sites or titles"
        />

        {#if searchQuery}
          <button
            type="button"
            class="clear-btn"
            on:click={clearSearch}
            title="Clear search"
            aria-label="Clear search"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <line x1="18" y1="6" x2="6" y2="18" />
              <line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        {/if}
      </div>

      <!-- Mode Switch: Sites / Titles / Both -->
      <div class="mode-switch-group" role="radiogroup" aria-label="Search mode">
        <button
          type="button"
          class="mode-btn"
          class:active={searchMode === 'sites'}
          on:click={() => handleModeChange('sites')}
          aria-checked={searchMode === 'sites'}
          role="radio"
        >
          Sites
        </button>
        <button
          type="button"
          class="mode-btn"
          class:active={searchMode === 'titles'}
          on:click={() => handleModeChange('titles')}
          aria-checked={searchMode === 'titles'}
          role="radio"
        >
          Titles
        </button>
        <button
          type="button"
          class="mode-btn"
          class:active={searchMode === 'both'}
          on:click={() => handleModeChange('both')}
          aria-checked={searchMode === 'both'}
          role="radio"
        >
          Both
        </button>
      </div>
    </div>

    <!-- Active Site Scope Indicator -->
    {#if selectedSiteDomain}
      <div class="scope-banner">
        <div class="scope-info">
          <svg class="scope-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <polygon points="22 3 2 3 10 12.46 10 19 14 21 14 12.46 22 3" />
          </svg>
          <span>
            Title search scoped to: <strong>{selectedSiteDomain}</strong>
          </span>
        </div>
        <button
          type="button"
          class="clear-scope-btn"
          on:click={clearScope}
          title="Remove site filter"
        >
          Clear scope
        </button>
      </div>
    {/if}
  </section>

  <!-- Content Section: Search Results OR Sites List -->
  <main class="view-body">
    {#if isSearching}
      <div class="state-message">
        <div class="spinner"></div>
        <p>Searching...</p>
      </div>
    {:else if hasSearched}
      <!-- Search Results View -->
      <section class="results-section">
        <div class="results-header">
          <h2 class="section-title">
            Search Results for "{searchQuery}"
            <span class="count-badge">{searchHits.length}</span>
          </h2>
          <button type="button" class="text-action-btn" on:click={clearSearch}>
            Back to all sites
          </button>
        </div>

        {#if searchError}
          <div class="error-banner">
            <span>{searchError}</span>
          </div>
        {:else if searchHits.length === 0}
          <div class="state-message empty">
            <svg class="empty-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <circle cx="11" cy="11" r="8" />
              <line x1="21" y1="21" x2="16.65" y2="16.65" />
            </svg>
            <h3>No matches found</h3>
            <p>Try adjusting your search query, mode, or removing site scoping.</p>
          </div>
        {:else}
          <div class="hits-list">
            {#each searchHits as hit, i (i)}
              <article class="hit-card">
                <div class="hit-main">
                  <div class="hit-meta-row">
                    <span class="kind-tag" class:page={hit.kind === 'page'}>
                      {hit.kind.toUpperCase()}
                    </span>
                    <span class="hit-domain">{hit.domain}</span>
                    {#if hit.score !== undefined && hit.score !== null}
                      <span class="score-badge" title={`Match score: ${hit.score}`}>
                        Score {Math.round(hit.score * 100)}%
                      </span>
                    {/if}
                  </div>

                  {#if hit.title}
                    <h3 class="hit-title">{hit.title}</h3>
                  {/if}

                  {#if hit.url}
                    <a
                      href={hit.url}
                      target="_blank"
                      rel="noreferrer"
                      class="hit-url"
                      title={hit.url}
                    >
                      {hit.url}
                    </a>
                  {/if}
                </div>

                <div class="hit-aside">
                  <div class="hit-duration">{formatDuration(hit.secs)}</div>
                  {#if hit.kind === 'site'}
                    <button
                      type="button"
                      class="scope-chip-btn"
                      class:active={selectedSiteDomain === hit.domain}
                      on:click={() => toggleScopeSite(hit.domain)}
                    >
                      {selectedSiteDomain === hit.domain ? 'Scoped' : 'Scope title search'}
                    </button>
                  {/if}
                </div>
              </article>
            {/each}
          </div>
        {/if}
      </section>
    {:else}
      <!-- Standard Sites View -->
      <section class="sites-section">
        <div class="sites-toolbar">
          <div class="toolbar-left">
            <h2 class="section-title">All Sites</h2>
            <span class="subtext">Sorted by time descending</span>
          </div>

          {#if sites.length > 0}
            <div class="toolbar-actions">
              <button
                type="button"
                class="utility-btn"
                on:click={expandAll}
                title="Expand all site pages"
              >
                Expand all
              </button>
              <button
                type="button"
                class="utility-btn"
                on:click={collapseAll}
                title="Collapse all site pages"
              >
                Collapse all
              </button>
            </div>
          {/if}
        </div>

        {#if loadingSites}
          <div class="state-message">
            <div class="spinner"></div>
            <p>Loading sites...</p>
          </div>
        {:else if sitesError}
          <div class="error-banner">
            <span>{sitesError}</span>
            <button type="button" class="retry-btn" on:click={loadSites}>Retry</button>
          </div>
        {:else if sites.length === 0}
          <div class="state-message empty">
            <svg class="empty-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <circle cx="12" cy="12" r="10" />
              <line x1="2" y1="12" x2="22" y2="12" />
              <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z" />
            </svg>
            <h3>No site activity recorded</h3>
            <p>Sites and pages will appear here once web activity is detected.</p>
          </div>
        {:else}
          <div class="sites-list">
            {#each sites as site (site.domain)}
              {@const isExpanded = expandedDomains.has(site.domain)}
              {@const isScoped = selectedSiteDomain === site.domain}
              {@const sitePercent = formatPercent(site.secs, maxSiteSecs)}

              <article class="site-card" class:scoped={isScoped} class:expanded={isExpanded}>
                <div class="site-row">
                  <!-- Expansion Chevron -->
                  <button
                    type="button"
                    class="chevron-toggle"
                    class:expanded={isExpanded}
                    on:click={() => toggleExpand(site.domain)}
                    aria-label={isExpanded ? `Collapse ${site.domain}` : `Expand ${site.domain}`}
                    aria-expanded={isExpanded}
                  >
                    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5">
                      <polyline points="9 18 15 12 9 6" />
                    </svg>
                  </button>

                  <!-- Site Main Info -->
                  <div
                    class="site-info"
                    role="button"
                    tabindex="0"
                    on:click={() => toggleExpand(site.domain)}
                    on:keydown={(e) => e.key === 'Enter' && toggleExpand(site.domain)}
                  >
                    <div class="site-name-row">
                      <span class="site-domain">{site.domain}</span>
                      {#if isScoped}
                        <span class="active-scope-badge">Scoped for search</span>
                      {/if}
                      <span class="pages-count">
                        {(site.pages || []).length} {(site.pages || []).length === 1 ? 'page' : 'pages'}
                      </span>
                    </div>

                    <!-- Proportion Bar -->
                    <div class="bar-container" title={`${sitePercent}% of top site time`}>
                      <div class="bar-fill" style={`width: ${sitePercent}%;`}></div>
                    </div>
                  </div>

                  <!-- Right Side: Duration & Actions -->
                  <div class="site-actions-wrapper">
                    <div class="site-time">
                      <span class="time-main">{formatDuration(site.secs)}</span>
                    </div>

                    <button
                      type="button"
                      class="scope-btn"
                      class:active={isScoped}
                      on:click|stopPropagation={() => toggleScopeSite(site.domain)}
                      title={isScoped ? 'Remove search scoping' : `Scope title search to ${site.domain}`}
                    >
                      {isScoped ? 'Scoped' : 'Scope'}
                    </button>
                  </div>
                </div>

                <!-- Expanded Pages List -->
                {#if isExpanded}
                  <div class="pages-drawer">
                    {#if !site.pages || site.pages.length === 0}
                      <div class="no-pages-note">No individual page details available for this site.</div>
                    {:else}
                      <ul class="pages-list">
                        {#each site.pages as page (page.url + page.title)}
                          {@const pagePercent = formatPercent(page.secs, site.secs)}
                          <li class="page-item">
                            <div class="page-details">
                              <div class="page-title-row">
                                <span class="page-title" title={page.title}>{page.title || '(No title)'}</span>
                              </div>
                              <a
                                href={page.url}
                                target="_blank"
                                rel="noreferrer"
                                class="page-url"
                                title={page.url}
                              >
                                {page.url}
                              </a>
                              <div class="page-bar-container" title={`${pagePercent}% of ${site.domain} time`}>
                                <div class="page-bar-fill" style={`width: ${pagePercent}%;`}></div>
                              </div>
                            </div>

                            <div class="page-stats">
                              <span class="page-time">{formatDuration(page.secs)}</span>
                              <span class="page-share">{pagePercent}%</span>
                            </div>
                          </li>
                        {/each}
                      </ul>
                    {/if}
                  </div>
                {/if}
              </article>
            {/each}
          </div>
        {/if}
      </section>
    {/if}
  </main>
</div>

<style>
  .sites-container {
    padding: 1.75rem 2rem;
    max-width: 1040px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  /* Header */
  .view-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-end;
    flex-wrap: wrap;
    gap: 1rem;
    padding-bottom: 0.5rem;
  }

  .view-title {
    font-size: 1.6rem;
    font-weight: 700;
    margin: 0 0 0.25rem 0;
    color: var(--text-primary);
    letter-spacing: -0.02em;
  }

  .view-subtitle {
    font-size: 0.9rem;
    color: var(--text-secondary);
    margin: 0;
  }

  .header-stats {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  .stat-pill {
    font-size: 0.8rem;
    background-color: var(--bg-card);
    border: 1px solid var(--border-color);
    padding: 0.35rem 0.65rem;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
  }

  .stat-pill.highlight {
    background-color: var(--accent-light);
    color: var(--accent);
    border-color: transparent;
  }

  /* Search Panel */
  .search-panel {
    background-color: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 0.85rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    box-shadow: var(--shadow-sm);
  }

  .search-row {
    display: flex;
    gap: 0.75rem;
    align-items: center;
    flex-wrap: wrap;
  }

  .search-input-wrapper {
    position: relative;
    flex: 1;
    min-width: 240px;
    display: flex;
    align-items: center;
  }

  .search-icon {
    position: absolute;
    left: 0.75rem;
    width: 16px;
    height: 16px;
    color: var(--text-muted);
    pointer-events: none;
  }

  .search-input {
    width: 100%;
    padding: 0.55rem 2.2rem 0.55rem 2.25rem;
    background-color: var(--input-bg);
    border: 1px solid var(--input-border);
    border-radius: var(--radius-md);
    font-size: 0.9rem;
    outline: none;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .search-input:focus {
    border-color: var(--input-focus-border);
    box-shadow: 0 0 0 2px var(--accent-light);
  }

  .clear-btn {
    position: absolute;
    right: 0.6rem;
    background: transparent;
    border: none;
    cursor: pointer;
    color: var(--text-muted);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0.2rem;
    border-radius: var(--radius-sm);
  }

  .clear-btn:hover {
    color: var(--text-primary);
  }

  .clear-btn svg {
    width: 14px;
    height: 14px;
  }

  /* Mode Switch Group */
  .mode-switch-group {
    display: flex;
    background-color: var(--item-active-bg);
    padding: 2px;
    border-radius: var(--radius-md);
    gap: 2px;
  }

  .mode-btn {
    padding: 0.45rem 0.85rem;
    font-size: 0.82rem;
    font-weight: 500;
    border: none;
    background: transparent;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.12s ease;
  }

  .mode-btn:hover {
    color: var(--text-primary);
  }

  .mode-btn.active {
    background-color: var(--bg-card);
    color: var(--text-primary);
    font-weight: 600;
    box-shadow: var(--shadow-sm);
  }

  /* Scope Banner */
  .scope-banner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    background-color: var(--accent-light);
    padding: 0.45rem 0.75rem;
    border-radius: var(--radius-md);
    font-size: 0.82rem;
    color: var(--text-primary);
  }

  .scope-info {
    display: flex;
    align-items: center;
    gap: 0.45rem;
  }

  .scope-icon {
    width: 14px;
    height: 14px;
    color: var(--accent);
  }

  .clear-scope-btn {
    background: transparent;
    border: none;
    color: var(--accent);
    font-weight: 600;
    font-size: 0.8rem;
    cursor: pointer;
    padding: 0.15rem 0.4rem;
    border-radius: var(--radius-sm);
  }

  .clear-scope-btn:hover {
    text-decoration: underline;
  }

  /* Body & Toolbars */
  .view-body {
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  .sites-toolbar,
  .results-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.75rem;
  }

  .section-title {
    font-size: 1.15rem;
    font-weight: 600;
    margin: 0;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .subtext {
    font-size: 0.8rem;
    color: var(--text-muted);
  }

  .count-badge {
    font-size: 0.75rem;
    padding: 0.15rem 0.45rem;
    border-radius: var(--radius-sm);
    background-color: var(--badge-bg);
    color: var(--badge-text);
  }

  .toolbar-actions {
    display: flex;
    gap: 0.4rem;
  }

  .utility-btn,
  .text-action-btn {
    background-color: var(--bg-card);
    border: 1px solid var(--border-color);
    padding: 0.35rem 0.65rem;
    font-size: 0.8rem;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .utility-btn:hover,
  .text-action-btn:hover {
    background-color: var(--item-hover-bg);
    color: var(--text-primary);
  }

  .text-action-btn {
    color: var(--accent);
    font-weight: 500;
    border-color: transparent;
    background: transparent;
  }

  .text-action-btn:hover {
    background-color: var(--accent-light);
  }

  /* Sites List */
  .sites-list {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }

  .site-card {
    background-color: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    overflow: hidden;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
    box-shadow: var(--shadow-sm);
  }

  .site-card:hover {
    border-color: var(--input-focus-border);
  }

  .site-card.scoped {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }

  .site-row {
    display: flex;
    align-items: center;
    padding: 0.85rem 1rem;
    gap: 0.85rem;
  }

  .chevron-toggle {
    background: transparent;
    border: none;
    cursor: pointer;
    padding: 0.35rem;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--text-muted);
    border-radius: var(--radius-sm);
    transition: transform 0.18s ease, color 0.15s ease;
  }

  .chevron-toggle:hover {
    color: var(--text-primary);
    background-color: var(--item-hover-bg);
  }

  .chevron-toggle svg {
    width: 16px;
    height: 16px;
  }

  .chevron-toggle.expanded svg {
    transform: rotate(90deg);
  }

  .site-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    cursor: pointer;
  }

  .site-name-row {
    display: flex;
    align-items: center;
    gap: 0.65rem;
    flex-wrap: wrap;
  }

  .site-domain {
    font-size: 0.98rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .active-scope-badge {
    font-size: 0.68rem;
    font-weight: 600;
    padding: 0.1rem 0.4rem;
    border-radius: var(--radius-sm);
    background-color: var(--accent-light);
    color: var(--accent);
  }

  .pages-count {
    font-size: 0.78rem;
    color: var(--text-muted);
  }

  .bar-container {
    width: 100%;
    max-width: 320px;
    height: 5px;
    background-color: var(--bar-bg);
    border-radius: 3px;
    overflow: hidden;
  }

  .bar-fill {
    height: 100%;
    background-color: var(--bar-fill);
    border-radius: 3px;
    transition: width 0.3s ease;
  }

  .site-actions-wrapper {
    display: flex;
    align-items: center;
    gap: 0.85rem;
  }

  .site-time {
    text-align: right;
  }

  .time-main {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .scope-btn {
    padding: 0.35rem 0.65rem;
    font-size: 0.78rem;
    font-weight: 500;
    border: 1px solid var(--border-color);
    background-color: var(--bg-card);
    color: var(--text-secondary);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.15s ease;
  }

  .scope-btn:hover {
    background-color: var(--item-hover-bg);
    color: var(--text-primary);
  }

  .scope-btn.active {
    background-color: var(--accent);
    color: var(--accent-text);
    border-color: var(--accent);
  }

  /* Pages Drawer */
  .pages-drawer {
    background-color: var(--bg-card-subtle);
    border-top: 1px solid var(--border-color);
    padding: 0.75rem 1rem 0.75rem 3.25rem;
  }

  .no-pages-note {
    font-size: 0.82rem;
    color: var(--text-muted);
    font-style: italic;
    padding: 0.4rem 0;
  }

  .pages-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
  }

  .page-item {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    padding: 0.45rem 0;
    border-bottom: 1px solid var(--border-subtle);
  }

  .page-item:last-child {
    border-bottom: none;
  }

  .page-details {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    min-width: 0;
  }

  .page-title {
    font-size: 0.86rem;
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
  }

  .page-url {
    font-size: 0.76rem;
    color: var(--accent);
    text-decoration: none;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    display: block;
  }

  .page-url:hover {
    text-decoration: underline;
  }

  .page-bar-container {
    width: 100%;
    max-width: 220px;
    height: 4px;
    background-color: var(--bar-bg);
    border-radius: 2px;
    overflow: hidden;
    margin-top: 0.25rem;
  }

  .page-bar-fill {
    height: 100%;
    background-color: var(--accent);
    opacity: 0.75;
    border-radius: 2px;
  }

  .page-stats {
    text-align: right;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    flex-shrink: 0;
  }

  .page-time {
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .page-share {
    font-size: 0.72rem;
    color: var(--text-muted);
  }

  /* Search Hits */
  .hits-list {
    display: flex;
    flex-direction: column;
    gap: 0.65rem;
  }

  .hit-card {
    background-color: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 0.85rem 1.1rem;
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    box-shadow: var(--shadow-sm);
  }

  .hit-main {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }

  .hit-meta-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .kind-tag {
    font-size: 0.65rem;
    font-weight: 700;
    padding: 0.15rem 0.4rem;
    border-radius: var(--radius-sm);
    background-color: var(--badge-bg);
    color: var(--badge-text);
  }

  .kind-tag.page {
    background-color: var(--accent-light);
    color: var(--accent);
  }

  .hit-domain {
    font-size: 0.82rem;
    font-weight: 600;
    color: var(--text-secondary);
  }

  .score-badge {
    font-size: 0.7rem;
    color: var(--text-muted);
  }

  .hit-title {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-primary);
    margin: 0;
  }

  .hit-url {
    font-size: 0.78rem;
    color: var(--accent);
    text-decoration: none;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .hit-url:hover {
    text-decoration: underline;
  }

  .hit-aside {
    text-align: right;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 0.4rem;
    flex-shrink: 0;
  }

  .hit-duration {
    font-size: 0.95rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .scope-chip-btn {
    font-size: 0.75rem;
    padding: 0.3rem 0.55rem;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border-color);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .scope-chip-btn:hover {
    background-color: var(--item-hover-bg);
    color: var(--text-primary);
  }

  .scope-chip-btn.active {
    background-color: var(--accent);
    color: var(--accent-text);
  }

  /* Utility & State messages */
  .state-message {
    padding: 3rem 1rem;
    text-align: center;
    color: var(--text-muted);
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
  }

  .empty-icon {
    width: 42px;
    height: 42px;
    stroke-width: 1.5;
    color: var(--text-muted);
  }

  .state-message h3 {
    margin: 0;
    font-size: 1.1rem;
    color: var(--text-primary);
  }

  .state-message p {
    margin: 0;
    font-size: 0.88rem;
  }

  .spinner {
    width: 24px;
    height: 24px;
    border: 2.5px solid var(--border-color);
    border-top-color: var(--accent);
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  .error-banner {
    padding: 0.75rem 1rem;
    background-color: #fde8e8;
    color: #9b1c1c;
    border-radius: var(--radius-md);
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 0.88rem;
  }

  .retry-btn {
    padding: 0.25rem 0.6rem;
    background: transparent;
    border: 1px solid #9b1c1c;
    color: #9b1c1c;
    border-radius: var(--radius-sm);
    cursor: pointer;
  }
</style>
