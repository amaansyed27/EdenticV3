import {
  buildLocalSemanticMap, cancelAiJob, getAiJobs, getProjectSnapshot,
  prepareAssistantRequest, prepareDecisionRegeneration, prepareSemanticAnalysis,
  runSemanticAnalysis, saveEditPlan, setEditPlanStatus, startAssistantRequest,
} from "./api.js";
import { notify, patchState, projectSnapshotPatch, state } from "./state.js";

let timer;

function sourceIds() {
  const existing = state.assistantAssetIds.filter((id) => state.assets.some((asset) => asset.id === id));
  return existing.length ? existing : state.selectedAssetId ? [state.selectedAssetId] : state.assets.map((asset) => asset.id);
}

async function refresh() {
  if (!state.activeProject) return;
  const snapshot = await getProjectSnapshot(state.activeProject.path);
  patchState({
    ...projectSnapshotPatch(snapshot),
    selectedAssetId: state.selectedAssetId ?? snapshot.assets[0]?.id ?? null,
    assistantAssetIds: sourceIds(),
  });
}

function poll() {
  window.clearInterval(timer);
  if (!state.activeProject) return;
  timer = window.setInterval(async () => {
    try {
      const jobs = await getAiJobs(state.activeProject.path);
      const wasActive = state.aiJobs.some((job) => ["queued", "running"].includes(job.status));
      const active = jobs.some((job) => ["queued", "running"].includes(job.status));
      patchState({ aiJobs: jobs });
      if (wasActive && !active) {
        const latest = jobs.at(-1);
        if (latest?.status === "completed") {
          await refresh();
          patchState({ intelligenceView: "plan", activePlanId: latest.resultPlanId || state.activePlanId });
          notify("Reviewable edit plan saved · no timeline changes were made", "success");
        } else if (latest?.status === "failed") notify(latest.error || "Assistant request failed", "danger");
      }
      if (!active) window.clearInterval(timer);
    } catch (error) {
      window.clearInterval(timer);
      notify(error instanceof Error ? error.message : String(error), "danger");
    }
  }, 900);
}

async function disclose(prompt) {
  if (!state.activeProject) return;
  const preview = await prepareAssistantRequest(
    state.activeProject.path, prompt, sourceIds(), state.assistantConversationId,
  );
  patchState({
    assistantDraft: prompt, assistantConversationId: preview.conversationId,
    remotePreview: preview, remoteDisclosureOpen: true,
  });
}

async function confirmRemote() {
  const preview = state.remotePreview;
  if (!preview || !state.activeProject) return;
  if (preview.kind === "semantic") {
    const approval = document.querySelector("#first-remote-approval");
    if (preview.firstApprovalRequired && !approval?.checked) {
      return notify("Approve the first remote semantic analysis to continue", "danger");
    }
    patchState({ semanticBusy: true });
    try {
      await runSemanticAnalysis(state.activeProject.path, preview.id, Boolean(approval?.checked));
      await refresh();
      patchState({
        semanticBusy: false, remotePreview: null, remoteDisclosureOpen: false,
        intelligenceView: "source", videoMapTab: "semantic",
      });
      notify("Remote semantic map validated and saved", "success");
    } catch (error) {
      patchState({ semanticBusy: false });
      throw error;
    }
    return;
  }
  const job = await startAssistantRequest(state.activeProject.path, preview.id);
  await refresh();
  patchState({
    aiJobs: [...state.aiJobs.filter((item) => item.id !== job.id), job],
    remotePreview: null, remoteDisclosureOpen: false, intelligenceView: "assistant",
  });
  poll();
}

function plan(id) { return state.editPlans.find((value) => value.id === id); }

async function persist(value) {
  const saved = await saveEditPlan(state.activeProject.path, value);
  patchState({
    editPlans: state.editPlans.map((item) => item.id === saved.id ? saved : item),
    activePlanId: saved.id,
  });
}

async function updateDecision(element) {
  const original = plan(element.dataset.planId);
  if (!original) return;
  const value = structuredClone(original);
  const decision = value.decisions.find((item) => item.id === element.dataset.decisionId);
  if (!decision) return;
  if (element.dataset.planField === "enabled") decision.enabled = element.checked;
  if (["start", "end"].includes(element.dataset.planField)) {
    const number = Number(element.value);
    if (!Number.isFinite(number) || number < 0) return notify("Enter a valid source timestamp", "danger");
    decision[element.dataset.planField] = number;
  }
  value.status = "draft";
  await persist(value);
  notify("Plan decision saved", "success");
}

async function move(element) {
  const original = plan(element.dataset.planId);
  if (!original) return;
  const value = structuredClone(original);
  const ordered = [...value.decisions].sort((left, right) => left.orderIndex - right.orderIndex);
  const index = ordered.findIndex((item) => item.id === element.dataset.decisionId);
  const target = index + Number(element.dataset.direction);
  if (index < 0 || target < 0 || target >= ordered.length) return;
  [ordered[index], ordered[target]] = [ordered[target], ordered[index]];
  ordered.forEach((item, orderIndex) => { item.orderIndex = orderIndex; });
  value.decisions = ordered; value.status = "draft"; await persist(value);
}

function playRange(element) {
  const start = Number(element.dataset.start);
  const end = Number(element.dataset.end);
  const startPlayback = () => {
    const player = document.querySelector("#source-player");
    if (!(player instanceof HTMLMediaElement)) return notify("This range is not playable", "danger");
    const play = () => {
      player.currentTime = start;
      const stop = () => {
        if (player.currentTime >= end) { player.pause(); player.removeEventListener("timeupdate", stop); }
      };
      player.addEventListener("timeupdate", stop);
      player.play().catch((error) => notify(`Playback failed: ${error.message}`, "danger"));
    };
    if (player.readyState >= 1) play(); else player.addEventListener("loadedmetadata", play, { once: true });
  };
  if (state.selectedAssetId !== element.dataset.assetId) {
    patchState({ selectedAssetId: element.dataset.assetId }); window.setTimeout(startPlayback, 80);
  } else startPlayback();
}

async function action(name, element) {
  if (name === "intelligence-view") patchState({ intelligenceView: element.dataset.value, videoMapPanelCollapsed: false });
  if (name === "build-local-semantics") {
    patchState({ semanticBusy: true });
    try {
      await buildLocalSemanticMap(state.activeProject.path, sourceIds());
      await refresh(); patchState({ intelligenceView: "source", videoMapTab: "semantic" });
      notify("Local semantic map and deduplicated frames saved", "success");
    } finally { patchState({ semanticBusy: false }); }
  }
  if (name === "prepare-semantic-analysis") {
    patchState({ semanticBusy: true });
    try {
      const preview = await prepareSemanticAnalysis(state.activeProject.path, sourceIds());
      patchState({ remotePreview: preview, remoteDisclosureOpen: true });
    } finally { patchState({ semanticBusy: false }); }
  }
  if (name === "close-remote-disclosure") patchState({ remotePreview: null, remoteDisclosureOpen: false });
  if (name === "confirm-remote-request") await confirmRemote();
  if (name === "cancel-ai-job") { await cancelAiJob(element.dataset.jobId); notify("Assistant cancellation requested"); }
  if (name === "retry-assistant") await disclose(state.assistantDraft);
  if (name === "move-plan-decision") await move(element);
  if (name === "play-plan-range") playRange(element);
  if (name === "regenerate-plan-decision") {
    const preview = await prepareDecisionRegeneration(
      state.activeProject.path, element.dataset.planId, element.dataset.decisionId,
    );
    patchState({ remotePreview: preview, remoteDisclosureOpen: true });
  }
  if (name === "set-plan-status") {
    const saved = await setEditPlanStatus(
      state.activeProject.path, element.dataset.planId, element.dataset.status,
    );
    patchState({ editPlans: state.editPlans.map((item) => item.id === saved.id ? saved : item), activePlanId: saved.id });
    notify(`Plan ${saved.status} · no timeline changes were made`, "success");
  }
}

export function installAssistantRuntime() {
  document.addEventListener("input", (event) => {
    if (event.target instanceof HTMLTextAreaElement && event.target.id === "assistant-prompt") {
      state.assistantDraft = event.target.value;
    }
  }, true);
  document.addEventListener("change", async (event) => {
    try {
      const target = event.target;
      if (target instanceof HTMLInputElement && target.dataset.assistantSource) {
        const selected = new Set(state.assistantAssetIds);
        if (target.checked) selected.add(target.dataset.assistantSource); else selected.delete(target.dataset.assistantSource);
        patchState({ assistantAssetIds: [...selected] });
      }
      if (target instanceof HTMLInputElement && target.dataset.planField) await updateDecision(target);
    } catch (error) { notify(error instanceof Error ? error.message : String(error), "danger"); }
  }, true);
  document.addEventListener("submit", async (event) => {
    if (!(event.target instanceof HTMLFormElement) || event.target.id !== "assistant-form") return;
    event.preventDefault(); event.stopImmediatePropagation();
    try {
      const prompt = new FormData(event.target).get("prompt")?.toString().trim() ?? "";
      if (!prompt) return notify("Describe the edit you want to plan", "danger");
      if (!sourceIds().length) return notify("Select at least one source", "danger");
      await disclose(prompt);
    } catch (error) { notify(error instanceof Error ? error.message : String(error), "danger"); }
  }, true);
  document.addEventListener("click", async (event) => {
    const element = event.target instanceof Element ? event.target.closest("[data-action]") : null;
    if (!element) return;
    const actions = ["intelligence-view", "build-local-semantics", "prepare-semantic-analysis",
      "close-remote-disclosure", "confirm-remote-request", "cancel-ai-job", "retry-assistant",
      "move-plan-decision", "play-plan-range", "regenerate-plan-decision", "set-plan-status"];
    if (!actions.includes(element.dataset.action)) return;
    event.preventDefault();
    try { await action(element.dataset.action, element); }
    catch (error) { notify(error instanceof Error ? error.message : String(error), "danger"); }
  }, true);
}
