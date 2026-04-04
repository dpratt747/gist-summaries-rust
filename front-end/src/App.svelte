<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'
  import { listen } from '@tauri-apps/api/event'

  interface LoadProgress {
    phase: string
    done: number
    total: number
  }

  interface GistFileRow {
    filename: string
    gist_url: string
  }

  let gists = $state<GistFileRow[]>([])
  let summaries = $state<Record<string, string>>({})
  let loadingGists = $state(false)
  let summarising = $state(false)
  let summarisedCount = $state(0)
  let error = $state('')
  let username = $state('')
  let githubToken = $state('')
  let tokenConfirmed = $state(false)
  let saveToken = $state(false)

  let searchQuery = $state('')
  let progressPhase = $state('')
  let progressDone = $state(0)
  let progressTotal = $state(0)

  $effect(() => {
    const unlisten = listen<LoadProgress>('load-progress', (event) => {
      progressPhase = event.payload.phase
      progressDone = event.payload.done
      progressTotal = event.payload.total
    })
    return () => { unlisten.then(fn => fn()) }
  })

  let summariesStarted = $derived(summarisedCount > 0 || summarising)

  let filteredGists = $derived.by(() => {
    const q = searchQuery.trim().toLowerCase()
    if (!q) return gists
    return gists.filter(g => {
      const key = `${g.gist_url}\0${g.filename}`
      const summary = (summaries[key] ?? '').toLowerCase()
      return g.filename.toLowerCase().includes(q) || summary.includes(q)
    })
  })

  let scrollContainer = $state<HTMLDivElement | null>(null)

  function startResize(e: MouseEvent) {
    const handle = e.currentTarget as HTMLDivElement
    const th = handle.parentElement as HTMLTableCellElement
    const startX = e.clientX
    const startWidth = th.offsetWidth

    function onMove(ev: MouseEvent) {
      th.style.width = `${Math.max(40, startWidth + ev.clientX - startX)}px`
    }
    function onUp() {
      document.removeEventListener('mousemove', onMove)
      document.removeEventListener('mouseup', onUp)
    }
    document.addEventListener('mousemove', onMove)
    document.addEventListener('mouseup', onUp)
  }

  $effect(() => {
    searchQuery  // dependency
    if (scrollContainer) scrollContainer.scrollTop = 0
  })

  invoke<string>('load_token').then((saved) => {
    if (saved) {
      githubToken = saved
      tokenConfirmed = true
    }
  })

  async function confirmToken() {
    if (!githubToken.trim()) {
      error = 'Please enter a GitHub token.'
      return
    }
    error = ''
    if (saveToken) {
      await invoke('save_token', { token: githubToken.trim() })
    }
    tokenConfirmed = true
  }

  async function loadGists() {
    if (!username.trim()) {
      error = 'Please enter a GitHub username.'
      return
    }
    loadingGists = true
    error = ''
    summaries = {}
    summarisedCount = 0
    progressPhase = ''
    progressDone = 0
    progressTotal = 0
    try {
      gists = await invoke<GistFileRow[]>('get_gists', { username: username.trim(), token: githubToken.trim() })
    } catch (e) {
      error = String(e)
    } finally {
      loadingGists = false
      progressPhase = ''
    }
  }

  async function generateSummaries() {
    if (summarising) return
    summarising = true
    summarisedCount = 0
    error = ''
    try {
      for (const gist of gists) {
        const key = `${gist.gist_url}\0${gist.filename}`
        summaries[key] = await invoke<string>('summarise_file', {
          gistUrl: gist.gist_url,
          filename: gist.filename,
        })
        summarisedCount += 1
        await new Promise(r => setTimeout(r, 100)) // small delay to allow UI updates between summaries - reduce lag
      }
    } catch (e) {
      error = String(e)
    } finally {
      summarising = false
    }
  }
</script>

<main>
  <h1>🗂 Gist Summary</h1>

  {#if !tokenConfirmed}
    <div class="token-screen">
      <p class="token-hint">Enter your GitHub personal access token to continue.</p>
      <div class="token-row">
        <input
          type="password"
          placeholder="ghp_••••••••••••••••••••"
          bind:value={githubToken}
          onkeydown={(e) => e.key === 'Enter' && confirmToken()}
        />
        <button onclick={confirmToken}>Continue</button>
      </div>
      <label class="save-label">
        <input type="checkbox" bind:checked={saveToken} />
        Save token to .env
      </label>
      {#if error}<p class="error">✗ {error}</p>{/if}
      <p class="token-help">
        Generate one at
        <a href="https://github.com/settings/tokens" target="_blank" rel="noreferrer">
          github.com/settings/tokens
        </a>
        — only <code>gist:read</code> scope is required.
      </p>
    </div>
  {:else}

  <div class="toolbar">
    <input
      type="text"
      placeholder="GitHub username"
      bind:value={username}
      disabled={loadingGists || summarising}
      onkeydown={(e) => e.key === 'Enter' && loadGists()}
    />
    <button onclick={loadGists} disabled={loadingGists || summarising}>
      {loadingGists ? 'Loading…' : 'Load Gists'}
    </button>

    {#if gists.length > 0}
      <button
        class="secondary"
        onclick={generateSummaries}
        disabled={summarising || loadingGists}
      >
        {summarising
          ? `Summarising… (${summarisedCount}/${gists.length})`
          : 'Generate Summaries'}
      </button>
    {/if}

    {#if gists.length > 0}
      <input
        type="text"
        class="search-input"
        placeholder="Search gists…"
        bind:value={searchQuery}
        disabled={loadingGists}
      />
    {/if}

    <button class="token-change" onclick={() => { tokenConfirmed = false; error = '' }} disabled={loadingGists || summarising}>
      Change Token
    </button>
  </div>

  {#if loadingGists && progressPhase}
    {@const pct = progressTotal > 0 ? (progressDone / progressTotal) * 100 : 0}
    <div class="progress-section">
      <div class="progress-label">
        {#if progressPhase === 'gists'}
          Fetching gists… ({progressDone}/{progressTotal})
        {:else}
          Downloading file contents… ({progressDone}/{progressTotal})
        {/if}
      </div>
      <div class="progress-bar">
        <div class="progress-fill" style="width: {pct}%"></div>
      </div>
    </div>
  {/if}

  {#if error}
    <p class="error">✗ {error}</p>
  {/if}

  {#if filteredGists.length > 0}
    <div class="gist-count">{filteredGists.length} gist{filteredGists.length === 1 ? '' : 's'}{searchQuery.trim() ? ` matching "${searchQuery.trim()}"` : ''}</div>
    <div
      class="scroll-container"
      bind:this={scrollContainer}
    >
      <table>
        <thead>
          <tr>
            <th style="width: 20%">Filename<div class="resize-handle" onmousedown={startResize}></div></th>
            <th style="width: 35%">Gist URL<div class="resize-handle" onmousedown={startResize}></div></th>
            {#if summariesStarted}<th>Summary</th>{/if}
          </tr>
        </thead>
        <tbody>
          {#each filteredGists as gist}
            {@const key = `${gist.gist_url}\0${gist.filename}`}
            <tr>
              <td class="filename">{gist.filename}</td>
              <td>
                <a href={gist.gist_url} target="_blank" rel="noreferrer">
                  {gist.gist_url}
                </a>
              </td>
              {#if summariesStarted}
              <td class="summary">
                {#if summaries[key]}
                  {summaries[key]}
                {:else if summarising}
                  <span class="pending">…</span>
                {:else}
                  <span class="empty">—</span>
                {/if}
              </td>
              {/if}
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
  {/if}
</main>

<style>
  :global(*) { box-sizing: border-box; margin: 0; padding: 0; }
  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    background: #0d1117;
    color: #c9d1d9;
  }

  main { padding: 24px; }

  h1 { font-size: 1.6rem; margin-bottom: 16px; color: #f0f6fc; }

  .toolbar {
    display: flex;
    gap: 10px;
    margin-bottom: 16px;
  }

  input[type='text'] {
    background: #161b22;
    color: #c9d1d9;
    border: 1px solid #30363d;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 0.9rem;
    width: 200px;
  }
  input[type='text']:focus { outline: none; border-color: #58a6ff; }
  input[type='text']:disabled { opacity: 0.5; }

  button {    background: #238636;
    color: #fff;
    border: none;
    padding: 8px 18px;
    border-radius: 6px;
    font-size: 0.9rem;
    cursor: pointer;
  }
  button.secondary { background: #1f6feb; }
  button.secondary:hover:not(:disabled) { background: #388bfd; }
  button.token-change { background: transparent; color: #8b949e; border: 1px solid #30363d; margin-left: auto; }
  button.token-change:hover:not(:disabled) { background: #161b22; color: #c9d1d9; }
  button:hover:not(:disabled) { background: #2ea043; }
  button:disabled { opacity: 0.5; cursor: not-allowed; }

  .progress-section { margin-bottom: 12px; }
  .progress-label { font-size: 0.8rem; color: #8b949e; margin-bottom: 6px; }
  .progress-bar {
    height: 6px;
    background: #21262d;
    border-radius: 3px;
    overflow: hidden;
  }
  .progress-fill {
    height: 100%;
    background: #238636;
    border-radius: 3px;
    transition: width 0.15s ease;
  }
  .error { margin-top: 12px; color: #f85149; }

  .search-input { flex: 1; }

  .gist-count { font-size: 0.8rem; color: #8b949e; margin-top: 4px; margin-bottom: 4px; }

  .scroll-container {
    height: calc(100vh - 140px);
    overflow-y: auto;
    margin-top: 8px;
  }

  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.875rem;
    table-layout: fixed;
  }
  th {
    background: #161b22;
    padding: 10px 14px;
    text-align: left;
    border-bottom: 1px solid #30363d;
    color: #8b949e;
    font-weight: 600;
    text-transform: uppercase;
    font-size: 0.75rem;
    letter-spacing: 0.05em;
    position: sticky;
    top: 0;
    z-index: 1;
  }
  .resize-handle {
    position: absolute;
    right: -2px;
    top: 0;
    bottom: 0;
    width: 5px;
    z-index: 2;
    cursor: col-resize;
    background: transparent;
  }
  .resize-handle:hover { background: #58a6ff; }
  tbody tr {
    content-visibility: auto;
    contain-intrinsic-size: auto 41px;
  }
  td {
    padding: 10px 14px;
    border-bottom: 1px solid #21262d;
    vertical-align: top;
    overflow-wrap: break-word;
  }
  tr:hover td { background: #161b22; }
  a { color: #58a6ff; text-decoration: none; }
  a:hover { text-decoration: underline; }
  .filename { font-family: monospace; color: #79c0ff; white-space: nowrap; }
  .pending { color: #8b949e; font-style: italic; }
  .empty { color: #484f58; }

  .token-screen {
    max-width: 480px;
    margin-top: 40px;
    display: flex;
    flex-direction: column;
    gap: 16px;
  }
  .token-hint { color: #c9d1d9; font-size: 0.95rem; }
  .token-row { display: flex; gap: 10px; }
  .token-row input[type='password'] {
    flex: 1;
    background: #161b22;
    color: #c9d1d9;
    border: 1px solid #30363d;
    padding: 8px 12px;
    border-radius: 6px;
    font-size: 0.9rem;
    font-family: monospace;
  }
  .token-row input[type='password']:focus { outline: none; border-color: #58a6ff; }
  .token-help { font-size: 0.8rem; color: #8b949e; }
  .token-help code { color: #79c0ff; }
  .save-label { display: flex; align-items: center; gap: 8px; font-size: 0.85rem; color: #8b949e; cursor: pointer; }
  .save-label input[type='checkbox'] { accent-color: #58a6ff; cursor: pointer; }
</style>
