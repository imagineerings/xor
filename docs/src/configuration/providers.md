---
title: Provider Configuration
description: Configure hosted, subscription, gateway, and local AI providers in Sim.
---

# Provider Configuration

Sim can connect to hosted APIs, existing subscriptions, gateways, and local model servers. Choose the path that matches your organization's security, billing, and latency requirements.

## Hosted API Access

Use direct API access when your team manages provider credentials. Store credentials in Sim's provider settings or the provider's recommended environment variables. Rotate keys regularly and avoid committing them to project files.

## Existing Subscriptions

Some providers support authenticated desktop or CLI sessions. Use this path when your organization already manages access through provider accounts rather than raw API keys.

## Gateways

Gateways centralize routing, logging, billing, and policy enforcement. They are useful for teams that need consistent model access across multiple tools.

## Local Models

Local providers reduce external network dependency and can be useful for private work. They still need enough local CPU, GPU, and memory for the selected model.

## Verification

After changing provider settings:

1. Restart or reconnect the provider if required.
2. Open the Agent Panel.
3. Send a small prompt.
4. Check model selection and error messages if the request fails.

Related guides:

- [Use API Access](../ai/use-api-access.md)
- [Use an Existing Subscription](../ai/use-an-existing-subscription.md)
- [Use a Gateway](../ai/use-a-gateway.md)
- [Use a Local Model](../ai/use-a-local-model.md)
