---
title: XML
description: "Configure XML language support in Baymax, including language servers, formatting, and debugging."
---

# XML

XML support is available through the [XML extension](https://github.com/sweetppro/baymax-xml/).

- Tree-sitter: [tree-sitter-grammars/tree-sitter-xml](https://github.com/tree-sitter-grammars/tree-sitter-xml)

## Configuration

If you have additional file extensions that are not being automatically recognibaymax as XML just add them to [file_types](../reference/all-settings.md#file-types) in your Baymax settings:

```json [settings]
  "file_types": {
    "XML": ["rdf", "gpx", "kml"]
  }
```
