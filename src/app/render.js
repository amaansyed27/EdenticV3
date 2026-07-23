import {
  addPastedContext,
  cancelIndexJob,
  chooseProjectsRoot,
  completeOnboarding,
  createProject,
  deleteOpenRouterKey,
  forgetProject,
  getBootstrap,
  getIndexJobs,
  getProjectSnapshot,
  importContext,
  importMedia,
  listOpenRouterModels,
  openProject,
  pickProject,
  repairApp,
  resetAll,
  resetAppData,
  resetCache,
  resetSettings,
  saveOpenRouterKey,
  saveSettings,
  startIndex,
  testOpenRouter,
} from "./api.js";
import { escapeHtml, formatDuration } from "./format.js";
import { icon } from "./icons.js";
import { applyTheme, notify, patchState, state } from "./state.js";
import { renderCreateProject, renderHome, renderOnboarding } from "./views/home.js";
import { renderContextDialog, renderRecoveryDialog, renderSettings } from "./views/overlays.js";
import { renderWorkspace } from "./views/workspace.js";

const app = document.querySelector("#app");
let jobTimer;

function renderToast() {
  if (!state.toast) return "";
  return `<div class="toast ${state.toast.tone}">${state.toast.tone === "danger" ? icon("close", 16) : icon("spark", 16)}<span>${escapeHtml(state.toast.message)}</span></div>`;
}

export function renderApp() {
  let content = "";
  if (state.screen === "boot") {
    content = `<main class="boot-screen"><span class="brand-mark large"><i></i></span><p>EDENTIC</p><div class="boot-line"></div></main>`;
  } else if (state.screen === "onboarding") {
    content = renderOnboarding(state);
  } else if (state.screen === "home") {
    content = renderHome(state);
  } else if (state.screen === "workspace") {
    content = renderWorkspace(state);
  } else {
    content = `<main class="error-screen"><h1>Edentic could not start.</h1><p>Check the terminal logs and try again.</p></main>`;
  }
  app.innerHTML = `${content}${renderCreateProject(state)}${renderSettings(state)}${renderRecoveryDialog(state)}${renderContextDialog(state)}${renderToast()}`;
  bindPlayer();
}

function bindPlayer() {
  const player = document.querySelector("#source-player");
  if (!player) return;
  player.addEventListener("timeupdate", () => {
    const current = document.querySelector("#current-time");
    const playhead = document.querySelector("#waveform-playhead");
    if (current) current.textContent = formatDuration(player.currentTime);
    if (playhead && player.duration) playhead.style.left = `${(player.currentTime / player.duration) * 100}%`;
  });
  player.addEventListener("play", updatePlayButton);
  player.addEventListener("pause", updatePlayButton);
}

function updatePlayButton() {
  const player = document.querySelector("#source-player");
  const button = document.querySelector('[data-action="toggle-play"]');
  if (!player || !button) return;
  button.innerHTML = player.paused ? icon("play", 19) : icon("pause", 19);
}

async function refreshProject() {
  if (!state.activeProject) return;
  const snapshot = await getProjectSnapshot(state.activeProject.path);
  patchState({
    activeProject: snapshot.project,
    assets: snapshot.assets,
    scenes: snapshot.scenes,
    transcript: snapshot.transcript,
    contexts: snapshot.contexts,
    jobs: snapshot.jobs ?? state.jobs,
    selectedAssetId: state.selectedAssetId ?? snapshot.assets[0]?.id ?? null,
  });
}

function startJobPolling() {
  window.clearInterval(jobTimer);
  if (!state.activeProject) return;
  jobTimer = window.setInterval(async () => {
    try {
      const jobs = await getIndexJobs(state.activeProject.path);
      const previouslyActive = state.jobs.some((job) => ["queued", "running"].includes(job.status));
      const currentlyActive = jobs.some((job) => ["queued", "running"].includes(job.status));
      patchState({ jobs });
      if (previouslyActive && !currentlyActive) {
        await refreshProject();
        const failure = jobs.find((job) => job.status === "failed");
        notify(failure ? failure.error || "Indexing failed" : "Video Map is ready", failure ? "danger" : "success");
      }
    } catch {
      // Polling is best-effort; command failures are surfaced by direct actions.
    }
  }, 1100);
}

function clearedProjectState() {
  return {
    activeProject: null,
    assets: [],
    scenes: [],
    transcript: [],
    contexts: [],
    jobs: [],
    selectedAssetId: null,
    selectedSceneId: null,
  };
}

function recoveryMessage(report) {
  const warnings = report.warnings?.length ?? 0;
  return warnings ? `${report.message} · ${warnings} warning${warnings === 1 ? "" : "s"}` : report.message;
}

async function runRecovery(scope) {
  const operations = {
    settings: resetSettings,
    data: resetAppData,
    cache: resetCache,
    repair: repairApp,
    all: resetAll,
  };
  const operation = operations[scope];
  if (!operation) throw new Error("Unknown recovery action");
  const report = await operation();
  const bootstrap = await getBootstrap();
  applyTheme(bootstrap.settings.theme);

  if (scope === "all") {
    window.clearInterval(jobTimer);
    patchState({
      ...clearedProjectState(),
      screen: "onboarding",
      settings: bootstrap.settings,
      hardware: bootstrap.hardware,
      projects: [],
      settingsOpen: false,
      recoveryDialog: null,
      openrouterModels: [],
    });
  } else if (scope === "data") {
    window.clearInterval(jobTimer);
    patchState({
      ...clearedProjectState(),
      screen: "home",
      settings: bootstrap.settings,
      hardware: bootstrap.hardware,
      projects: bootstrap.projects,
      settingsOpen: false,
      recoveryDialog: null,
      openrouterModels: [],
    });
  } else {
    patchState({
      settings: bootstrap.settings,
      hardware: bootstrap.hardware,
      projects: bootstrap.projects,
      recoveryDialog: null,
      settingsSection: "recovery",
    });
    if (state.activeProject && ["cache", "repair"].includes(scope)) {
      await refreshProject();
    }
  }
  notify(recoveryMessage(report), report.warnings?.length ? "neutral" : "success");
}

async function handleAction(action, element) {
  if (action === "new-project") patchState({ createProjectOpen: true });
  if (action === "close-create-project") patchState({ createProjectOpen: false });
  if (action === "open-settings") patchState({ settingsOpen: true });
  if (action === "close-settings") patchState({ settingsOpen: false, recoveryDialog: null });
  if (action === "settings-section") patchState({ settingsSection: element.dataset.value });
  if (action === "project-view") patchState({ projectView: element.dataset.value });
  if (action === "map-tab") patchState({ videoMapTab: element.dataset.value });
  if (action === "add-context") patchState({ contextDialogOpen: true });
  if (action === "close-context-dialog") patchState({ contextDialogOpen: false });
  if (action === "request-recovery") patchState({ recoveryDialog: element.dataset.value });
  if (action === "close-recovery") patchState({ recoveryDialog: null });
  if (action === "confirm-recovery") {
    const scope = element.dataset.value;
    if (scope === "all" && document.querySelector("#reset-all-confirmation")?.value.trim() !== "RESET") {
      return notify("Type RESET to confirm Reset All", "danger");
    }
    await runRecovery(scope);
  }
  if (action === "go-home") {
    window.clearInterval(jobTimer);
    const bootstrap = await getBootstrap();
    patchState({ screen: "home", projects: bootstrap.projects, activeProject: null });
  }
  if (action === "choose-projects-root" || action === "change-projects-root") {
    const projectsRoot = await chooseProjectsRoot();
    if (projectsRoot) {
      const settings = { ...state.settings, projectsRoot };
      patchState({ settings });
      if (state.settings.onboardingComplete) await saveSettings(settings);
    }
  }
  if (action === "finish-onboarding") {
    const bootstrap = await completeOnboarding(state.settings.projectsRoot);
    patchState({ settings: bootstrap.settings, projects: bootstrap.projects, screen: "home" });
  }
  if (action === "open-project-picker") {
    const snapshot = await pickProject();
    if (!snapshot) return;
    patchState({
      screen: "workspace",
      activeProject: snapshot.project,
      assets: snapshot.assets,
      scenes: snapshot.scenes,
      transcript: snapshot.transcript,
      contexts: snapshot.contexts,
      jobs: snapshot.jobs ?? [],
      selectedAssetId: snapshot.assets[0]?.id ?? null,
    });
    startJobPolling();
  }
  if (action === "forget-project") {
    await forgetProject(element.dataset.projectId);
    patchState({ projects: state.projects.filter((project) => project.id !== element.dataset.projectId) });
    notify("Project removed from recents");
  }
  if (action === "open-project") {
    patchState({ loading: true });
    const snapshot = await openProject(element.dataset.projectPath);
    patchState({
      loading: false,
      screen: "workspace",
      activeProject: snapshot.project,
      assets: snapshot.assets,
      scenes: snapshot.scenes,
      transcript: snapshot.transcript,
      contexts: snapshot.contexts,
      jobs: snapshot.jobs ?? [],
      selectedAssetId: snapshot.assets[0]?.id ?? null,
    });
    startJobPolling();
  }
  if (action === "import-media") {
    if (!state.activeProject) return;
    const assets = await importMedia(state.activeProject.path);
    if (assets.length) {
      await refreshProject();
      patchState({ selectedAssetId: assets.at(-1)?.id ?? state.selectedAssetId });
      const jobs = [];
      for (const asset of assets) {
        jobs.push(await startIndex(state.activeProject.path, asset.id));
      }
      patchState({ jobs: [...state.jobs, ...jobs] });
      startJobPolling();
      notify(`${assets.length} ${assets.length === 1 ? "source" : "sources"} imported · building Video Map`, "success");
    }
  }
  if (action === "select-asset") patchState({ selectedAssetId: element.dataset.assetId, selectedSceneId: null });
  if (action === "index-asset") {
    const job = await startIndex(state.activeProject.path, element.dataset.assetId);
    patchState({ jobs: [...state.jobs.filter((item) => item.id !== job.id), job] });
    startJobPolling();
  }
  if (action === "cancel-job") {
    await cancelIndexJob(element.dataset.jobId);
    notify("Index cancellation requested");
  }
  if (action === "seek-scene" || action === "seek-video") {
    const player = document.querySelector("#source-player");
    if (player) {
      player.currentTime = Number(element.dataset.time || 0);
      player.play().catch(() => {});
    }
    if (element.dataset.sceneId) patchState({ selectedSceneId: element.dataset.sceneId });
  }
  if (action === "toggle-play") {
    const player = document.querySelector("#source-player");
    if (player) player.paused ? player.play() : player.pause();
  }
  if (action === "set-theme") {
    const settings = { ...state.settings, theme: element.dataset.value };
    applyTheme(settings.theme);
    patchState({ settings });
  }
  if (action === "save-settings") {
    const settings = readSettingsForm();
    await saveSettings(settings);
    applyTheme(settings.theme);
    patchState({ settings, settingsOpen: false, recoveryDialog: null });
    notify("Settings saved", "success");
  }
  if (action === "save-openrouter-key") {
    const input = document.querySelector("#openrouter-key");
    if (!input?.value.trim()) return notify("Enter an OpenRouter API key", "danger");
    await saveOpenRouterKey(input.value.trim());
    const settings = { ...state.settings, openrouterConfigured: true };
    await saveSettings(settings);
    patchState({ settings });
    notify("OpenRouter key stored securely", "success");
  }
  if (action === "delete-openrouter-key") {
    await deleteOpenRouterKey();
    const settings = { ...state.settings, openrouterConfigured: false };
    await saveSettings(settings);
    patchState({ settings });
    notify("OpenRouter key removed");
  }
  if (action === "test-openrouter") {
    const result = await testOpenRouter();
    notify(result.message, result.ok ? "success" : "danger");
  }
  if (action === "load-openrouter-models") {
    const models = await listOpenRouterModels();
    patchState({ openrouterModels: models });
    notify(`${models.length} OpenRouter models loaded`, "success");
  }
  if (action === "import-context-file") {
    const context = await importContext(state.activeProject.path);
    if (context) {
      patchState({ contexts: [...state.contexts, context], contextDialogOpen: false });
      notify("Context added", "success");
    }
  }
}

function readSettingsForm() {
  const checkedMode = document.querySelector('input[name="computeMode"]:checked')?.value;
  return {
    ...state.settings,
    computeMode: checkedMode ?? state.settings.computeMode,
    proxyQuality: document.querySelector("#proxy-quality")?.value ?? state.settings.proxyQuality,
    cacheLimitGb: Number(document.querySelector("#cache-limit")?.value ?? state.settings.cacheLimitGb),
    maxConcurrentJobs: Number(document.querySelector("#concurrent-jobs")?.value ?? state.settings.maxConcurrentJobs),
    whisperModel: document.querySelector("#whisper-model")?.value ?? state.settings.whisperModel,
    openrouterModel: document.querySelector("#openrouter-model")?.value.trim() || state.settings.openrouterModel,
  };
}

async function handleSubmit(form) {
  const data = new FormData(form);
  if (form.id === "create-project-form") {
    const project = await createProject({
      name: data.get("name").trim(),
      aspectRatio: data.get("aspectRatio"),
      resolution: data.get("resolution"),
      frameRate: Number(data.get("frameRate")),
    });
    const snapshot = await openProject(project.path);
    patchState({
      createProjectOpen: false,
      screen: "workspace",
      activeProject: snapshot.project,
      assets: snapshot.assets,
      scenes: snapshot.scenes,
      transcript: snapshot.transcript,
      contexts: snapshot.contexts,
      jobs: [],
      selectedAssetId: null,
    });
    startJobPolling();
  }
  if (form.id === "context-form") {
    const context = await addPastedContext(
      state.activeProject.path,
      data.get("name").trim(),
      data.get("content").trim(),
    );
    patchState({ contexts: [...state.contexts, context], contextDialogOpen: false });
    notify("Context added", "success");
  }
}

export function wireEvents() {
  document.querySelectorAll("[data-stop-propagation]").forEach((element) => {
    element.addEventListener("click", (event) => event.stopPropagation());
  });
  document.querySelectorAll("[data-action]").forEach((element) => {
    element.addEventListener("click", async (event) => {
      event.preventDefault();
      const action = element.dataset.action;
      try {
        await handleAction(action, element);
      } catch (error) {
        notify(error instanceof Error ? error.message : String(error), "danger");
      }
    });
  });
  document.querySelectorAll("form").forEach((form) => {
    form.addEventListener("submit", async (event) => {
      event.preventDefault();
      try {
        await handleSubmit(form);
      } catch (error) {
        notify(error instanceof Error ? error.message : String(error), "danger");
      }
    });
  });
  document.querySelector("#project-search")?.addEventListener("input", (event) => {
    state.projectQuery = event.target.value;
    const query = event.target.value.trim().toLowerCase();
    document.querySelectorAll(".project-item").forEach((project) => {
      project.hidden = !project.textContent.toLowerCase().includes(query);
    });
  });
  document.querySelector("#video-map-search")?.addEventListener("input", (event) => {
    state.videoMapQuery = event.target.value;
    const query = event.target.value.trim().toLowerCase();
    document.querySelectorAll(".transcript-row").forEach((row) => {
      row.hidden = !row.textContent.toLowerCase().includes(query);
    });
  });
}
