import type { Config } from "@docusaurus/types";
import type { Options as PresetOptions } from "@docusaurus/preset-classic";
import type { Options as LocalSearchOptions } from "@easyops-cn/docusaurus-search-local";

const config: Config = {
  title: "Sim Documentation",
  tagline: "The fast, collaborative AI-powered code editor",
  favicon: "theme/favicon.png",
  url: "https://sim.dev",
  baseUrl: "/docs/",
  organizationName: "simtropolis",
  projectName: "sim",
  trailingSlash: false,
  onBrokenLinks: "throw",
  onBrokenMarkdownLinks: "warn",
  markdown: {
    mermaid: true,
  },
  themes: ["@docusaurus/theme-mermaid"],
  presets: [
    [
      "classic",
      {
        docs: {
          path: "src",
          routeBasePath: "/",
          sidebarPath: "./sidebars.ts",
          exclude: ["SUMMARY.md"],
          editUrl: "https://github.com/simtropolis/sim/edit/main/docs/src/",
          showLastUpdateAuthor: true,
          showLastUpdateTime: true,
        },
        blog: {
          path: "blog",
          routeBasePath: "blog",
          showReadingTime: true,
          blogSidebarTitle: "All posts",
          blogSidebarCount: "ALL",
          editUrl: "https://github.com/simtropolis/sim/edit/main/docs/blog/",
        },
        theme: {
          customCss: "./src/css/custom.css",
        },
      } satisfies PresetOptions,
    ],
  ],
  plugins: [
    [
      "@easyops-cn/docusaurus-search-local",
      {
        hashed: true,
        indexDocs: true,
        indexBlog: true,
        docsRouteBasePath: "/",
        language: ["en"],
      } satisfies LocalSearchOptions,
    ],
    [
      "@docusaurus/plugin-content-docs",
      {
        id: "tutorials",
        path: "tutorials",
        routeBasePath: "tutorials",
        sidebarPath: "./tutorials/sidebars.ts",
        editUrl: "https://github.com/simtropolis/sim/edit/main/docs/tutorials/",
        showLastUpdateAuthor: true,
        showLastUpdateTime: true,
      },
    ],
  ],
  themeConfig: {
    image: "theme/favicon.png",
    navbar: {
      title: "Sim",
      logo: {
        alt: "Sim",
        src: "theme/favicon.png",
      },
      items: [
        { to: "/", label: "Docs", position: "left" },
        { to: "/ai/overview", label: "AI", position: "left" },
        { to: "/extensions", label: "Extensions", position: "left" },
        { to: "/tutorials", label: "Tutorials", position: "left" },
        { to: "/development", label: "Developers", position: "left" },
        { to: "/blog", label: "Blog", position: "left" },
        {
          href: "https://github.com/simtropolis/sim",
          label: "GitHub",
          position: "right",
        },
      ],
    },
    footer: {
      style: "dark",
      links: [
        {
          title: "Docs",
          items: [
            { label: "Getting Started", to: "/getting-started" },
            { label: "Installation", to: "/installation" },
            { label: "Troubleshooting", to: "/troubleshooting" },
            { label: "Tutorials", to: "/tutorials" },
          ],
        },
        {
          title: "Agent",
          items: [
            { label: "AI Overview", to: "/ai/overview" },
            { label: "Agent Panel", to: "/ai/agent-panel" },
            { label: "MCP", to: "/ai/mcp" },
          ],
        },
        {
          title: "Community",
          items: [
            { label: "GitHub", href: "https://github.com/simtropolis/sim" },
          ],
        },
      ],
      copyright: `Copyright © ${new Date().getFullYear()} Sim Industries.`,
    },
    prism: {
      additionalLanguages: ["bash", "diff", "json", "rust", "toml"],
    },
  },
};

export default config;
