# Skill audit for repository setup

On 9 November 2026, the local skill catalog and community candidates were checked before repository and site work. The community repository paths were confirmed through GitHub trees: `anthropics/skills/skills/frontend-design`, `vercel-labs/agent-skills/skills/web-design-guidelines`, `olehsvyrydov/ai-development-team/claude/skills/specialized/technical-writer`, `ucdavis/ai-skills-registry/skills/documentation-readme`, and `udaysharmadev/skills/skills/frontpage`. Only the small README skill was installed after reading its file and confirming its folder contains no scripts. The Vercel guidelines were read from their current upstream source for a site review. No candidate script was executed.

| Skill | Source | Actual use |
| --- | --- | --- |
| `ponytail` | Installed local skill | Import existing crates and keep site/build logic small. |
| `documentation-readme` | Installed from `ucdavis/ai-skills-registry` | Check README purpose, installation, usage, configuration, dependencies, and contact. Its generic-placeholder suggestion was skipped because the project requires factual claims. |
| `frontend-ui-engineering` | Installed local skill | Responsive landing page, semantic links, accessible focus, specific visual identity. |
| `web-design-guidelines` | Read from `vercel-labs/agent-skills` | Check semantics, reduced motion, image dimensions, focus, and dark color scheme. |
| `webapp-testing` | Installed local skill | Playwright desktop/mobile, docs navigation, keyboard, and console smoke. |
| `debugging-and-error-recovery` | Installed local skill | Diagnose partial-workspace lockfile and docs build/link issues before continuing. |
| `code-review-and-quality` | Installed local skill | Review release gate, document claims, dependency lockfiles, and site paths before merge. |
| `computer-use` | Installed local skill | Launch the actual Windows executable, inspect its accessibility tree, and capture an unaltered screenshot. |
| `skill-installer` | Installed system skill | Install the inspected README skill from its verified GitHub path. |

The other candidate skills were not installed because the local frontend skill and short README skill covered the requested work. Rust and GitHub behavior was checked through local commands, current official documentation, and real GitHub Actions runs.
