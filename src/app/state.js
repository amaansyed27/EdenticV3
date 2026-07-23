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

export function applyTheme(theme) {
  const resolved = theme === "system"
    ? (matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark")
    : theme;
  document.documentElement.dataset.theme = resolved;
  document.documentElement.dataset.themePreference = theme;
  const meta = document.querySelector('meta[name="theme-color"]');
  meta?.setAttribute("content", resolved === "light" ? "#f3eee3" : "#151515");
}

export function notify(message, tone = "neutral") {
  patchState({ toast: { message, tone } });
  window.clearTimeout(notify.timer);
  notify.timer = window.setTimeout(() => patchState({ toast: null }), 3600);
}
