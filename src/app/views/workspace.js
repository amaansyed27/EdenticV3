import { toAssetUrl } from "../api.js";
import { escapeHtml, fileName, formatBytes, formatDuration } from "../format.js";
import { icon } from "../icons.js";

function assetItem(asset, selected) {
  const poster = toAssetUrl(asset.posterPath);
  return `
    <button class="asset-item ${selected ? "selected" : ""}" type="button" data-action="select-asset" data-asset-id="${asset.id}">
      <span class="asset-thumb">
        ${poster ? `<img src="${poster}" alt="" />` : icon("media", 22)}
        <small>${formatDuration(asset.duration)}</small>
      </span>
      <span class="asset-copy">
        <strong title="${escapeHtml(asset.name)}">${escapeHtml(asset.name)}</strong>
        <small>${asset.width}×${asset.height} · ${formatBytes(asset.sizeBytes)}</small>
      </span>
      <span class="status-dot ${asset.indexStatus}" title="${escapeHtml(asset.indexStatus)}"></span>
    </button>`;
}

function scenesPanel(state, scenes) {
  if (!scenes.length) {
    return `
      <div class="map-empty">
        ${icon("film", 25)}
        <h3>No scenes yet</h3>
        <p>Index this source to detect visual changes and build its Video Map.</p>
      </div>`;
  }
  return `
    <div class="scene-list">
      ${scenes.map((scene, index) => {
        const thumb = toAssetUrl(scene.thumbnailPath);
        return `
          <button class="scene-row ${state.selectedSceneId === scene.id ? "active" : ""}" type="button" data-action="seek-scene" data-scene-id="${scene.id}" data-time="${scene.start}">
            <span class="scene-thumb">${thumb ? `<img src="${thumb}" alt="" />` : `<span>${String(index + 1).padStart(2, "0")}</span>`}</span>
            <span class="scene-copy">
              <strong>${escapeHtml(scene.label || `Scene ${index + 1}`)}</strong>
              <small>${formatDuration(scene.start)} — ${formatDuration(scene.end)}</small>
            </span>
          </button>`;
      }).join("")}
    </div>`;
}

function transcriptPanel(transcript, query) {
  const normalized = query.trim().toLowerCase();
  const matches = transcript.filter((segment) => segment.text.toLowerCase().includes(normalized));
  if (!transcript.length) {
    return `
      <div class="map-empty">
        ${icon("waveform", 25)}
        <h3>No transcript yet</h3>
        <p>Configure local transcription in Settings, then index this source.</p>
      </div>`;
  }
  if (!matches.length) {
    return `<div class="map-empty compact"><h3>No matches</h3><p>Try a different word or phrase.</p></div>`;
  }
  return `
    <div class="transcript-list">
      ${matches.map((segment) => `
        <button class="transcript-row" type="button" data-action="seek-video" data-time="${segment.start}">
          <time>${formatDuration(segment.start)}</time>
          <span>${escapeHtml(segment.text)}</span>
        </button>`).join("")}
    </div>`;
}

function contextsPanel(contexts) {
  if (!contexts.length) {
    return `
      <div class="map-empty">
        ${icon("spark", 25)}
        <h3>Add project context</h3>
        <p>Attach a prompt, implementation summary, recipe or event notes to explain the footage.</p>
        <button class="button button-quiet" type="button" data-action="add-context">Add context</button>
      </div>`;
  }
  return `
    <div class="context-list">
      ${contexts.map((context) => `
        <article class="context-row">
          <span>${icon("spark", 17)}</span>
          <div><strong>${escapeHtml(context.name)}</strong><small>${escapeHtml(context.source)}</small></div>
        </article>`).join("")}
      <button class="text-button context-add" type="button" data-action="add-context">${icon("plus", 15)} Add another</button>
    </div>`;
}

function jobStrip(jobs) {
  const active = jobs.find((job) => ["queued", "running"].includes(job.status));
  if (!active) return "";
  return `
    <div class="job-strip">
      <div class="job-progress" style="--progress:${Math.round(active.progress * 100)}%"></div>
      <span class="spinner"></span>
      <strong>${escapeHtml(active.stage)}</strong>
      <span>${Math.round(active.progress * 100)}%</span>
      <button class="text-button" type="button" data-action="cancel-job" data-job-id="${active.id}">Cancel</button>
    </div>`;
}

export function renderWorkspace(state) {
  const project = state.activeProject;
  const selectedAsset = state.assets.find((asset) => asset.id === state.selectedAssetId) ?? state.assets[0];
  const scenes = state.scenes.filter((scene) => !selectedAsset || scene.assetId === selectedAsset.id);
  const transcript = state.transcript.filter((segment) => !selectedAsset || segment.assetId === selectedAsset.id);
  const activeIndexJob = selectedAsset
    ? state.jobs.find((job) => job.assetId === selectedAsset.id && ["queued", "running"].includes(job.status))
    : null;
  const mediaUrl = selectedAsset ? toAssetUrl(selectedAsset.proxyPath || selectedAsset.managedPath) : "";
  const waveformUrl = selectedAsset ? toAssetUrl(selectedAsset.waveformPath) : "";

  return `
    <main class="workspace-shell">
      <header class="editor-header">
        <div class="editor-header-left">
          <button class="icon-button" type="button" data-action="go-home" aria-label="Back to projects">${icon("arrowLeft", 19)}</button>
          <span class="editor-brand-mark brand-mark" aria-hidden="true"><i></i></span>
          <div class="project-title">
            <strong>${escapeHtml(project?.name ?? "Untitled")}</strong>
            <small>${project?.resolution ?? ""} · ${project?.frameRate ?? ""} fps</small>
          </div>
        </div>
        <div class="workspace-stage">
          <span class="stage-badge">VIDEO MAP</span>
          <span>Slice 1 workspace</span>
        </div>
        <div class="editor-header-actions">
          <button class="button button-quiet" type="button" data-action="add-context">${icon("spark", 16)} Context</button>
          <button class="icon-button" type="button" data-action="open-settings" aria-label="Settings">${icon("gear", 18)}</button>
        </div>
      </header>

      <div class="editor-grid">
        <aside class="media-panel">
          <div class="panel-header">
            <div><p class="panel-kicker">PROJECT</p><h2>Media</h2></div>
            <button class="icon-button" type="button" data-action="import-media" aria-label="Import media">${icon("plus", 18)}</button>
          </div>
          <label class="search-field media-search">
            ${icon("search", 15)}
            <input type="search" placeholder="Search media" aria-label="Search media" />
          </label>
          <div class="media-list">
            ${state.assets.length
              ? state.assets.map((asset) => assetItem(asset, asset.id === selectedAsset?.id)).join("")
              : `
                <div class="media-empty">
                  <span>${icon("upload", 25)}</span>
                  <h3>Bring in footage</h3>
                  <p>Originals are copied into this project.</p>
                  <button class="button button-primary" type="button" data-action="import-media">Import media</button>
                </div>`}
          </div>
          <div class="media-panel-footer">
            <span>${state.assets.length} ${state.assets.length === 1 ? "source" : "sources"}</span>
            <button class="text-button" type="button" data-action="import-media">Import</button>
          </div>
        </aside>

        <section class="viewer-panel">
          <div class="viewer-stage">
            ${selectedAsset
              ? `<video id="source-player" src="${escapeHtml(mediaUrl)}" poster="${escapeHtml(toAssetUrl(selectedAsset.posterPath))}" preload="metadata"></video>`
              : `
                <div class="viewer-empty">
                  <div class="viewer-empty-mark">${icon("play", 34)}</div>
                  <h2>Your footage appears here.</h2>
                  <p>Import a screen recording, tutorial, match clip or any other source video.</p>
                </div>`}
          </div>
          <div class="transport">
            <div class="transport-time"><span id="current-time">00:00</span><i>/</i><span>${formatDuration(selectedAsset?.duration)}</span></div>
            <button class="transport-play" type="button" data-action="toggle-play" ${selectedAsset ? "" : "disabled"}>${icon("play", 19)}</button>
            <div class="transport-meta">${selectedAsset ? `${selectedAsset.width}×${selectedAsset.height}` : "No source selected"}</div>
          </div>
          <div class="source-overview">
            <div class="source-overview-header">
              <div>
                <p class="panel-kicker">SELECTED SOURCE</p>
                <h2>${selectedAsset ? escapeHtml(fileName(selectedAsset.name)) : "Nothing selected"}</h2>
              </div>
              ${activeIndexJob
                ? `<button class="button button-primary" type="button" disabled>${icon("spark", 16)} ${escapeHtml(activeIndexJob.stage)}</button>`
                : selectedAsset && selectedAsset.indexStatus !== "ready"
                  ? `<button class="button button-primary" type="button" data-action="index-asset" data-asset-id="${selectedAsset.id}">${icon("spark", 16)} Build Video Map</button>`
                : selectedAsset
                  ? `<button class="button button-quiet" type="button" data-action="index-asset" data-asset-id="${selectedAsset.id}">${icon("refresh", 15)} Reindex</button>`
                  : ""}
            </div>
            <div class="waveform-track">
              ${waveformUrl ? `<img src="${escapeHtml(waveformUrl)}" alt="Audio waveform" />` : `<div class="waveform-placeholder">${Array.from({ length: 72 }, (_, index) => `<i style="--h:${18 + ((index * 17) % 68)}%"></i>`).join("")}</div>`}
              <div class="waveform-playhead" id="waveform-playhead"></div>
            </div>
            <div class="source-facts">
              <span><small>Duration</small>${formatDuration(selectedAsset?.duration)}</span>
              <span><small>Codec</small>${selectedAsset?.videoCodec || "—"}</span>
              <span><small>Audio</small>${selectedAsset?.audioCodec || "—"}</span>
              <span><small>Index</small><b class="index-state ${selectedAsset?.indexStatus || "none"}">${selectedAsset?.indexStatus || "Waiting"}</b></span>
            </div>
          </div>
        </section>

        <aside class="video-map-panel">
          <div class="panel-header map-header">
            <div><p class="panel-kicker">SOURCE INTELLIGENCE</p><h2>Video Map</h2></div>
            <span class="local-badge">LOCAL</span>
          </div>
          <div class="map-tabs" role="tablist">
            <button type="button" role="tab" data-action="map-tab" data-value="scenes" class="${state.videoMapTab === "scenes" ? "active" : ""}">Scenes <span>${scenes.length}</span></button>
            <button type="button" role="tab" data-action="map-tab" data-value="transcript" class="${state.videoMapTab === "transcript" ? "active" : ""}">Transcript <span>${transcript.length}</span></button>
            <button type="button" role="tab" data-action="map-tab" data-value="context" class="${state.videoMapTab === "context" ? "active" : ""}">Context <span>${state.contexts.length}</span></button>
          </div>
          ${state.videoMapTab === "transcript"
            ? `<label class="search-field map-search">${icon("search", 15)}<input id="video-map-search" type="search" placeholder="Search transcript" value="${escapeHtml(state.videoMapQuery)}" /></label>`
            : ""}
          <div class="map-content">
            ${state.videoMapTab === "scenes"
              ? scenesPanel(state, scenes)
              : state.videoMapTab === "transcript"
                ? transcriptPanel(transcript, state.videoMapQuery)
                : contextsPanel(state.contexts)}
          </div>
          <div class="map-footer">
            <span>${icon("clock", 14)} ${selectedAsset?.indexStatus === "ready" ? "Index ready" : "Awaiting local index"}</span>
          </div>
        </aside>
      </div>
      ${jobStrip(state.jobs)}
    </main>`;
}
