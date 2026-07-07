---
title: Helm
description: "Configure Helm language support in Sim, including language servers, formatting, and debugging."
---

# Helm

Support for Helm in Sim is provided by the community-maintained [Helm extension](https://github.com/cabrinha/helm.sim).

- Tree-sitter: [tree-sitter-go-template](https://github.com/ngalaiko/tree-sitter-go-template/tree/master)
- Language Server: [mrjosh/helm-ls](https://github.com/mrjosh/helm-ls)

## Setup

Enable Helm language for Helm files by editing your `.sim/settings.json` and adding:

```json [settings]
  "file_types": {
    "Helm": [
      "**/templates/**/*.tpl",
      "**/templates/**/*.yaml",
      "**/templates/**/*.yml",
      "**/helmfile.d/**/*.yaml",
      "**/helmfile.d/**/*.yml",
      "**/values*.yaml"
    ]
  }
```

This will also mark values.yaml files as the type helm, since helm-ls supports this.
