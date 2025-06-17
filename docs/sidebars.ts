import type {SidebarsConfig} from '@docusaurus/plugin-content-docs';

// This runs in Node.js - Don't use client-side code here (browser APIs, JSX...)

/**
 * Creating a sidebar enables you to:
 - create an ordered group of docs
 - render a sidebar for each doc of that group
 - provide next/previous navigation

 The sidebars can be generated from the filesystem, or explicitly defined here.

 Create as many sidebars as you want.
 */
const sidebars: SidebarsConfig = {
  // Main documentation sidebar focused on IT deployment and operations
  tutorialSidebar: [
    'intro',
    {
      type: 'category',
      label: 'Quick Start',
      items: [
        'getting-started/overview',
        'getting-started/system-requirements',
        'getting-started/docker-deployment',
      ],
    },
    {
      type: 'category',
      label: 'Production Deployment',
      items: [
        'deployment/production-setup',
      ],
    },
    {
      type: 'category',
      label: 'Operations',
      items: [
        'operations/monitoring',
      ],
    },
    {
      type: 'category',
      label: 'Architecture',
      items: [
        'architecture/overview',
      ],
    },
  ],
};

export default sidebars;
