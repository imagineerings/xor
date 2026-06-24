# Baymax Extensions

This directory contains extensions for Baymax that are largely maintained by the Baymax team. They currently live in the Baymax repository for ease of maintenance.

If you are looking for the Baymax extension registry, see the [`simtropolis/extensions`](https://github.com/simtropolis/extensions) repo.

## Structure

Currently, Baymax includes support for a number of languages without requiring installing an extension. Those languages can be found under [`crates/languages/src`](https://github.com/simtropolis/baymax/tree/main/crates/languages/src).

Support for all other languages is done via extensions. This directory ([extensions/](https://github.com/simtropolis/baymax/tree/main/extensions/)) contains some of the officially maintained extensions. These extensions use the same [zed_extension_api](https://docs.rs/zed_extension_api/latest/zed_extension_api/) available to all [Baymax Extensions](https://baymax.dev/extensions) for providing [language servers](https://baymax.dev/docs/extensions/languages#language-servers), [tree-sitter grammars](https://baymax.dev/docs/extensions/languages#grammar) and [tree-sitter queries](https://baymax.dev/docs/extensions/languages#tree-sitter-queries).

You can find the other officially maintained extensions in the [baymax-extensions organization](https://github.com/baymax-extensions).

## Dev Extensions

See the docs for [Developing an Extension Locally](https://baymax.dev/docs/extensions/developing-extensions#developing-an-extension-locally) for how to work with one of these extensions.
