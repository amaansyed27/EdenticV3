import { toAssetUrl } from "../api.js";
import { escapeHtml, formatRelative } from "../format.js";
import { icon } from "../icons.js";

function projectVisual(project) {
  if (project.thumbnailPath) {
    return `<img src="${escapeHtml(toAssetUrl(project.thumbnailPath))}" alt="" />`;
  }
  return `
    <div class="project-placeholder" aria-hidden="true">
      <span class="project-placeholder-line"></span>
      <span class="project-placeholder-sun"></span>
      <span class="project-placeholder-horizon"></span>
    </div>`;
}

function projectCard(project) {
  return `
    <article class="project-item" data-project-path="${escapeHtml(project.path)}" tabindex="0">
      <button class="project-open-hitbox" type="button" data-action="open-project" data-project-path="${escapeHtml(project.path)}" aria-label="Open ${escapeHtml(project.name)}"></button>
      <div class="project-visual">${projectVisual(project)}</div>
      <div class="project-details">
        <div>
          <h3>${escapeHtml(project.name)}</h3>
          <p>${formatRelative(project.updatedAt)} · ${project.aspectRatio} · ${project.frameRate} fps</p>
        </div>
        <button class="icon-button project-menu" type="button" data-action="forget-project" data-project-id="${project.id}" aria-label="Remove ${escapeHtml(project.name)} from recent projects" title="Remove from recents">
          ${icon("close", 16)}
        </button>
      </div>
    </article>`;
}

export function renderHome(state) {
  const projects = state.projects.filter((project) =>
    project.name.toLowerCase().includes((state.projectQuery ?? "").toLowerCase()),
  );

  return `
    <main class="home-shell">
      <header class="home-header">
        <a class="brand-lockup" href="#" data-action="go-home" aria-label="Edentic home">
          <span class="brand-mark" aria-hidden="true"><i></i></span>
          <span>EDENTIC</span>
        </a>
        <div class="home-header-actions">
          <button class="button button-quiet" type="button" data-action="open-project-picker">
            ${icon("folderOpen", 17)} Open project
          </button>
          <button class="icon-button" type="button" data-action="open-settings" aria-label="Settings">
            ${icon("gear", 19)}
          </button>
        </div>
      </header>

      <section class="home-intro">
        <div>
          <p class="eyebrow">YOUR WORKSPACE</p>
          <h1>Continue creating.</h1>
          <p class="home-subtitle">Your projects, source media and edit data stay together.</p>
        </div>
        <button class="button button-primary button-large" type="button" data-action="new-project">
          ${icon("plus", 18)} New project
        </button>
      </section>

      <section class="projects-section" aria-labelledby="projects-title">
        <div class="section-toolbar">
          <div>
            <h2 id="projects-title">Recent projects</h2>
            <p>${projects.length} ${projects.length === 1 ? "project" : "projects"}</p>
          </div>
          <div class="project-tools">
            <label class="search-field compact">
              ${icon("search", 16)}
              <input id="project-search" type="search" placeholder="Search projects" value="${escapeHtml(state.projectQuery ?? "")}" />
            </label>
            <div class="segmented compact" aria-label="Project view">
              <button type="button" data-action="project-view" data-value="grid" class="${state.projectView === "grid" ? "active" : ""}" aria-label="Grid view">${icon("grid", 16)}</button>
              <button type="button" data-action="project-view" data-value="list" class="${state.projectView === "list" ? "active" : ""}" aria-label="List view">${icon("list", 16)}</button>
            </div>
          </div>
        </div>

        ${projects.length
          ? `<div class="projects-grid ${state.projectView === "list" ? "list-view" : ""}">${projects.map(projectCard).join("")}</div>`
          : `
            <div class="empty-home">
              <span class="empty-home-icon">${icon("film", 30)}</span>
              <h3>${state.projects.length ? "No matching projects" : "Create your first project"}</h3>
              <p>${state.projects.length ? "Try another project name." : "Choose a format, bring in footage and build your Video Map."}</p>
              ${state.projects.length ? "" : '<button class="button button-primary" type="button" data-action="new-project">New project</button>'}
            </div>`
        }
      </section>

      <footer class="home-footer">
        <span>${icon("folder", 15)} ${escapeHtml(state.settings.projectsRoot)}</span>
        <button class="text-button" type="button" data-action="change-projects-root">Change location</button>
      </footer>
    </main>`;
}

export function renderOnboarding(state) {
  return `
    <main class="onboarding">
      <div class="onboarding-brand">
        <span class="brand-mark large" aria-hidden="true"><i></i></span>
        <span>EDENTIC</span>
      </div>
      <section class="onboarding-content">
        <p class="step-count">WELCOME TO EDENTIC</p>
        <h1>Choose where your projects live.</h1>
        <p class="onboarding-copy">
          Edentic keeps source media, edit data, proxies and backups together in a managed project folder.
          You can export finished videos anywhere.
        </p>
        <button class="location-picker" type="button" data-action="choose-projects-root">
          <span class="location-icon">${icon("folder", 22)}</span>
          <span>
            <strong>${state.settings.projectsRoot ? escapeHtml(state.settings.projectsRoot) : "Choose projects folder"}</strong>
            <small>${state.settings.projectsRoot ? "Edentic will create new projects here" : "A folder such as Videos\\Edentic Projects works well"}</small>
          </span>
          ${icon("chevronDown", 18)}
        </button>
        <div class="onboarding-actions">
          <p>You can change this later in Settings.</p>
          <button class="button button-primary button-large" type="button" data-action="finish-onboarding" ${state.settings.projectsRoot ? "" : "disabled"}>
            Continue
          </button>
        </div>
      </section>
      <div class="onboarding-art" aria-hidden="true">
        <div class="onboarding-glow"></div>
        <div class="onboarding-horizon"></div>
      </div>
    </main>`;
}

export function renderCreateProject(state) {
  if (!state.createProjectOpen) return "";
  return `
    <div class="modal-layer" data-action="close-create-project">
      <form class="dialog create-project-dialog" id="create-project-form" data-stop-propagation>
        <div class="dialog-header">
          <div>
            <p class="eyebrow">NEW PROJECT</p>
            <h2>Project setup</h2>
          </div>
          <button class="icon-button" type="button" data-action="close-create-project" aria-label="Close">${icon("close", 19)}</button>
        </div>
        <label class="field">
          <span>Project name</span>
          <input name="name" required maxlength="80" autocomplete="off" placeholder="Untitled project" autofocus />
        </label>
        <div class="field-row three">
          <label class="field">
            <span>Format</span>
            <select name="aspectRatio">
              <option value="16:9">Landscape · 16:9</option>
              <option value="9:16">Vertical · 9:16</option>
              <option value="1:1">Square · 1:1</option>
            </select>
          </label>
          <label class="field">
            <span>Resolution</span>
            <select name="resolution">
              <option value="1920x1080">1080p</option>
              <option value="3840x2160">4K</option>
              <option value="1280x720">720p</option>
            </select>
          </label>
          <label class="field">
            <span>Frame rate</span>
            <select name="frameRate">
              <option value="30">30 fps</option>
              <option value="24">24 fps</option>
              <option value="25">25 fps</option>
              <option value="60">60 fps</option>
            </select>
          </label>
        </div>
        <div class="project-path-preview">
          ${icon("folder", 17)}
          <span>${escapeHtml(state.settings.projectsRoot)}\\<em>Project name</em></span>
        </div>
        <div class="dialog-footer">
          <button class="button button-quiet" type="button" data-action="close-create-project">Cancel</button>
          <button class="button button-primary" type="submit">Create project</button>
        </div>
      </form>
    </div>`;
}
