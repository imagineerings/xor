---
title: Configure a Local Model
description: Connect Sim to a local model provider and verify agent requests.
---

# Configure a Local Model

Local models are useful when you want low-latency or private inference for supported workflows.

## Before You Start

- Install and start your local model server.
- Confirm the server exposes a compatible endpoint.
- Download a model that fits your machine.

## Steps

1. Open Sim settings.
2. Navigate to the AI provider section.
3. Add or select the local provider.
4. Enter the provider URL and model name.
5. Save settings.
6. Open the Agent Panel.
7. Send a small verification prompt:

   ```text
   Reply with one sentence confirming the local provider is connected.
   ```

## Expected Result

The agent should respond without remote-provider authentication errors. If the request fails, verify the provider URL, model name, and local server logs.
