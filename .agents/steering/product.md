# Product

This repository (internally "sim") is the source for **Sim**, a high-performance, GPU-accelerated, multiplayer code editor built in Rust by Sim Industries. Sim targets macOS, Linux, and Windows.

Key product areas:
- **Code editor** — fast text editing, LSP integration, syntax highlighting via Tree-sitter, vim mode
- **AI / agent features** — inline edit predictions (Zeta), an agentic coding assistant (`crates/agent*`), and integrations with many LLM providers (OpenAI, Anthropic, Gemini, Bedrock, Ollama, etc.)
- **Collaboration** — real-time multiplayer editing, channels, and voice/video via LiveKit
- **Remote development** — SSH remoting and a remote server binary
- **Extensions** — WASM-based extension system for languages, themes, and debug adapters
- **Debugger** — built-in DAP (Debug Adapter Protocol) support
- **Terminal** — integrated terminal using alacritty_terminal

Sim unifies the former Goose agent capabilities into Sim's native agent crates and ships native editor, CLI, and local API surfaces from the main workspace.
