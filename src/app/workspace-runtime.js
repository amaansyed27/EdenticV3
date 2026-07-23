import { deleteMediaAsset } from "./api.js";
import { notify, patchState, state } from "./state.js";

export function captureWorkspacePlayback() {
  const player = document.querySelector("#source-player");
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
  const player = document.querySelector("#source-player");
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
}

async function handleWorkspaceAction(action, element) {
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
      activeProject: snapshot.project,
      assets: snapshot.assets,
      scenes: snapshot.scenes,
      transcript: snapshot.transcript,
      contexts: snapshot.contexts,
      jobs: snapshot.jobs ?? [],
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
  document.addEventListener("click", async (event) => {
    const element = event.target instanceof Element ? event.target.closest("[data-action]") : null;
    if (!element) return;
    const action = element.dataset.action;
    if (!["toggle-media-panel", "toggle-video-map-panel", "request-delete-asset", "close-delete-asset", "confirm-delete-asset"].includes(action)) return;
    event.preventDefault();
    try {
      await handleWorkspaceAction(action, element);
    } catch (error) {
      notify(error instanceof Error ? error.message : String(error), "danger");
    }
  }, true);

  document.addEventListener("input", (event) => {
    if (!(event.target instanceof HTMLInputElement) || event.target.id !== "media-search") return;
    const query = event.target.value.trim().toLowerCase();
    document.querySelectorAll(".asset-item").forEach((item) => {
      item.hidden = !item.textContent.toLowerCase().includes(query);
    });
  });

  window.addEventListener("keydown", (event) => {
    if (state.screen !== "workspace" || editableTarget(event.target)) return;
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === "b") {
      event.preventDefault();
      if (event.shiftKey) patchState({ videoMapPanelCollapsed: !state.videoMapPanelCollapsed });
      else patchState({ mediaPanelCollapsed: !state.mediaPanelCollapsed });
    }
  });
}
