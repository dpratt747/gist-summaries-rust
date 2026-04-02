<script lang="ts">
  import { invoke } from '@tauri-apps/api/core'

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
  let summariesStarted = $state(false)
  let githubToken = $state('')
  let tokenConfirmed = $state(false)
  let saveToken = $state(false)

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
    summariesStarted = false
    try {
      gists = await invoke<GistFileRow[]>('get_gists', { username: username.trim(), token: githubToken.trim() })
    } catch (e) {
      error = String(e)
    } finally {
      loadingGists = false
    }
  }

  const CALL_DELAY_MS = 100

  async function generateSummaries() {
    if (summarising) return
    summarising = true
    summariesStarted = true
    summarisedCount = 0
    error = ''
    try {
      for (const gist of gists) {
        const summary = await invoke<string>('summarise_file', {
          gistUrl: gist.gist_url,
          filename: gist.filename,
        })
        summaries = { ...summaries, [`${gist.gist_url}\0${gist.filename}`]: summary }
        summarisedCount += 1
        await new Promise(r => setTimeout(r, CALL_DELAY_MS))
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
  </div>

  {#if error}
    <p class="error">✗ {error}</p>
  {/if}

  {#if gists.length > 0}
    <table>
      <thead>
        <tr>
          <th>Filename</th>
          <th>Gist URL</th>
          {#if summariesStarted}<th>Summary</th>{/if}
        </tr>
      </thead>
      <tbody>
        {#each gists as gist}
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
  button:hover:not(:disabled) { background: #2ea043; }
  button:disabled { opacity: 0.5; cursor: not-allowed; }

  .error { margin-top: 12px; color: #f85149; }

  table {
    width: 100%;
    border-collapse: collapse;
    margin-top: 16px;
    font-size: 0.875rem;
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
  }
  td {
    padding: 10px 14px;
    border-bottom: 1px solid #21262d;
    vertical-align: top;
  }
  tr:hover td { background: #161b22; }
  a { color: #58a6ff; text-decoration: none; }
  a:hover { text-decoration: underline; }
  .filename { font-family: monospace; color: #79c0ff; white-space: nowrap; }
  .summary { max-width: 400px; }
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
