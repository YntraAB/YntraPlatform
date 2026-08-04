import { defineConfig } from 'astro/config';
import starlight from '@astrojs/starlight';

export default defineConfig({
  integrations: [
    starlight({
      title: 'Yntra Platform Documentation',
      description: 'Dynamic Modular Workspace Engine - Local-First Tripartite Architecture Docs',
      logo: {
        src: './src/assets/logo.svg',
        replacesTitle: false,
      },
      components: {
        SiteTitle: './src/components/SiteTitle.astro',
      },
      social: {
        github: 'https://github.com/YntraAB/YntraPlatform',
      },
      head: [],
      sidebar: [
        {
          label: '01. Getting Started',
          autogenerate: { directory: 'wiki/01-getting-started' },
        },
        {
          label: '02. Core Engine',
          autogenerate: { directory: 'wiki/02-core-engine' },
        },
        {
          label: '03. API & FFI Reference',
          autogenerate: { directory: 'wiki/03-api-and-ffi-reference' },
        },
        {
          label: '04. Functional Blocks & UI',
          autogenerate: { directory: 'wiki/04-functional-blocks' },
        },
        {
          label: '05. Enterprise & Security',
          autogenerate: { directory: 'wiki/05-enterprise-and-security' },
        },
        {
          label: '06. Operations & Deployment',
          autogenerate: { directory: 'wiki/06-operations-and-deployment' },
        },
        {
          label: '07. Troubleshooting',
          autogenerate: { directory: 'wiki/07-troubleshooting' },
        },
      ],
      customCss: [
        './src/styles/custom.css',
      ],
    }),
  ],
});
