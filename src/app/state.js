export const defaultSettings = {
  onboardingComplete: false,
  projectsRoot: "",
  theme: "dark",
  computeMode: "auto",
  proxyQuality: "balanced",
  cacheLimitGb: 40,
  maxConcurrentJobs: 2,
  whisperModel: "small",
  openrouterConfigured: false,
  openrouterModel: "openrouter/free",
};

export const state = {
  screen: "boot",
  settings: { ...defaultSettings },
  hardware: {
    gpuName: "Detecting…",
    ffmpegVersion: "Detecting…",
    ffprobeVersion: "Detecting…",
    pythonVersion: "Detecting…",
  },
  projects: [],
  activeProject: null,
  assets: [],
  scenes: [],
  transcript: [],
  contexts: [],
  jobs: [],
  selectedAssetId: null,
  selectedSceneId: null,
  settingsOpen: false,
  settingsSection: "appearance",
  recoveryDialog: null,
  createProjectOpen: false,
  contextDialogOpen: false,
  toast: null,
  loading: false,
  projectView: "grid",
  projectQuery: "",
  videoMapTab: "scenes",
  videoMapQuery: "",
  openrouterModels: [],
};

const subscribers = new Set();

export function subscribe(listener) {
  subscribers.add(listener);
  return () => subscribers.delete(listener);
}

export function patchState(patch) {
  Object.assign(state, patch);
  for (const listener of subscribers) listener(state);
}

export function resolveTheme(theme) {
  if (theme !== "system") return theme;
  return matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark";
}

function syncNativeTheme(resolvedTheme) {
  const invoke = globalThis.window?.__TAURI__?.core?.invoke;
  if (!invoke) return;
  invoke("sync_window_theme", { theme: resolvedTheme }).catch((error) => {
    globalThis.window?.dispatchEvent(new CustomEvent("edentic:native-error", {
      detail: error instanceof Error ? error.message : String(error),
    }));
  });
}

export function applyTheme(theme) {
  const resolved = resolveTheme(theme);
  document.documentElement.dataset.theme = resolved;
  document.documentElement.dataset.themePreference = theme;
  const meta = document.querySelector('meta[name="theme-color"]');
  meta?.setAttribute("content", resolved === "light" ? "#f3eee3" : "#151515");
  syncNativeTheme(resolved);
}

export function notify(message, tone = "neutral") {
  patchState({ toast: { message, tone } });
  window.clearTimeout(notify.timer);
  notify.timer = window.setTimeout(() => patchState({ toast: null }), 3600);
}
