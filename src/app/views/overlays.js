import { escapeHtml } from "../format.js";
import { icon } from "../icons.js";

function settingNavItem(id, label, iconName, active) {
  return `<button type="button" data-action="settings-section" data-value="${id}" class="${active ? "active" : ""}">${icon(iconName, 17)} ${label}</button>`;
}

function appearanceSettings(state) {
  return `
    <section class="settings-page">
      <div class="settings-page-heading"><h2>Appearance</h2><p>Choose how Edentic feels while you work.</p></div>
      <div class="settings-group">
        <h3>Theme</h3>
        <div class="theme-options">
          ${["dark", "light", "system"].map((theme) => `
            <button type="button" data-action="set-theme" data-value="${theme}" class="theme-option ${state.settings.theme === theme ? "active" : ""}">
              <span class="theme-preview ${theme}"><i></i><i></i><i></i></span>
              <strong>${theme[0].toUpperCase() + theme.slice(1)}</strong>
              <small>${theme === "dark" ? "Dark gray and warm black" : theme === "light" ? "Paper beige and warm white" : "Follow Windows"}</small>
            </button>`).join("")}
        </div>
      </div>
    </section>`;
}

function performanceSettings(state) {
  const modes = [
    ["auto", "Auto", "Choose the best processor for each task."],
    ["gpu", "GPU preferred", "Prioritize supported GPU workloads."],
    ["hybrid", "GPU + CPU", "Distribute compatible work across both."],
    ["cpu", "CPU only", "Do not initialize GPU processing."],
  ];
  return `
    <section class="settings-page">
      <div class="settings-page-heading"><h2>Performance</h2><p>Control how Edentic uses your hardware.</p></div>
      <div class="settings-group">
        <h3>Processing mode</h3>
        <div class="radio-list">
          ${modes.map(([value, title, copy]) => `
            <label>
              <input type="radio" name="computeMode" value="${value}" ${state.settings.computeMode === value ? "checked" : ""} />
              <span><strong>${title}</strong><small>${copy}</small></span>
            </label>`).join("")}
        </div>
      </div>
      <div class="settings-group">
        <h3>Detected hardware</h3>
        <dl class="diagnostic-list">
          <div><dt>Graphics</dt><dd>${escapeHtml(state.hardware.gpuName)}</dd></div>
          <div><dt>FFmpeg</dt><dd>${escapeHtml(state.hardware.ffmpegVersion)}</dd></div>
          <div><dt>ffprobe</dt><dd>${escapeHtml(state.hardware.ffprobeVersion)}</dd></div>
          <div><dt>Python</dt><dd>${escapeHtml(state.hardware.pythonVersion)}</dd></div>
        </dl>
      </div>
      <div class="settings-group field-pair">
        <label class="field"><span>Concurrent jobs</span><input id="concurrent-jobs" type="number" min="1" max="8" value="${state.settings.maxConcurrentJobs}" /></label>
        <label class="field"><span>Local transcription model</span>
          <select id="whisper-model">
            ${["tiny", "base", "small", "medium"].map((model) => `<option value="${model}" ${state.settings.whisperModel === model ? "selected" : ""}>${model}</option>`).join("")}
          </select>
        </label>
      </div>
    </section>`;
}

function mediaSettings(state) {
  return `
    <section class="settings-page">
      <div class="settings-page-heading"><h2>Media and cache</h2><p>Manage project storage and derived media.</p></div>
      <div class="settings-group">
        <label class="field">
          <span>Default projects folder</span>
          <div class="inline-field"><input value="${escapeHtml(state.settings.projectsRoot)}" readonly /><button class="button button-quiet" type="button" data-action="change-projects-root">Choose</button></div>
        </label>
      </div>
      <div class="settings-group field-pair">
        <label class="field"><span>Proxy quality</span>
          <select id="proxy-quality">
            <option value="performance" ${state.settings.proxyQuality === "performance" ? "selected" : ""}>Performance</option>
            <option value="balanced" ${state.settings.proxyQuality === "balanced" ? "selected" : ""}>Balanced</option>
            <option value="quality" ${state.settings.proxyQuality === "quality" ? "selected" : ""}>High quality</option>
          </select>
        </label>
        <label class="field"><span>Cache limit</span><div class="suffix-input"><input id="cache-limit" type="number" min="5" max="500" value="${state.settings.cacheLimitGb}" /><span>GB</span></div></label>
      </div>
    </section>`;
}

function providersSettings(state) {
  return `
    <section class="settings-page">
      <div class="settings-page-heading"><h2>AI providers</h2><p>OpenRouter is the first provider. Editing intelligence arrives in Slice 2.</p></div>
      <div class="provider-heading">
        <div class="provider-logo">OR</div>
        <div><h3>OpenRouter</h3><p>Bring your own key. Stored in the Windows credential vault.</p></div>
        <span class="connection-state ${state.settings.openrouterConfigured ? "connected" : ""}">${state.settings.openrouterConfigured ? "Configured" : "Not configured"}</span>
      </div>
      <div class="settings-group">
        <label class="field">
          <span>API key</span>
          <div class="inline-field">
            <input id="openrouter-key" type="password" autocomplete="off" placeholder="${state.settings.openrouterConfigured ? "Key stored securely" : "sk-or-v1-…"}" />
            <button class="button button-primary" type="button" data-action="save-openrouter-key">Save key</button>
          </div>
        </label>
        <p class="privacy-note">${icon("spark", 15)} Source video never leaves the device during local indexing. Future AI requests will show exactly what derived context is being sent.</p>
      </div>
      <div class="settings-group">
        <label class="field">
          <span>Default model</span>
          <div class="inline-field">
            <input id="openrouter-model" list="openrouter-model-options" value="${escapeHtml(state.settings.openrouterModel)}" />
            <button class="button button-quiet" type="button" data-action="load-openrouter-models" ${state.settings.openrouterConfigured ? "" : "disabled"}>Refresh models</button>
          </div>
          <datalist id="openrouter-model-options">
            ${state.openrouterModels.map((model) => `<option value="${escapeHtml(model.id)}">${escapeHtml(model.name)}${model.isFree ? " · free" : ""}</option>`).join("")}
          </datalist>
          <small><code>openrouter/free</code> selects an available free model that supports the requested capabilities.</small>
        </label>
        <div class="provider-actions">
          <button class="button button-quiet" type="button" data-action="test-openrouter" ${state.settings.openrouterConfigured ? "" : "disabled"}>Test connection</button>
          <button class="text-button danger" type="button" data-action="delete-openrouter-key" ${state.settings.openrouterConfigured ? "" : "disabled"}>Delete stored key</button>
        </div>
      </div>
    </section>`;
}

export function renderSettings(state) {
  if (!state.settingsOpen) return "";
  const active = state.settingsSection ?? "appearance";
  const page = active === "performance"
    ? performanceSettings(state)
    : active === "media"
      ? mediaSettings(state)
      : active === "providers"
        ? providersSettings(state)
        : appearanceSettings(state);
  return `
    <div class="modal-layer settings-layer">
      <section class="settings-dialog">
        <aside class="settings-nav">
          <div class="settings-nav-heading"><span class="brand-mark small"><i></i></span><strong>Settings</strong></div>
          <nav>
            ${settingNavItem("appearance", "Appearance", "spark", active === "appearance")}
            ${settingNavItem("performance", "Performance", "waveform", active === "performance")}
            ${settingNavItem("media", "Media and cache", "folder", active === "media")}
            ${settingNavItem("providers", "AI providers", "spark", active === "providers")}
          </nav>
          <div class="settings-future">
            <small>COMING WITH EDITING</small>
            <span>Project defaults</span><span>Editing</span><span>Export</span><span>Accessibility</span>
          </div>
        </aside>
        <div class="settings-main">
          <div class="settings-close-row"><button class="icon-button" type="button" data-action="close-settings" aria-label="Close settings">${icon("close", 19)}</button></div>
          <div class="settings-scroll">${page}</div>
          <footer class="settings-footer">
            <span>Settings save to this device.</span>
            <button class="button button-primary" type="button" data-action="save-settings">Done</button>
          </footer>
        </div>
      </section>
    </div>`;
}

export function renderContextDialog(state) {
  if (!state.contextDialogOpen) return "";
  return `
    <div class="modal-layer" data-action="close-context-dialog">
      <form class="dialog context-dialog" id="context-form" data-stop-propagation>
        <div class="dialog-header">
          <div><p class="eyebrow">PROJECT CONTEXT</p><h2>Explain the footage</h2></div>
          <button class="icon-button" type="button" data-action="close-context-dialog" aria-label="Close">${icon("close", 19)}</button>
        </div>
        <p class="dialog-copy">Paste the coding-agent prompt, recipe, event information or creative brief. This remains inside the project.</p>
        <label class="field"><span>Name</span><input name="name" required placeholder="Implementation context" /></label>
        <label class="field"><span>Context</span><textarea name="content" required rows="10" placeholder="Describe what the recording shows and what matters…"></textarea></label>
        <div class="dialog-footer split">
          <button class="button button-quiet" type="button" data-action="import-context-file">${icon("folderOpen", 16)} Import .txt or .md</button>
          <div><button class="button button-quiet" type="button" data-action="close-context-dialog">Cancel</button><button class="button button-primary" type="submit">Add context</button></div>
        </div>
      </form>
    </div>`;
}
