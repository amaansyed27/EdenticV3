# Edentic working agreement

Edentic is developed one verified product slice per conversation.

## Required workflow

1. Inspect the current repository and recent commits before changing code.
2. State the active slice and do not expand into later slices.
3. Implement real end-to-end behavior. Do not add fake controls or placeholder success states.
4. Keep source files modular. Split files when they mix unrelated responsibilities.
5. Run the checks that are possible in the active environment.
6. Give the creator a focused manual test checklist.
7. Fix every bug found in that slice before preparing the next handoff.
8. At approval, write a verified continuation summary and a complete prompt for the next chat.

## Product invariants

- Source media is copied into managed projects by default and never modified in place.
- Project state survives restart.
- Derived artifacts are rebuildable.
- Local processing must remain useful without a model provider.
- Remote providers receive project-derived context only after clear disclosure.
- Credentials belong in the operating-system credential vault.
- Hardware and locality labels must describe what actually happened.
- Manual, Assisted and Agentic will share one project and timeline model.

## Interface rules

- Dark theme: warm dark gray/light black.
- Light theme: beige-white with goldish-yellow accents.
- No blue product theme.
- No decorative gradients.
- Avoid cards nested inside cards, pill-heavy interfaces and generic SaaS dashboards.
- Use spacing, typography, subtle surface changes and thin dividers for hierarchy.
- The working editor should feel professional like a major NLE and inviting like Lovable.

## Current boundary

Slice 1 includes setup, project home, managed project folders, source import, local Video Map, settings and OpenRouter BYOK foundations. Timeline editing and the Manual/Assisted/Agentic modes are later slices.

