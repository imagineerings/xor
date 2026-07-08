const tools = {
  summarize_text({ text }) {
    const words = String(text || "")
      .trim()
      .split(/\s+/)
      .filter(Boolean);
    return {
      wordCount: words.length,
      summary: words.slice(0, 12).join(" "),
    };
  },
};

async function callTool(name, argumentsObject) {
  const tool = tools[name];
  if (!tool) throw new Error(`unknown frontend tool: ${name}`);
  return tool(argumentsObject);
}

const input = document.querySelector("#input");
const output = document.querySelector("#output");
document.querySelector("#run").addEventListener("click", async () => {
  try {
    const result = await callTool("summarize_text", { text: input.value });
    output.textContent = JSON.stringify(result, null, 2);
  } catch (error) {
    output.textContent = error.message;
  }
});
