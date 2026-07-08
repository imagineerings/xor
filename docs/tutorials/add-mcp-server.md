---
title: Add an MCP Server
description: Configure a trusted Model Context Protocol server and verify it appears in Sim.
---

# Add an MCP Server

MCP servers expose extra tools to the agent. Configure them carefully because they can access local resources depending on the server.

## Before You Start

- Choose a trusted MCP server.
- Read its installation and security notes.
- Decide whether the server belongs in user settings or project settings.

## Steps

1. Install the server using its documented package manager or binary.
2. Open Sim settings.
3. Add the MCP server command, arguments, environment, and working directory.
4. Restart or reload the agent context if required.
5. Open the Agent Panel and ask:

   ```text
   List the MCP tools currently available and describe what each one can do.
   ```

6. Review the listed tools before approving any calls.

## Expected Result

The MCP server should appear as available tools in the agent context. If it does not, run the configured command manually and check missing environment variables or permissions.
