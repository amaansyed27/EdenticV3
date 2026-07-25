import { demoBootstrap, demoProject } from "./demo.js";

const isTauri = () => Boolean(window.__TAURI__?.core?.invoke);
const demoMode = () => new URLSearchParams(location.search).has("demo") || !isTauri();

export function toAssetUrl(path) {
  if (!path) return "";
  if (isTauri() && window.__TAURI__.core.convertFileSrc) {
    return window.__TAURI__.core.convertFileSrc(path);
  }
  return path;
}

async function invoke(command, args = {}) {
  if (!isTauri()) throw new Error("This action requires the Edentic desktop app.");
  return window.__TAURI__.core.invoke(command, args);
}

function demoRecovery(message) {
  return { message, projects: 0, files: 0, bytes: 0, warnings: [] };
}

export async function getBootstrap() {
  return demoMode() ? structuredClone(demoBootstrap) : invoke("get_bootstrap");
}

export async function completeOnboarding(projectsRoot) {
  if (demoMode()) return structuredClone(demoBootstrap);
  return invoke("complete_onboarding", { projectsRoot });
}

export async function chooseProjectsRoot() {
  if (demoMode()) return "C:\\Users\\Amaan\\Videos\\Edentic Projects";
  return invoke("choose_projects_root");
}

export async function saveSettings(settings) {
  if (demoMode()) return settings;
  return invoke("save_settings", { settings });
}

export async function createProject(input) {
  if (demoMode()) {
    return { ...demoBootstrap.projects[0], ...input, id: crypto.randomUUID(), path: `${demoBootstrap.settings.projectsRoot}\\${input.name}` };
  }
  return invoke("create_project", { input });
}

export async function openProject(projectPath) {
  if (demoMode()) return structuredClone(demoProject);
  return invoke("open_project", { projectPath });
}

export async function pickProject() {
  if (demoMode()) return structuredClone(demoProject);
  return invoke("pick_project");
}

export async function forgetProject(projectId) {
  if (demoMode()) return true;
  return invoke("forget_project", { projectId });
}

export async function importMedia(projectPath) {
  if (demoMode()) return structuredClone(demoProject.assets);
  return invoke("import_media", { projectPath });
}

export async function deleteMediaAsset(projectPath, assetId) {
  if (demoMode()) {
    const snapshot = structuredClone(demoProject);
    snapshot.assets = snapshot.assets.filter((asset) => asset.id !== assetId);
    snapshot.scenes = snapshot.scenes.filter((scene) => scene.assetId !== assetId);
    snapshot.transcript = snapshot.transcript.filter((segment) => segment.assetId !== assetId);
    return snapshot;
  }
  return invoke("delete_media_asset", { projectPath, assetId });
}

export async function importContext(projectPath) {
  if (demoMode()) return structuredClone(demoProject.contexts[0]);
  return invoke("import_context_file", { projectPath });
}

export async function addPastedContext(projectPath, name, content) {
  if (demoMode()) return { id: crypto.randomUUID(), name, content, source: "pasted", createdAt: new Date().toISOString() };
  return invoke("add_pasted_context", { projectPath, name, content });
}

export async function startIndex(projectPath, assetId) {
  if (demoMode()) return { id: crypto.randomUUID(), assetId, status: "ready", progress: 1, stage: "Complete" };
  return invoke("start_index", { projectPath, assetId });
}

export async function getProjectSnapshot(projectPath) {
  if (demoMode()) return structuredClone(demoProject);
  return invoke("get_project_snapshot", { projectPath });
}

export async function getIndexJobs(projectPath) {
  if (demoMode()) return [];
  return invoke("get_index_jobs", { projectPath });
}

export async function cancelIndexJob(jobId) {
  if (demoMode()) return true;
  return invoke("cancel_index_job", { jobId });
}

export async function saveOpenRouterKey(apiKey) {
  if (demoMode()) return { configured: true };
  return invoke("save_openrouter_key", { apiKey });
}

export async function deleteOpenRouterKey() {
  if (demoMode()) return { configured: false };
  return invoke("delete_openrouter_key");
}

export async function testOpenRouter() {
  if (demoMode()) return { ok: true, message: "Connected to OpenRouter", modelCount: 314 };
  return invoke("test_openrouter");
}

export async function listOpenRouterModels() {
  if (demoMode()) {
    return [
      { id: "openrouter/free", name: "Free Models Router", contextLength: 200000, isFree: true, inputModalities: ["text", "image"], supportedParameters: ["response_format"] },
      { id: "qwen/qwen3.5-9b:free", name: "Qwen 3.5 9B (free)", contextLength: 131072, isFree: true, inputModalities: ["text"], supportedParameters: ["response_format"] },
    ];
  }
  return invoke("list_openrouter_models");
}

function desktopOnly() {
  if (demoMode()) throw new Error("Slice 2 analysis requires the Edentic desktop app.");
}

export async function buildLocalSemanticMap(projectPath, assetIds) {
  desktopOnly(); return invoke("build_local_semantic_map", { projectPath, assetIds });
}
export async function prepareSemanticAnalysis(projectPath, assetIds) {
  desktopOnly(); return invoke("prepare_semantic_analysis", { projectPath, assetIds });
}
export async function runSemanticAnalysis(projectPath, previewId, approveFirst) {
  desktopOnly(); return invoke("run_semantic_analysis", { projectPath, previewId, approveFirst });
}
export async function prepareAssistantRequest(projectPath, prompt, assetIds, conversationId) {
  desktopOnly(); return invoke("prepare_assistant_request", { projectPath, prompt, assetIds, conversationId });
}
export async function prepareDecisionRegeneration(projectPath, planId, decisionId) {
  desktopOnly(); return invoke("prepare_decision_regeneration", { projectPath, planId, decisionId });
}
export async function startAssistantRequest(projectPath, previewId) {
  desktopOnly(); return invoke("start_assistant_request", { projectPath, previewId });
}
export async function getAiJobs(projectPath) {
  desktopOnly(); return invoke("get_ai_jobs", { projectPath });
}
export async function cancelAiJob(jobId) {
  desktopOnly(); return invoke("cancel_ai_job", { jobId });
}
export async function saveEditPlan(projectPath, plan) {
  desktopOnly(); return invoke("save_edit_plan", { projectPath, plan });
}
export async function setEditPlanStatus(projectPath, planId, status) {
  desktopOnly(); return invoke("set_edit_plan_status", { projectPath, planId, status });
}

export async function resetSettings() {
  if (demoMode()) return demoRecovery("Settings restored to defaults");
  return invoke("reset_settings");
}

export async function resetAppData() {
  if (demoMode()) return demoRecovery("App data cleared · project folders were preserved");
  return invoke("reset_app_data");
}

export async function resetCache() {
  if (demoMode()) return demoRecovery("Generated cache cleared");
  return invoke("reset_cache");
}

export async function repairApp() {
  if (demoMode()) return demoRecovery("Repair completed");
  return invoke("repair_app");
}

export async function resetAll() {
  if (demoMode()) return demoRecovery("Edentic reset complete · onboarding will open next");
  return invoke("reset_all");
}
