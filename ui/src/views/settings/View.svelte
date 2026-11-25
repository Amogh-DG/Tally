<script lang="ts">
  import { onMount } from 'svelte';
  import {
    get_settings,
    set_settings,
    export_json,
    type Settings
  } from '../../app/api';

  let currentSettings: Settings | null = null;
  let loading = true;
  let saving = false;
  let errorMsg: string | null = null;
  let successMsg: string | null = null;
  let successTimer: any = null;

  // Form model
  let afkTimeoutSecs: number = 180;
  let dayStart: string = '00:00';
  let port: number = 5600;
  let dataDir: string = '%APPDATA%\\Tally\\data';
  let extensionOrigins: string[] = [];

  // Validation errors
  let validationErrors: { afk?: string; dayStart?: string } = {};

  // JSON Export state
  function localDateISO(d: Date): string {
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  }
  let exportFrom = localDateISO(new Date());
  let exportTo = localDateISO(new Date());
  let exportPath = `tidemark-export-${exportTo}.json`;
  let exporting = false;
  let exportResultMsg: string | null = null;
  let exportErrorMsg: string | null = null;

  async function handleExport() {
    exporting = true;
    exportResultMsg = null;
    exportErrorMsg = null;
    try {
      const written = await export_json(exportFrom, exportTo, exportPath.trim());
      exportResultMsg = `Exported JSON to ${written}`;
    } catch (err: any) {
      exportErrorMsg = err?.message || String(err) || 'Export failed';
    } finally {
      exporting = false;
    }
  }

  onMount(() => {
    loadSettings();
  });

  async function loadSettings() {
    loading = true;
    errorMsg = null;
    try {
      const data = await get_settings();
      currentSettings = data;
      afkTimeoutSecs = data.afk_timeout_secs;
      dayStart = data.day_start || '00:00';
      port = data.port || 5600;
      dataDir = data.data_dir || '%APPDATA%\\Tally\\data';
      extensionOrigins = data.extension_origins || [];
    } catch (err: any) {
      errorMsg = err?.message || String(err) || 'Failed to load settings';
    } finally {
      loading = false;
    }
  }

  function validate(): boolean {
    const errors: { afk?: string; dayStart?: string } = {};

    if (!afkTimeoutSecs || afkTimeoutSecs < 5 || isNaN(afkTimeoutSecs)) {
      errors.afk = 'AFK timeout must be at least 5 seconds.';
    } else if (afkTimeoutSecs > 86400) {
      errors.afk = 'AFK timeout cannot exceed 24 hours (86,400 seconds).';
    }

    const timeRegex = /^([01]\d|2[0-3]):([0-5]\d)$/;
    if (!dayStart || !timeRegex.test(dayStart.trim())) {
      errors.dayStart = 'Start of day must be in HH:MM 24-hour format (e.g. 00:00 or 04:00).';
    }

    validationErrors = errors;
    return Object.keys(errors).length === 0;
  }

  async function handleSave() {
    if (!validate()) {
      return;
    }

    saving = true;
    errorMsg = null;
    successMsg = null;
    clearTimeout(successTimer);

    try {
      const payload: Settings = {
        afk_timeout_secs: Math.round(Number(afkTimeoutSecs)),
        day_start: dayStart.trim(),
        port: port,
        extension_origins: extensionOrigins,
        data_dir: dataDir
      };

      const result = await set_settings(payload);

      // Support mock round-trip persistence by syncing the referenced object
      if (currentSettings) {
        Object.assign(currentSettings, result);
      } else {
        currentSettings = result;
      }

      afkTimeoutSecs = result.afk_timeout_secs;
      dayStart = result.day_start;

      successMsg = `Settings saved successfully (AFK: ${result.afk_timeout_secs}s, Day start: ${result.day_start})`;
      successTimer = setTimeout(() => {
        successMsg = null;
      }, 5000);
    } catch (err: any) {
      errorMsg = err?.message || String(err) || 'Failed to save settings';
    } finally {
      saving = false;
    }
  }

  function resetToDefaults() {
    afkTimeoutSecs = 180;
    dayStart = '00:00';
    validationErrors = {};
  }

  function discardChanges() {
    if (currentSettings) {
      afkTimeoutSecs = currentSettings.afk_timeout_secs;
      dayStart = currentSettings.day_start;
      validationErrors = {};
    }
  }

  $: hasChanges =
    currentSettings !== null &&
    (afkTimeoutSecs !== currentSettings.afk_timeout_secs ||
      dayStart !== currentSettings.day_start);
</script>

<div class="settings-container">
  <header class="settings-header">
    <div class="title-section">
      <h1 class="view-title">Settings</h1>
      <p class="view-subtitle">
        Configure tracking parameters, AFK detection, and daily roll-over.
      </p>
    </div>
  </header>

  <!-- Notification banners -->
  {#if successMsg}
    <div class="banner success" role="status" aria-live="polite">
      <div class="banner-content">
        <svg class="banner-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
          <polyline points="22 4 12 14.01 9 11.01" />
        </svg>
        <span>{successMsg}</span>
      </div>
      <button
        type="button"
        class="banner-close"
        on:click={() => (successMsg = null)}
        aria-label="Dismiss message"
      >
        ✕
      </button>
    </div>
  {/if}

  {#if errorMsg}
    <div class="banner error" role="alert">
      <div class="banner-content">
        <svg class="banner-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <circle cx="12" cy="12" r="10" />
          <line x1="12" y1="8" x2="12" y2="12" />
          <line x1="12" y1="16" x2="12.01" y2="16" />
        </svg>
        <span>{errorMsg}</span>
      </div>
      <button
        type="button"
        class="banner-close"
        on:click={() => (errorMsg = null)}
        aria-label="Dismiss error"
      >
        ✕
      </button>
    </div>
  {/if}

  {#if loading}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>Loading settings...</p>
    </div>
  {:else}
    <form class="settings-form" on:submit|preventDefault={handleSave}>
      <!-- Section 1: AFK Timeout -->
      <section class="card-section">
        <div class="card-header">
          <div class="section-badge">1</div>
          <div class="header-text">
            <h2 class="card-title">AFK Timeout</h2>
            <p class="card-description">
              The duration of user inactivity before Tally marks the state as AFK. Window and website tracking are paused while AFK.
            </p>
          </div>
        </div>

        <div class="field-group">
          <label for="afk-timeout" class="field-label">
            Timeout in seconds
          </label>
          <div class="input-with-presets">
            <div class="input-unit-wrapper">
              <input
                id="afk-timeout"
                type="number"
                min="5"
                max="86400"
                step="1"
                class="text-input"
                class:has-error={!!validationErrors.afk}
                bind:value={afkTimeoutSecs}
                on:input={() => {
                  if (validationErrors.afk) validate();
                }}
              />
              <span class="unit-label">sec</span>
            </div>

            <!-- Quick Presets -->
            <div class="preset-chips">
              <button
                type="button"
                class="chip-btn"
                class:selected={afkTimeoutSecs === 60}
                on:click={() => {
                  afkTimeoutSecs = 60;
                  validate();
                }}
              >
                1 min (60s)
              </button>
              <button
                type="button"
                class="chip-btn"
                class:selected={afkTimeoutSecs === 180}
                on:click={() => {
                  afkTimeoutSecs = 180;
                  validate();
                }}
              >
                3 min (default)
              </button>
              <button
                type="button"
                class="chip-btn"
                class:selected={afkTimeoutSecs === 300}
                on:click={() => {
                  afkTimeoutSecs = 300;
                  validate();
                }}
              >
                5 min (300s)
              </button>
              <button
                type="button"
                class="chip-btn"
                class:selected={afkTimeoutSecs === 600}
                on:click={() => {
                  afkTimeoutSecs = 600;
                  validate();
                }}
              >
                10 min (600s)
              </button>
            </div>
          </div>

          {#if validationErrors.afk}
            <span class="field-error">{validationErrors.afk}</span>
          {:else}
            <span class="field-hint">
              Current setting: {Math.floor(afkTimeoutSecs / 60)} min {afkTimeoutSecs % 60} sec
            </span>
          {/if}
        </div>
      </section>

      <!-- Section 2: Start of Day -->
      <section class="card-section">
        <div class="card-header">
          <div class="section-badge">2</div>
          <div class="header-text">
            <h2 class="card-title">Start of Day</h2>
            <p class="card-description">
              The hour when a new tracking day begins. If you frequently work past midnight, set this to 04:00 to keep late-night work in the previous day's tally.
            </p>
          </div>
        </div>

        <div class="field-group">
          <label for="day-start" class="field-label">
            Day transition time (24-hour format HH:MM)
          </label>
          <div class="input-with-presets">
            <div class="input-unit-wrapper">
              <input
                id="day-start"
                type="text"
                placeholder="00:00"
                maxlength="5"
                class="text-input"
                class:has-error={!!validationErrors.dayStart}
                bind:value={dayStart}
                on:input={() => {
                  if (validationErrors.dayStart) validate();
                }}
              />
            </div>

            <!-- Quick Presets -->
            <div class="preset-chips">
              <button
                type="button"
                class="chip-btn"
                class:selected={dayStart === '00:00'}
                on:click={() => {
                  dayStart = '00:00';
                  validate();
                }}
              >
                00:00 (Midnight default)
              </button>
              <button
                type="button"
                class="chip-btn"
                class:selected={dayStart === '04:00'}
                on:click={() => {
                  dayStart = '04:00';
                  validate();
                }}
              >
                04:00 (Night owl)
              </button>
              <button
                type="button"
                class="chip-btn"
                class:selected={dayStart === '06:00'}
                on:click={() => {
                  dayStart = '06:00';
                  validate();
                }}
              >
                06:00 (Early morning)
              </button>
            </div>
          </div>

          {#if validationErrors.dayStart}
            <span class="field-error">{validationErrors.dayStart}</span>
          {:else}
            <span class="field-hint">
              Daily reports partition time based on this threshold.
            </span>
          {/if}
        </div>
      </section>

      <!-- Section 3: Read-only Server & System Info -->
      <section class="card-section secondary">
        <div class="card-header">
          <div class="header-text">
            <h2 class="card-title muted">System Information</h2>
            <p class="card-description">
              Internal storage and connection details configured in settings.json.
            </p>
          </div>
        </div>

        <div class="info-grid">
          <div class="info-cell">
            <span class="cell-label">Server Port</span>
            <span class="cell-value">{port}</span>
          </div>
          <div class="info-cell">
            <span class="cell-label">Data Directory</span>
            <span class="cell-value path" title={dataDir}>{dataDir}</span>
          </div>
          <div class="info-cell">
            <span class="cell-label">Extension Origins</span>
            <span class="cell-value">
              {extensionOrigins.length > 0 ? extensionOrigins.join(', ') : 'None configured'}
            </span>
          </div>
        </div>
      </section>

      <!-- Section 4: Data Export -->
      <section class="card-section">
        <div class="card-header">
          <div class="section-badge">3</div>
          <div class="header-text">
            <h2 class="card-title">Export JSON</h2>
            <p class="card-description">
              Export recorded events and computed analytics for a date range to a JSON file.
            </p>
          </div>
        </div>

        <div class="info-grid">
          <div class="field-group">
            <label for="export-from" class="field-label">From</label>
            <input id="export-from" type="date" class="text-input" bind:value={exportFrom} />
          </div>
          <div class="field-group">
            <label for="export-to" class="field-label">To</label>
            <input id="export-to" type="date" class="text-input" bind:value={exportTo} />
          </div>
          <div class="field-group">
            <label for="export-path" class="field-label">Output File Path</label>
            <input
              id="export-path"
              type="text"
              class="text-input"
              style="width: 100%; padding-right: 0.75rem;"
              bind:value={exportPath}
            />
          </div>
        </div>

        {#if exportResultMsg}
          <div class="banner success" role="status">{exportResultMsg}</div>
        {/if}
        {#if exportErrorMsg}
          <div class="banner error" role="alert">{exportErrorMsg}</div>
        {/if}

        <div>
          <button
            type="button"
            class="action-btn secondary"
            on:click={handleExport}
            disabled={exporting || !exportPath.trim()}
          >
            {exporting ? 'Exporting...' : 'Export JSON'}
          </button>
        </div>
      </section>

      <!-- Action Footer -->
      <footer class="form-actions">
        <div class="left-actions">
          <button
            type="button"
            class="action-btn secondary"
            on:click={resetToDefaults}
            disabled={saving}
          >
            Reset to defaults
          </button>
          {#if hasChanges}
            <button
              type="button"
              class="action-btn secondary"
              on:click={discardChanges}
              disabled={saving}
            >
              Discard changes
            </button>
          {/if}
        </div>

        <button
          type="submit"
          class="action-btn primary"
          disabled={saving}
        >
          {#if saving}
            <span class="button-spinner"></span>
            <span>Saving...</span>
          {:else}
            <svg class="btn-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
              <polyline points="17 21 17 13 7 13 7 21" />
              <polyline points="7 3 7 8 15 8" />
            </svg>
            <span>Save Settings</span>
          {/if}
        </button>
      </footer>
    </form>
  {/if}
</div>

<style>
  .settings-container {
    padding: 1.75rem 2rem;
    max-width: 860px;
    margin: 0 auto;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
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

  /* Banners */
  .banner {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem 1rem;
    border-radius: var(--radius-md);
    font-size: 0.88rem;
    animation: fadeIn 0.15s ease-out;
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(-4px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  .banner-content {
    display: flex;
    align-items: center;
    gap: 0.65rem;
  }

  .banner-icon {
    width: 18px;
    height: 18px;
    flex-shrink: 0;
  }

  .banner-close {
    background: transparent;
    border: none;
    font-size: 0.9rem;
    cursor: pointer;
    color: inherit;
    padding: 0.2rem 0.4rem;
    border-radius: var(--radius-sm);
  }

  .banner.success {
    background-color: var(--success-bg);
    border: 1px solid var(--success-border);
    color: var(--success-text);
  }

  .banner.error {
    background-color: #fde8e8;
    border: 1px solid #f98080;
    color: #9b1c1c;
  }

  /* Form and cards */
  .settings-form {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
  }

  .card-section {
    background-color: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-lg);
    padding: 1.25rem 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.15rem;
    box-shadow: var(--shadow-sm);
  }

  .card-section.secondary {
    background-color: var(--bg-card-subtle);
  }

  .card-header {
    display: flex;
    align-items: flex-start;
    gap: 0.85rem;
  }

  .section-badge {
    width: 24px;
    height: 24px;
    border-radius: 50%;
    background-color: var(--accent-light);
    color: var(--accent);
    font-weight: 700;
    font-size: 0.8rem;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    margin-top: 0.1rem;
  }

  .header-text {
    flex: 1;
  }

  .card-title {
    font-size: 1.05rem;
    font-weight: 600;
    margin: 0 0 0.25rem 0;
    color: var(--text-primary);
  }

  .card-title.muted {
    font-size: 0.95rem;
    color: var(--text-secondary);
  }

  .card-description {
    font-size: 0.84rem;
    color: var(--text-secondary);
    margin: 0;
    line-height: 1.4;
  }

  .field-group {
    display: flex;
    flex-direction: column;
    gap: 0.45rem;
  }

  .field-label {
    font-size: 0.84rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .input-with-presets {
    display: flex;
    align-items: center;
    gap: 1rem;
    flex-wrap: wrap;
  }

  .input-unit-wrapper {
    position: relative;
    display: flex;
    align-items: center;
  }

  .text-input {
    padding: 0.5rem 2rem 0.5rem 0.75rem;
    background-color: var(--input-bg);
    border: 1px solid var(--input-border);
    border-radius: var(--radius-md);
    font-size: 0.9rem;
    color: var(--text-primary);
    width: 140px;
    outline: none;
    transition: border-color 0.15s ease, box-shadow 0.15s ease;
  }

  .text-input:focus {
    border-color: var(--input-focus-border);
    box-shadow: 0 0 0 2px var(--accent-light);
  }

  .text-input.has-error {
    border-color: #e02424;
  }

  .unit-label {
    position: absolute;
    right: 0.65rem;
    font-size: 0.8rem;
    color: var(--text-muted);
    pointer-events: none;
  }

  .preset-chips {
    display: flex;
    gap: 0.35rem;
    flex-wrap: wrap;
  }

  .chip-btn {
    padding: 0.35rem 0.65rem;
    font-size: 0.78rem;
    border: 1px solid var(--border-color);
    background-color: var(--bg-card);
    color: var(--text-secondary);
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.12s ease;
  }

  .chip-btn:hover {
    background-color: var(--item-hover-bg);
    color: var(--text-primary);
  }

  .chip-btn.selected {
    background-color: var(--accent-light);
    border-color: var(--accent);
    color: var(--accent);
    font-weight: 600;
  }

  .field-hint {
    font-size: 0.78rem;
    color: var(--text-muted);
  }

  .field-error {
    font-size: 0.78rem;
    color: #e02424;
    font-weight: 500;
  }

  /* Info Grid */
  .info-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 0.75rem;
  }

  .info-cell {
    background-color: var(--bg-card);
    border: 1px solid var(--border-color);
    border-radius: var(--radius-md);
    padding: 0.65rem 0.85rem;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }

  .cell-label {
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    color: var(--text-muted);
    font-weight: 600;
  }

  .cell-value {
    font-size: 0.86rem;
    font-weight: 500;
    color: var(--text-primary);
  }

  .cell-value.path {
    font-family: monospace;
    font-size: 0.8rem;
    word-break: break-all;
  }

  /* Actions */
  .form-actions {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    padding-top: 0.5rem;
  }

  .left-actions {
    display: flex;
    gap: 0.5rem;
  }

  .action-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.6rem 1.15rem;
    font-size: 0.88rem;
    font-weight: 600;
    border-radius: var(--radius-md);
    cursor: pointer;
    transition: all 0.15s ease;
    border: 1px solid transparent;
  }

  .action-btn.primary {
    background-color: var(--accent);
    color: var(--accent-text);
  }

  .action-btn.primary:hover:not(:disabled) {
    background-color: var(--accent-hover);
  }

  .action-btn.primary:disabled {
    opacity: 0.65;
    cursor: not-allowed;
  }

  .action-btn.secondary {
    background-color: var(--bg-card);
    border-color: var(--border-color);
    color: var(--text-secondary);
  }

  .action-btn.secondary:hover:not(:disabled) {
    background-color: var(--item-hover-bg);
    color: var(--text-primary);
  }

  .btn-icon {
    width: 16px;
    height: 16px;
  }

  .button-spinner {
    width: 14px;
    height: 14px;
    border: 2px solid rgba(255, 255, 255, 0.4);
    border-top-color: #ffffff;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }

  .loading-state {
    padding: 3rem 1rem;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.75rem;
    color: var(--text-muted);
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
</style>
