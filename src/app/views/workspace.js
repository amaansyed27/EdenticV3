import { toAssetUrl } from "../api.js";
import { escapeHtml, fileName, formatBytes, formatDuration } from "../format.js";
import { icon } from "../icons.js";
import {
  intelligenceNav, renderAssistantPanel, renderPlanPanel, renderRemoteDisclosure,
} from "./intelligence.js";

const imageExtensions = new Set(["png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff", "avif"]);
const audioExtensions = new Set(["wav", "mp3", "m4a", "aac", "flac", "ogg", "opus", "wma", "aiff", "aif"]);

function assetKind(asset) {
  const extension = asset.name.split(".").pop()?.toLowerCase() ?? "";
  if (imageExtensions.has(extension)) return "image";
  if (audioExtensions.has(extension) || asset.videoCodec === "none") return "audio";
  return "video";
}

function assetDetails(asset, kind) {
  if (kind === "audio") return `Audio · ${formatBytes(asset.sizeBytes)}`;
  return `${asset.width}×${asset.height} · ${formatBytes(asset.sizeBytes)}`;
}

function processingLabels(kind) {
  if (kind === "audio") {
    return {
      title: "Audio Map",
      shortTitle: "AUDIO MAP",
      icon: "waveform",
      build: "Build Audio Map",
      rebuild: "Rebuild Audio Map",
      ready: "Audio Map ready",
      partial: "Audio Map ready with warnings",
      waiting: "Awaiting local audio analysis",
      firstTab: "Markers",
    };
  }
  if (kind === "image") {
    return {
      title: "Image Preview",
      shortTitle: "IMAGE",
      icon: "media",
      build: "Prepare Image",
      rebuild: "Refresh Image Preview",
      ready: "Image preview ready",
      partial: "Image preview ready with warnings",
      waiting: "Image preview not prepared",
      firstTab: "Preview",
    };
  }
  return {
    title: "Video Map",
    shortTitle: "VIDEO MAP",
    icon: "film",
    build: "Build Video Map",
    rebuild: "Rebuild Video Map",
    ready: "Video Map ready",
    partial: "Video Map ready with warnings",
    waiting: "Awaiting local video analysis",
    firstTab: "Scenes",
  };
}

function assetItem(asset, selected) {
  const kind = assetKind(asset);
  const preview = toAssetUrl(kind === "image" ? asset.posterPath || asset.managedPath : asset.posterPath);
  return `
    <button class="asset-item ${selected ? "selected" : ""}" type="button" data-action="select-asset" data-asset-id="${asset.id}">
      <span class="asset-thumb ${kind}">
        ${preview ? `<img src="${preview}" alt="" />` : icon(kind === "audio" ? "waveform" : "media", 22)}
        ${kind !== "image" ? `<small>${formatDuration(asset.duration)}</small>` : ""}
      </span>
      <span class="asset-copy">
        <strong title="${escapeHtml(asset.name)}">${escapeHtml(asset.name)}</strong>
        <small>${assetDetails(asset, kind)}</small>
      </span>
      <span class="status-dot ${asset.indexStatus}" title="${escapeHtml(asset.indexStatus)}"></span>
    </button>`;
}

function scenesPanel(state, scenes, kind) {
  if (!scenes.length) {
    const emptyCopy = kind === "audio"
      ? ["No visual markers", "Audio sources use their waveform and transcript instead of visual scenes."]
      : kind === "image"
        ? ["Preview not prepared", "Prepare this image to create a compatible local preview."]
        : ["No scenes yet", "Build the Video Map to detect visual changes in this source."];
    return `
      <div class="map-empty">
        ${icon(kind === "audio" ? "waveform" : kind === "image" ? "media" : "film", 25)}
        <h3>${emptyCopy[0]}</h3>
        <p>${emptyCopy[1]}</p>
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

function transcriptPanel(transcript, query, kind) {
  const normalized = query.trim().toLowerCase();
  const matches = transcript.filter((segment) => segment.text.toLowerCase().includes(normalized));
  if (!transcript.length) {
    return `
      <div class="map-empty">
        ${icon("waveform", 25)}
        <h3>No transcript yet</h3>
        <p>${kind === "image" ? "Still images do not contain audio to transcribe." : "Configure local transcription in Settings, then process this source."}</p>
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
        ${icon("note", 25)}
        <h3>Add project context</h3>
        <p>Attach a prompt, implementation summary, recipe or event notes to explain the footage.</p>
        <button class="button button-quiet" type="button" data-action="add-context">Add context</button>
      </div>`;
  }
  return `
    <div class="context-list">
      <div class="context-list-heading">
        <span>${contexts.length} saved ${contexts.length === 1 ? "item" : "items"}</span>
        <button class="text-button" type="button" data-action="add-context">${icon("plus", 15)} Add</button>
      </div>
      ${contexts.map((context, index) => `
        <details class="context-row" ${index === 0 ? "open" : ""}>
          <summary>
            <span>${icon("note", 17)}</span>
            <span><strong>${escapeHtml(context.name)}</strong><small>${escapeHtml(context.source)}</small></span>
          </summary>
          <p class="context-body">${escapeHtml(context.content)}</p>
        </details>`).join("")}
    </div>`;
}

function semanticPanel(state, selectedAsset) {
  const segments = state.semanticSegments.filter((value) => !selectedAsset || value.assetId === selectedAsset.id);
  if (!segments.length) return `<div class="map-empty semantic-empty">
    ${icon("note", 25)}<h3>No semantic map yet</h3>
    <p>Build a useful local map first, or preview the exact derived data before remote analysis.</p>
    <div><button class="button button-quiet" type="button" data-action="build-local-semantics" ${state.semanticBusy ? "disabled" : ""}>${state.semanticBusy ? "Building…" : "Build locally"}</button>
    <button class="button button-primary" type="button" data-action="prepare-semantic-analysis" ${state.semanticBusy ? "disabled" : ""}>Preview remote analysis</button></div>
  </div>`;
  return `<div class="semantic-list"><div class="semantic-actions"><span>${segments.length} persisted segments</span>
    <button class="text-button" type="button" data-action="prepare-semantic-analysis" ${state.semanticBusy ? "disabled" : ""}>${icon("refresh", 14)} ${state.semanticBusy ? "Preparing…" : "Analyze remotely"}</button></div>
    ${segments.map((segment) => `<button class="semantic-row" type="button" data-action="seek-video" data-time="${segment.start}">
      <span><strong>${escapeHtml(segment.title)}</strong><small>${escapeHtml(segment.provenance)}</small></span>
      <time>${formatDuration(segment.start)}–${formatDuration(segment.end)}</time>
      <p>${escapeHtml(segment.description)}</p>
      ${segment.visualObservations?.length ? `<ul>${segment.visualObservations.map((value) => `<li>${escapeHtml(value)}</li>`).join("")}</ul>` : ""}
      <span class="semantic-row-footer"><b>${escapeHtml(segment.importance)}</b><b>${escapeHtml(segment.suggestedDecision)}</b><small>${Math.round(segment.confidence * 100)}%</small></span>
    </button>`).join("")}</div>`;
}

function activeJob(jobs) {
  return jobs.find((job) => ["queued", "running"].includes(job.status));
}

export function jobStrip(jobs) {
  const active = activeJob(jobs);
  if (!active) return "";
  return `
    <div class="job-strip" data-job-id="${active.id}">
      <div class="job-progress" style="--progress:${Math.round(active.progress * 100)}%"></div>
      <span class="spinner"></span>
      <strong data-job-stage>${escapeHtml(active.stage)}</strong>
      <span data-job-percent>${Math.round(active.progress * 100)}%</span>
      <button class="text-button" type="button" data-action="cancel-job" data-job-id="${active.id}">Cancel</button>
    </div>`;
}

export function applyWorkspaceTransientPatch(nextState, patch) {
  const keys = Object.keys(patch ?? {});
  if (!keys.length || keys.some((key) => !["jobs", "selectedSceneId"].includes(key))) return false;

  if (Object.hasOwn(patch, "jobs")) {
    const host = document.querySelector("#job-strip-host");
    if (!host) return false;
    const active = activeJob(nextState.jobs);
    const strip = host.querySelector(".job-strip");
    if (active && !strip) return false;
    if (!active) {
      host.innerHTML = "";
    } else {
      strip.dataset.jobId = active.id;
      strip.querySelector(".job-progress")?.style.setProperty("--progress", `${Math.round(active.progress * 100)}%`);
      const stage = strip.querySelector("[data-job-stage]");
      const percent = strip.querySelector("[data-job-percent]");
      const cancel = strip.querySelector('[data-action="cancel-job"]');
      if (stage) stage.textContent = active.stage;
      if (percent) percent.textContent = `${Math.round(active.progress * 100)}%`;
      if (cancel) cancel.dataset.jobId = active.id;
    }
  }

  if (Object.hasOwn(patch, "selectedSceneId")) {
    document.querySelectorAll(".scene-row").forEach((row) => {
      row.classList.toggle("active", row.dataset.sceneId === nextState.selectedSceneId);
    });
  }

  return true;
}

function collapsedRail(action, iconName, label, direction) {
  return `
    <div class="collapsed-panel-rail">
      <button class="collapsed-panel-button" type="button" data-action="${action}" aria-label="Open ${label}">
        ${icon(direction === "left" ? "chevronRight" : "chevronLeft", 17)}
        ${icon(iconName, 18)}
        <span>${label}</span>
      </button>
    </div>`;
}

function deleteAssetDialog(state) {
  const asset = state.assets.find((item) => item.id === state.assetDeleteId);
  if (!asset) return "";
  const busy = state.jobs.some((job) => job.assetId === asset.id && ["queued", "running"].includes(job.status));
  return `
    <div class="modal-layer" data-action="close-delete-asset">
      <section class="dialog delete-asset-dialog" data-stop-propagation>
        <div class="dialog-header">
          <div><p class="eyebrow">PROJECT MEDIA</p><h2>Remove source?</h2></div>
          <button class="icon-button" type="button" data-action="close-delete-asset" aria-label="Close">${icon("close", 19)}</button>
        </div>
        <p class="dialog-copy"><strong>${escapeHtml(asset.name)}</strong> will be removed from this project together with its proxy and derived source data.</p>
        <p class="delete-boundary">The original file at its external import location will not be touched.</p>
        ${busy ? `<p class="delete-busy">Cancel indexing and wait for it to stop before removing this source.</p>` : ""}
        <div class="dialog-footer">
          <button class="button button-quiet" type="button" data-action="close-delete-asset">Cancel</button>
          <button class="button button-danger" type="button" data-action="confirm-delete-asset" data-asset-id="${asset.id}" ${busy ? "disabled" : ""}>${icon("trash", 16)} Remove source</button>
        </div>
      </section>
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
  const selectedKind = selectedAsset ? assetKind(selectedAsset) : null;
  const processing = processingLabels(selectedKind);
  const sourcePrepared = ["ready", "partial"].includes(selectedAsset?.indexStatus);
  const isTemporal = selectedKind === "video" || selectedKind === "audio";
  const mediaUrl = selectedAsset
    ? toAssetUrl(selectedKind === "image"
      ? selectedAsset.posterPath || selectedAsset.managedPath
      : selectedAsset.proxyPath || selectedAsset.managedPath)
    : "";
  const waveformUrl = selectedAsset ? toAssetUrl(selectedAsset.waveformPath) : "";
  const gridClasses = [
    "editor-grid",
    state.mediaPanelCollapsed ? "media-panel-collapsed" : "",
    state.videoMapPanelCollapsed ? "video-map-panel-collapsed" : "",
  ].filter(Boolean).join(" ");

  const mediaPanel = state.mediaPanelCollapsed
    ? collapsedRail("toggle-media-panel", "media", "Media", "left")
    : `
      <div class="panel-header">
        <div><p class="panel-kicker">PROJECT</p><h2>Media</h2></div>
        <div class="panel-header-actions">
          <button class="icon-button" type="button" data-action="import-media" aria-label="Import media">${icon("plus", 18)}</button>
          <button class="icon-button panel-collapse-button" type="button" data-action="toggle-media-panel" aria-label="Collapse Media panel">${icon("chevronLeft", 18)}</button>
        </div>
      </div>
      <label class="search-field media-search">
        ${icon("search", 15)}
        <input id="media-search" type="search" placeholder="Search media" aria-label="Search media" />
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
      </div>`;

  const semanticCount = state.semanticSegments.filter((value) => !selectedAsset || value.assetId === selectedAsset.id).length;
  const sourcePanel = `
      <div class="panel-header map-header">
        <div><p class="panel-kicker">SOURCE UNDERSTANDING</p><h2>${processing.title}</h2></div>
        <div class="panel-header-actions">
          <span class="local-badge">${semanticCount ? "SEMANTIC" : "LOCAL"}</span>
          <button class="icon-button panel-collapse-button" type="button" data-action="toggle-video-map-panel" aria-label="Collapse Video Map panel">${icon("chevronRight", 18)}</button>
        </div>
      </div>
      ${intelligenceNav(state)}
      <div class="map-tabs" role="tablist">
        <button type="button" role="tab" data-action="map-tab" data-value="scenes" class="${state.videoMapTab === "scenes" ? "active" : ""}">${processing.firstTab} <span>${scenes.length}</span></button>
        <button type="button" role="tab" data-action="map-tab" data-value="transcript" class="${state.videoMapTab === "transcript" ? "active" : ""}">Transcript <span>${transcript.length}</span></button>
        <button type="button" role="tab" data-action="map-tab" data-value="semantic" class="${state.videoMapTab === "semantic" ? "active" : ""}">Semantic <span>${semanticCount}</span></button>
        <button type="button" role="tab" data-action="map-tab" data-value="context" class="${state.videoMapTab === "context" ? "active" : ""}">Context <span>${state.contexts.length}</span></button>
      </div>
      ${state.videoMapTab === "transcript"
        ? `<label class="search-field map-search">${icon("search", 15)}<input id="video-map-search" type="search" placeholder="Search transcript" value="${escapeHtml(state.videoMapQuery)}" /></label><p class="map-search-status" id="video-map-search-status" aria-live="polite"></p>`
        : ""}
      <div class="map-content">
        ${state.videoMapTab === "scenes"
          ? scenesPanel(state, scenes, selectedKind)
          : state.videoMapTab === "transcript"
            ? transcriptPanel(transcript, state.videoMapQuery, selectedKind)
            : state.videoMapTab === "semantic"
              ? semanticPanel(state, selectedAsset)
              : contextsPanel(state.contexts)}
      </div>
      <div class="map-footer">
        <span>${icon("clock", 14)} ${semanticCount ? `${semanticCount} semantic segments persisted` : selectedAsset?.indexStatus === "ready" ? processing.ready : selectedAsset?.indexStatus === "partial" ? processing.partial : processing.waiting}</span>
      </div>`;
  const mapPanel = state.videoMapPanelCollapsed
    ? collapsedRail("toggle-video-map-panel", "note", "Project intelligence", "right")
    : state.intelligenceView === "assistant"
      ? renderAssistantPanel(state)
      : state.intelligenceView === "plan"
        ? renderPlanPanel(state)
        : sourcePanel;

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
          <span class="stage-badge">ASSISTED PLANNING</span>
          <span>Slice 2 workspace</span>
        </div>
        <div class="editor-header-actions">
          <button class="button button-quiet" type="button" data-action="view-context">${icon("note", 16)} Project context${state.contexts.length ? ` · ${state.contexts.length}` : ""}</button>
          <button class="button button-primary" type="button" data-action="intelligence-view" data-value="assistant">${icon("note", 16)} Assistant</button>
          <button class="icon-button" type="button" data-action="open-settings" aria-label="Settings">${icon("gear", 18)}</button>
        </div>
      </header>

      <div class="${gridClasses}">
        <aside class="media-panel ${state.mediaPanelCollapsed ? "collapsed" : ""}">${mediaPanel}</aside>

        <section class="viewer-panel">
          <div class="viewer-stage">
            ${selectedAsset
              ? selectedKind === "image"
                ? `<img class="source-image" src="${escapeHtml(mediaUrl)}" alt="${escapeHtml(selectedAsset.name)}" />`
                : selectedKind === "audio"
                  ? `<div class="audio-source-view">${icon("waveform", 42)}<strong>${escapeHtml(fileName(selectedAsset.name))}</strong><audio id="source-player" data-asset-id="${selectedAsset.id}" src="${escapeHtml(mediaUrl)}" preload="metadata" controls controlslist="nodownload"></audio></div>`
                  : `<video id="source-player" data-asset-id="${selectedAsset.id}" src="${escapeHtml(mediaUrl)}" ${selectedAsset.posterPath ? `poster="${escapeHtml(toAssetUrl(selectedAsset.posterPath))}"` : ""} preload="metadata" controls controlslist="nodownload" playsinline></video>`
              : `
                <div class="viewer-empty">
                  <div class="viewer-empty-mark">${icon("play", 34)}</div>
                  <h2>Your media appears here.</h2>
                  <p>Import video, audio or still-image sources. Originals are copied into this project.</p>
                </div>`}
          </div>
          <div class="transport">
            <div class="transport-time"><span id="current-time">${selectedKind === "image" ? "STILL" : "00:00"}</span>${isTemporal ? `<i>/</i><span>${formatDuration(selectedAsset?.duration)}</span>` : ""}</div>
            <button class="transport-play" type="button" data-action="toggle-play" ${isTemporal ? "" : "disabled"}>${icon("play", 19)}</button>
            <div class="transport-meta">${selectedAsset ? (selectedKind === "audio" ? "Audio source" : `${selectedAsset.width}×${selectedAsset.height}`) : "No source selected"}</div>
          </div>
          <div class="source-overview">
            <div class="source-overview-header">
              <div>
                <p class="panel-kicker">SELECTED SOURCE</p>
                <h2>${selectedAsset ? escapeHtml(fileName(selectedAsset.name)) : "Nothing selected"}</h2>
              </div>
              <div class="source-overview-actions">
                ${activeIndexJob
                  ? `<button class="button button-primary" type="button" disabled>${icon("clock", 16)} ${escapeHtml(activeIndexJob.stage)}</button>`
                  : selectedAsset && !sourcePrepared
                    ? `<button class="button button-primary" type="button" data-action="index-asset" data-asset-id="${selectedAsset.id}">${icon(processing.icon, 16)} ${processing.build}</button>`
                    : selectedAsset
                      ? `<button class="button button-quiet" type="button" data-action="index-asset" data-asset-id="${selectedAsset.id}">${icon("refresh", 15)} ${processing.rebuild}</button>`
                      : ""}
                ${selectedAsset ? `<button class="icon-button source-delete-button" type="button" data-action="request-delete-asset" data-asset-id="${selectedAsset.id}" aria-label="Remove source">${icon("trash", 17)}</button>` : ""}
              </div>
            </div>
            ${isTemporal ? `<div
              class="waveform-track"
              id="source-scrubber"
              data-source-scrubber
              data-duration="${selectedAsset?.duration ?? 0}"
              role="slider"
              tabindex="0"
              aria-label="Source position"
              aria-valuemin="0"
              aria-valuemax="${selectedAsset?.duration ?? 0}"
              aria-valuenow="0"
              aria-valuetext="00:00"
              title="Click or drag to seek"
            >
              ${waveformUrl ? `<img src="${escapeHtml(waveformUrl)}" alt="Audio waveform" />` : `<div class="waveform-unavailable">Waveform is prepared locally for this source</div>`}
              <div class="waveform-progress" id="waveform-progress"></div>
              <div class="waveform-playhead" id="waveform-playhead"></div>
            </div>` : `<div class="still-source-note">${icon("media", 15)} Still image · no playback timeline</div>`}
            <div class="source-facts">
              <span><small>Type</small>${selectedKind || "—"}</span>
              <span><small>Duration</small>${selectedKind === "image" ? "Still" : formatDuration(selectedAsset?.duration)}</span>
              <span><small>Codec</small>${selectedKind === "audio" ? selectedAsset?.audioCodec : selectedAsset?.videoCodec || "—"}</span>
              <span><small>Processing</small><b class="index-state ${selectedAsset?.indexStatus || "none"}">${selectedAsset?.indexStatus || "Waiting"}</b></span>
            </div>
          </div>
        </section>

        <aside class="video-map-panel ${state.videoMapPanelCollapsed ? "collapsed" : ""}">${mapPanel}</aside>
      </div>
      <div id="job-strip-host" class="job-strip-host">${jobStrip(state.jobs)}</div>
    </main>
    ${deleteAssetDialog(state)}
    ${renderRemoteDisclosure(state)}`;
}
