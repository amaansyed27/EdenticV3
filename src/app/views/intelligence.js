import { toAssetUrl } from "../api.js";
import { escapeHtml, formatBytes, formatDuration } from "../format.js";
import { icon } from "../icons.js";

export function intelligenceNav(state) {
  return `<div class="intelligence-nav" role="tablist" aria-label="Project intelligence">
    <button type="button" data-action="intelligence-view" data-value="source" class="${state.intelligenceView === "source" ? "active" : ""}">Source</button>
    <button type="button" data-action="intelligence-view" data-value="assistant" class="${state.intelligenceView === "assistant" ? "active" : ""}">Assistant</button>
    <button type="button" data-action="intelligence-view" data-value="plan" class="${state.intelligenceView === "plan" ? "active" : ""}">Plan${state.editPlans.length ? ` <span>${state.editPlans.length}</span>` : ""}</button>
  </div>`;
}

function activeConversation(state) {
  const id = state.assistantConversationId || state.conversations.at(-1)?.id || "";
  return { id, messages: state.messages.filter((message) => message.conversationId === id) };
}

function latestJob(state) {
  return [...state.aiJobs].reverse().find((job) =>
    ["queued", "running", "failed", "cancelled"].includes(job.status));
}

function sourceSelector(state) {
  if (!state.assets.length) return `<p class="assistant-source-empty">Import and prepare a source first.</p>`;
  return `<details class="assistant-sources">
    <summary>${state.assistantAssetIds.length} of ${state.assets.length} sources selected</summary>
    <div>${state.assets.map((asset) => `<label>
      <input type="checkbox" data-assistant-source="${asset.id}" ${state.assistantAssetIds.includes(asset.id) ? "checked" : ""} />
      <span>${escapeHtml(asset.name)}</span>
    </label>`).join("")}</div>
  </details>`;
}

export function renderAssistantPanel(state) {
  const conversation = activeConversation(state);
  const job = latestJob(state);
  const running = job && ["queued", "running"].includes(job.status);
  return `<div class="intelligence-panel-header">
      <div><p class="panel-kicker">ASSISTED EDIT PLANNING</p><h2>Project assistant</h2></div>
      <span class="remote-badge">OPENROUTER</span>
    </div>
    ${intelligenceNav(state)}
    <div class="assistant-thread">
      ${conversation.messages.length ? conversation.messages.map((message) => `<article class="assistant-message ${escapeHtml(message.role)}">
        <small>${message.role === "user" ? "You" : "Edentic"}</small>
        <p>${escapeHtml(message.content).replaceAll("\n", "<br />")}</p>
      </article>`).join("") : `<div class="assistant-welcome">
        ${icon("note", 26)}<h3>What should this edit become?</h3>
        <p>Describe the outcome, audience, tone and target length. Edentic will propose a reviewable plan; it will not edit the timeline in Slice 2.</p>
      </div>`}
      ${job ? `<div class="assistant-job ${job.status}">
        <div class="assistant-job-line"><span class="${running ? "spinner" : ""}"></span><strong>${escapeHtml(job.stage)}</strong><span>${Math.round(job.progress * 100)}%</span></div>
        <div class="assistant-job-progress"><i style="width:${Math.round(job.progress * 100)}%"></i></div>
        ${job.error ? `<p>${escapeHtml(job.error)}</p>` : ""}
        <div class="assistant-job-actions">
          ${running ? `<button class="text-button" type="button" data-action="cancel-ai-job" data-job-id="${job.id}">Cancel</button>` : ""}
          ${job.status === "failed" ? `<button class="text-button" type="button" data-action="retry-assistant">Retry with disclosure</button>` : ""}
        </div>
      </div>` : ""}
    </div>
    <form id="assistant-form" class="assistant-composer">
      ${sourceSelector(state)}
      <textarea id="assistant-prompt" name="prompt" rows="4" placeholder="Example: Turn this WinReclaim demo into a clear 2-minute product walkthrough. Remove waiting, explain the safety model, and suggest concise voiceover." ${running ? "disabled" : ""}>${escapeHtml(state.assistantDraft)}</textarea>
      <div><span>${icon("info", 14)} Context is disclosed before every remote request.</span>
      <button class="button button-primary" type="submit" ${running || !state.assets.length ? "disabled" : ""}>Create plan</button></div>
    </form>`;
}

function suggestions(title, values, render) {
  return values?.length ? `<div class="plan-suggestions"><strong>${title}</strong>${values.map(render).join("")}</div>` : "";
}

function decisionView(decision, index, total, plan) {
  return `<article class="plan-decision ${decision.enabled ? "" : "disabled"}">
    <div class="plan-decision-heading">
      <label class="decision-toggle"><input type="checkbox" data-plan-field="enabled" data-plan-id="${plan.id}" data-decision-id="${decision.id}" ${decision.enabled ? "checked" : ""} /><span>${String(index + 1).padStart(2, "0")}</span></label>
      <div><strong>${escapeHtml(decision.title)}</strong><small>${escapeHtml(decision.sourceName)} · ${escapeHtml(decision.action)}</small></div>
      <div class="decision-order">
        <button type="button" data-action="move-plan-decision" data-direction="-1" data-plan-id="${plan.id}" data-decision-id="${decision.id}" ${index === 0 ? "disabled" : ""} aria-label="Move earlier">${icon("chevronDown", 15)}</button>
        <button type="button" data-action="move-plan-decision" data-direction="1" data-plan-id="${plan.id}" data-decision-id="${decision.id}" ${index === total - 1 ? "disabled" : ""} aria-label="Move later">${icon("chevronDown", 15)}</button>
      </div>
    </div>
    <div class="plan-range">
      <label>Start <input type="number" min="0" step="0.01" value="${decision.start.toFixed(2)}" data-plan-field="start" data-plan-id="${plan.id}" data-decision-id="${decision.id}" /></label>
      <label>End <input type="number" min="0" step="0.01" value="${decision.end.toFixed(2)}" data-plan-field="end" data-plan-id="${plan.id}" data-decision-id="${decision.id}" /></label>
      <button class="button button-quiet" type="button" data-action="play-plan-range" data-asset-id="${decision.assetId}" data-start="${decision.start}" data-end="${decision.end}">${icon("play", 14)} Play</button>
    </div>
    <p class="plan-reason">${escapeHtml(decision.reason)}</p>
    <p class="plan-pacing"><strong>Pacing</strong> ${escapeHtml(decision.pacing)}</p>
    ${suggestions("Text overlays", decision.textOverlays, (item) => `<p><time>${formatDuration(item.start)}–${formatDuration(item.end)}</time>${escapeHtml(item.text)} <small>${escapeHtml(item.placement)}</small></p>`)}
    ${suggestions("Voiceover", decision.voiceoverSections, (item) => `<p><time>${formatDuration(item.start)}–${formatDuration(item.end)}</time>${escapeHtml(item.text)}</p>`)}
    ${decision.warnings?.length ? `<div class="plan-warnings">${decision.warnings.map((warning) => `<p>${icon("info", 14)} ${escapeHtml(warning)}</p>`).join("")}</div>` : ""}
    <div class="plan-decision-footer"><span>${Math.round(decision.confidence * 100)}% confidence</span>
      <button class="text-button" type="button" data-action="regenerate-plan-decision" data-plan-id="${plan.id}" data-decision-id="${decision.id}">${icon("refresh", 14)} Regenerate only this decision</button>
    </div>
  </article>`;
}

export function renderPlanPanel(state) {
  const plan = state.editPlans.find((value) => value.id === state.activePlanId) ?? state.editPlans.at(-1);
  if (!plan) return `<div class="intelligence-panel-header"><div><p class="panel-kicker">STRUCTURED EDIT PLAN</p><h2>Plan review</h2></div></div>
    ${intelligenceNav(state)}<div class="plan-empty">${icon("list", 28)}<h3>No edit plan yet</h3>
    <p>Ask the project assistant to create a plan from sources, semantic map, transcript and Context.</p>
    <button class="button button-primary" type="button" data-action="intelligence-view" data-value="assistant">Open assistant</button></div>`;
  const ordered = [...plan.decisions].sort((left, right) => left.orderIndex - right.orderIndex);
  return `<div class="intelligence-panel-header"><div><p class="panel-kicker">STRUCTURED EDIT PLAN</p><h2>Plan review</h2></div><span class="plan-status">${escapeHtml(plan.status)}</span></div>
    ${intelligenceNav(state)}
    <div class="plan-summary"><p>${escapeHtml(plan.assistantSummary)}</p><small>${ordered.filter((value) => value.enabled).length} enabled decisions · no timeline changes applied</small></div>
    <div class="plan-list">${ordered.map((value, index) => decisionView(value, index, ordered.length, plan)).join("")}</div>
    <div class="plan-review-footer">
      <button class="button button-quiet" type="button" data-action="set-plan-status" data-status="rejected" data-plan-id="${plan.id}">Reject plan</button>
      <button class="button button-primary" type="button" data-action="set-plan-status" data-status="accepted" data-plan-id="${plan.id}">${icon("check", 15)} Accept plan</button>
    </div>`;
}

export function renderRemoteDisclosure(state) {
  const preview = state.remotePreview;
  if (!state.remoteDisclosureOpen || !preview) return "";
  const action = preview.kind === "semantic" ? "Approve and analyze" : preview.kind === "decision" ? "Regenerate decision" : "Send and create plan";
  return `<div class="modal-layer remote-disclosure-layer" data-action="close-remote-disclosure">
    <section class="dialog remote-disclosure" data-stop-propagation>
      <div class="dialog-header"><div><p class="eyebrow">REMOTE REQUEST DISCLOSURE</p><h2>Review exactly what will be sent</h2></div>
      <button class="icon-button" type="button" data-action="close-remote-disclosure" aria-label="Close">${icon("close", 19)}</button></div>
      <div class="disclosure-boundary">${icon("info", 17)}<div><strong>The source video is not uploaded.</strong><p>Only the derived items listed below are included in this immutable request snapshot.</p></div></div>
      <dl class="disclosure-facts">
        <div><dt>Model</dt><dd>${escapeHtml(preview.model)}</dd></div><div><dt>Payload</dt><dd>${formatBytes(preview.estimatedBytes)}</dd></div>
        <div><dt>Sources</dt><dd>${preview.sources.length}</dd></div><div><dt>Frames</dt><dd>${preview.frames.length}</dd></div>
        <div><dt>Semantic segments</dt><dd>${preview.semanticSegments.length}</dd></div><div><dt>Transcript</dt><dd>${preview.transcript.length}</dd></div>
      </dl>
      <section class="disclosure-section"><h3>Instruction</h3><pre>${escapeHtml(preview.prompt)}</pre></section>
      <section class="disclosure-section"><h3>Selected sources</h3>${preview.sources.map((source) => `<p><time>${formatDuration(source.duration)}</time> ${escapeHtml(source.name)}</p>`).join("")}</section>
      <section class="disclosure-section"><h3>Frames and timestamps</h3>${preview.frames.length ? `<div class="disclosure-frames">${preview.frames.map((frame) => `<figure><img src="${escapeHtml(toAssetUrl(frame.path))}" alt="" /><figcaption><strong>${formatDuration(frame.timestamp)}</strong>${escapeHtml(frame.reason)}</figcaption></figure>`).join("")}</div>` : "<p>No visual frames are included.</p>"}</section>
      <section class="disclosure-section"><h3>Semantic map</h3>${preview.semanticSegments.length ? preview.semanticSegments.map((segment) => `<p><time>${formatDuration(segment.start)}–${formatDuration(segment.end)}</time> <strong>${escapeHtml(segment.title)}</strong> · ${escapeHtml(segment.description)}</p>`).join("") : "<p>No existing semantic segments are included.</p>"}</section>
      <section class="disclosure-section"><h3>Transcript</h3>${preview.transcript.length ? preview.transcript.map((segment) => `<p><time>${formatDuration(segment.start)}</time> ${escapeHtml(segment.text)}</p>`).join("") : "<p>No transcript text is included.</p>"}</section>
      <section class="disclosure-section"><h3>Project Context</h3>${preview.contexts.length ? preview.contexts.map((context) => `<details><summary>${escapeHtml(context.name)}</summary><pre>${escapeHtml(context.content)}</pre></details>`).join("") : "<p>No saved Context is included.</p>"}</section>
      ${preview.firstApprovalRequired ? `<label class="first-remote-approval"><input id="first-remote-approval" type="checkbox" /><span>I approve Edentic’s first remote semantic analysis using only the disclosed derived data.</span></label>` : ""}
      <div class="dialog-footer"><button class="button button-quiet" type="button" data-action="close-remote-disclosure" ${state.semanticBusy ? "disabled" : ""}>Cancel</button>
      <button class="button button-primary" type="button" data-action="confirm-remote-request" ${state.semanticBusy ? "disabled" : ""}>${state.semanticBusy ? "Analyzing…" : action}</button></div>
    </section>
  </div>`;
}
