import { deleteMediaAsset } from "./api.js";
import { notify, patchState, projectSnapshotPatch, state } from "./state.js";

let activeScrubber = null;
let activePointerId = null;

function sourcePlayer() {
  return document.querySelector("#source-player");
}

function isSourceMediaElement(target) {
  return target instanceof HTMLMediaElement && target.id === "source-player";
}

function sourceScrubber() {
  return document.querySelector("[data-source-scrubber]");
}

function sourceDuration(player, scrubber = sourceScrubber()) {
  if (Number.isFinite(player?.duration) && player.duration > 0) return player.duration;
  return Number(scrubber?.dataset.duration) || 0;
}

function timeLabel(seconds) {
  const total = Math.max(0, Math.floor(seconds || 0));
  const minutes = Math.floor(total / 60);
  const remainder = total % 60;
  return `${String(minutes).padStart(2, "0")}:${String(remainder).padStart(2, "0")}`;
}

function updateScrubber(player = sourcePlayer()) {
  const scrubber = sourceScrubber();
  if (!player || !scrubber) return;
  const duration = sourceDuration(player, scrubber);
  const currentTime = Math.min(Math.max(player.currentTime || 0, 0), duration || 0);
  const progress = duration > 0 ? (currentTime / duration) * 100 : 0;
  const playhead = scrubber.querySelector("#waveform-playhead");
  const progressFill = scrubber.querySelector("#waveform-progress");
  if (playhead) playhead.style.left = `${progress}%`;
  if (progressFill) progressFill.style.width = `${progress}%`;
  const currentLabel = document.querySelector("#current-time");
  if (currentLabel) currentLabel.textContent = timeLabel(currentTime);
  scrubber.setAttribute("aria-valuemax", String(duration));
  scrubber.setAttribute("aria-valuenow", String(currentTime));
  scrubber.setAttribute("aria-valuetext", timeLabel(currentTime));
}

function setSourceTime(player, seconds) {
  const duration = sourceDuration(player);
  if (duration <= 0) return;
  const target = Math.min(duration, Math.max(0, seconds));
  const apply = () => {
    try {
      player.currentTime = target;
      updateScrubber(player);
    } catch (error) {
      notify(error instanceof Error ? error.message : "Could not seek this source", "danger");
    }
  };
  if (player.readyState >= 1) apply();
  else player.addEventListener("loadedmetadata", apply, { once: true });
}

function seekToRatio(scrubber, clientX) {
  const player = sourcePlayer();
  if (!player) return;
  const bounds = scrubber.getBoundingClientRect();
  if (bounds.width <= 0) return;
  const ratio = Math.min(1, Math.max(0, (clientX - bounds.left) / bounds.width));
  const duration = sourceDuration(player, scrubber);
  if (duration <= 0) return;
  setSourceTime(player, ratio * duration);
}

function adjustSourceTime(delta, absolute = false) {
  const player = sourcePlayer();
  if (!player) return;
  const duration = sourceDuration(player);
  if (duration <= 0) return;
  setSourceTime(player, absolute ? delta : player.currentTime + delta);
}

function toggleSourcePlayback() {
  const player = sourcePlayer();
  if (!player) return;
  if (!player.paused) {
    player.pause();
    return;
  }
  if (player.ended) setSourceTime(player, 0);
  player.play().catch((error) => {
    notify(error instanceof Error ? `Playback failed: ${error.message}` : "Playback failed", "danger");
  });
}

export function stopWorkspacePlayback() {
  const player = sourcePlayer();
  if (!player) return;
  player.pause();
  try {
    player.currentTime = 0;
  } catch {
    // A source without loaded metadata can still be safely paused before removal.
  }
}

export function captureWorkspacePlayback() {
  const player = sourcePlayer();
  if (!player) return null;
  return {
    assetId: player.dataset.assetId,
    currentTime: Number.isFinite(player.currentTime) ? player.currentTime : 0,
    paused: player.paused,
    muted: player.muted,
    volume: player.volume,
    playbackRate: player.playbackRate,
  };
}

export function restoreWorkspacePlayback(snapshot) {
  if (!snapshot) return;
  const player = sourcePlayer();
  if (!player || player.dataset.assetId !== snapshot.assetId) return;

  const restore = () => {
    player.muted = snapshot.muted;
    player.volume = snapshot.volume;
    player.playbackRate = snapshot.playbackRate;
    if (Number.isFinite(player.duration) && player.duration > 0) {
      player.currentTime = Math.min(snapshot.currentTime, Math.max(0, player.duration - 0.01));
    }
    if (!snapshot.paused) player.play().catch(() => {});
  };

  if (player.readyState >= 1) restore();
  else player.addEventListener("loadedmetadata", restore, { once: true });
  updateScrubber(player);
}

async function handleWorkspaceAction(action, element) {
  if (action === "toggle-play") {
    toggleSourcePlayback();
  }
  if (action === "toggle-media-panel") {
    patchState({ mediaPanelCollapsed: !state.mediaPanelCollapsed });
  }
  if (action === "toggle-video-map-panel") {
    patchState({ videoMapPanelCollapsed: !state.videoMapPanelCollapsed });
  }
  if (action === "request-delete-asset") {
    patchState({ assetDeleteId: element.dataset.assetId });
  }
  if (action === "close-delete-asset") {
    patchState({ assetDeleteId: null });
  }
  if (action === "confirm-delete-asset") {
    if (!state.activeProject) return;
    const assetId = element.dataset.assetId;
    const snapshot = await deleteMediaAsset(state.activeProject.path, assetId);
    patchState({
      ...projectSnapshotPatch(snapshot),
      assistantAssetIds: state.assistantAssetIds.filter((id) => id !== assetId),
      selectedAssetId: snapshot.assets[0]?.id ?? null,
      selectedSceneId: null,
      assetDeleteId: null,
    });
    notify("Source removed from the project", "success");
  }
}

function editableTarget(target) {
  return target instanceof HTMLElement
    && (target.matches("input, textarea, select") || target.isContentEditable);
}

export function installWorkspaceRuntime() {
  document.addEventListener("pointerdown", (event) => {
    const scrubber = event.target instanceof Element
      ? event.target.closest("[data-source-scrubber]")
      : null;
    if (!scrubber || event.button !== 0) return;
    event.preventDefault();
    activeScrubber = scrubber;
    activePointerId = event.pointerId;
    scrubber.classList.add("scrubbing");
    scrubber.setPointerCapture?.(event.pointerId);
    seekToRatio(scrubber, event.clientX);
  }, true);

  document.addEventListener("pointermove", (event) => {
    if (!activeScrubber || event.pointerId !== activePointerId) return;
    event.preventDefault();
    seekToRatio(activeScrubber, event.clientX);
  }, true);

  const finishScrubbing = (event) => {
    if (!activeScrubber || event.pointerId !== activePointerId) return;
    seekToRatio(activeScrubber, event.clientX);
    activeScrubber.classList.remove("scrubbing");
    if (activeScrubber.hasPointerCapture?.(event.pointerId)) {
      activeScrubber.releasePointerCapture(event.pointerId);
    }
    activeScrubber = null;
    activePointerId = null;
  };
  document.addEventListener("pointerup", finishScrubbing, true);
  document.addEventListener("pointercancel", finishScrubbing, true);

  for (const eventName of ["timeupdate", "loadedmetadata", "durationchange", "seeked", "ended"]) {
    document.addEventListener(eventName, (event) => {
      if (isSourceMediaElement(event.target)) updateScrubber(event.target);
    }, true);
  }

  document.addEventListener("click", async (event) => {
    const element = event.target instanceof Element ? event.target.closest("[data-action]") : null;
    if (!element) return;
    const action = element.dataset.action;
    if (!["toggle-play", "toggle-media-panel", "toggle-video-map-panel", "request-delete-asset", "close-delete-asset", "confirm-delete-asset"].includes(action)) return;
    event.preventDefault();
    try {
      await handleWorkspaceAction(action, element);
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error), "danger");
    }
  }, true);

  document.addEventListener("input", (event) => {
    if (!(event.target instanceof HTMLInputElement)) return;
    const query = event.target.value.trim().toLowerCase();
    if (event.target.id === "media-search") {
      document.querySelectorAll(".asset-item").forEach((item) => {
        item.hidden = !item.textContent.toLowerCase().includes(query);
      });
    }
    if (event.target.id === "video-map-search") {
      state.videoMapQuery = event.target.value;
      let matches = 0;
      const rows = document.querySelectorAll(".transcript-row");
      rows.forEach((row) => {
        const matched = row.textContent.toLowerCase().includes(query);
        row.hidden = !matched;
        if (matched) matches += 1;
      });
      const status = document.querySelector("#video-map-search-status");
      if (status) {
        status.textContent = query
          ? `${matches} ${matches === 1 ? "match" : "matches"}`
          : "";
      }
    }
  });

  window.addEventListener("keydown", (event) => {
    if (state.screen !== "workspace" || editableTarget(event.target)) return;
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "b") {
      event.preventDefault();
      if (event.shiftKey) patchState({ videoMapPanelCollapsed: !state.videoMapPanelCollapsed });
      else patchState({ mediaPanelCollapsed: !state.mediaPanelCollapsed });
      return;
    }

    const target = event.target instanceof Element ? event.target : null;
    const scrubber = target?.closest("[data-source-scrubber]");
    if (scrubber && ["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) {
      event.preventDefault();
      if (event.key === "Home") adjustSourceTime(0, true);
      else if (event.key === "End") adjustSourceTime(sourceDuration(sourcePlayer()), true);
      else adjustSourceTime(event.key === "ArrowLeft" ? -5 : 5);
      return;
    }

    const interactive = target?.closest("button, a, [role='slider']");
    if (!interactive && event.code === "Space" && !event.ctrlKey && !event.metaKey && !event.altKey) {
      event.preventDefault();
      toggleSourcePlayback();
    }
    if (!interactive && ["ArrowLeft", "ArrowRight"].includes(event.key) && !event.ctrlKey && !event.metaKey && !event.altKey) {
      event.preventDefault();
      adjustSourceTime(event.key === "ArrowLeft" ? -5 : 5);
    }
  });
}
