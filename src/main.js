import { getBootstrap } from "./app/api.js";
import { renderApp, wireEvents } from "./app/render.js";
import { applyTheme, defaultSettings, patchState, state, subscribe } from "./app/state.js";
import {
  applyWorkspaceTransientPatch,
  captureWorkspacePlayback,
  installWorkspaceRuntime,
  restoreWorkspacePlayback,
} from "./app/workspace-runtime.js";

subscribe((nextState, patch) => {
  if (nextState.screen === "workspace" && applyWorkspaceTransientPatch(nextState, patch)) return;
  const playback = captureWorkspacePlayback();
  renderApp();
  wireEvents();
  restoreWorkspacePlayback(playback);
});

installWorkspaceRuntime();

async function boot() {
  applyTheme(defaultSettings.theme);
  renderApp();
  wireEvents();
  try {
    const bootstrap = await getBootstrap();
    const settings = { ...defaultSettings, ...bootstrap.settings };
    applyTheme(settings.theme);
    patchState({
      screen: settings.onboardingComplete ? "home" : "onboarding",
      settings,
      hardware: bootstrap.hardware,
      projects: bootstrap.projects,
    });
  } catch (error) {
    patchState({
      screen: "error",
      toast: { tone: "danger", message: error instanceof Error ? error.message : String(error) },
    });
  }
}

window.addEventListener("keydown", (event) => {
  if (event.key === "Escape") {
    patchState({
      settingsOpen: false,
      createProjectOpen: false,
      contextDialogOpen: false,
      recoveryDialog: null,
      assetDeleteId: null,
    });
  }
  if ((event.ctrlKey || event.metaKey) && event.key === ",") {
    event.preventDefault();
    patchState({ settingsOpen: true });
  }
});

const systemTheme = window.matchMedia("(prefers-color-scheme: light)");
systemTheme.addEventListener?.("change", () => {
  if (state.settings.theme === "system") applyTheme("system");
});

window.addEventListener("edentic:native-error", (event) => {
  patchState({ toast: { tone: "danger", message: event.detail } });
});

boot();
