---
title: Diff
description: "Configure Diff language support in Sim, including language servers, formatting, and debugging."
---

# Diff

Diff support is available natively in Sim.

- Tree-sitter: [simtropolis/the-mikedavis/tree-sitter-diff](https://github.com/the-mikedavis/tree-sitter-diff)

## Configuration

Sim will not attempt to format diff files and has [`remove_trailing_whitespace_on_save`](https://sim.dev/docs/reference/all-settings#remove-trailing-whitespace-on-save) and [`ensure-final-newline-on-save`](https://sim.dev/docs/reference/all-settings#ensure-final-newline-on-save) set to false.

Sim will automatically recognize files with `patch` and `diff` extensions as Diff files. To recognize other extensions, add them to `file_types` in your Sim settings.json:

```json [settings]
  "file_types": {
    "Diff": ["dif"]
  },
```
