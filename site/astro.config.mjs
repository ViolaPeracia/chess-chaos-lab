import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  site: 'https://violaperacia.github.io',
  base: '/chess-chaos-lab',
  integrations: [starlight({
    title: 'Chess Chaos Lab',
    description: 'Guides and engineering notes for the local Rust chess desktop app.',
    social: [{ icon: 'github', label: 'GitHub', href: 'https://github.com/ViolaPeracia/chess-chaos-lab' }],
    sidebar: [
      { label: 'Start', items: [{ label: 'Documentation', slug: 'docs' }, { label: 'User guide', slug: 'docs/user-guide' }, { label: 'Troubleshooting', slug: 'docs/troubleshooting' }] },
      { label: 'Product', items: [{ label: 'Features', slug: 'docs/feature-matrix' }, { label: 'Limitations', slug: 'docs/known-limitations' }, { label: 'Roadmap', slug: 'docs/roadmap' }] },
      { label: 'Engineering', items: [{ label: 'Architecture', slug: 'docs/architecture' }, { label: 'Build', slug: 'docs/build' }, { label: 'Testing', slug: 'docs/testing' }, { label: 'Performance', slug: 'docs/performance' }, { label: 'UCI engines', slug: 'docs/engine-integration' }, { label: 'Offline mode', slug: 'docs/offline-mode' }, { label: 'Releases', slug: 'docs/release' }, { label: 'Security', slug: 'docs/security' }] }
    ],
    customCss: ['./src/styles/docs.css']
  })]
});
