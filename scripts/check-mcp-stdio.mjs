#!/usr/bin/env node
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import path from "node:path";

const cliPath = process.argv[2] ?? defaultCliPath();
const requiredTools = [
  "codex_speak_prepare",
  "codex_speak_speak_text",
  "codex_speak_set_pronunciation"
];
const framings = ["header", "line"];

if (!existsSync(cliPath)) {
  fail(`CLI not found: ${cliPath}`);
}

const results = [];
for (const framing of framings) {
  results.push(await checkFraming(framing));
}

console.log(
  JSON.stringify(
    {
      ok: true,
      cli: cliPath,
      framings: results,
      requiredTools
    },
    null,
    2
  )
);

async function checkFraming(framing) {
  return new Promise((resolve) => {
    const child = spawn(cliPath, ["mcp"], {
      stdio: ["pipe", "pipe", "pipe"]
    });

    let stdout = Buffer.alloc(0);
    let stderr = "";
    const responses = [];
    let failed = false;

    const timeout = setTimeout(() => {
      failed = true;
      child.kill();
      fail(`Timed out waiting for MCP ${framing} responses.`);
    }, 5000);

    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString("utf8");
    });

    child.stdout.on("data", (chunk) => {
      stdout = Buffer.concat([stdout, chunk]);
      stdout = parseResponses(stdout, framing, responses);
      if (responses.length >= 2) {
        child.stdin.end();
      }
    });

    child.on("error", (error) => {
      failed = true;
      clearTimeout(timeout);
      fail(error.message);
    });

    child.on("exit", (code) => {
      clearTimeout(timeout);
      if (failed) {
        return;
      }
      const tools = responses.find((item) => item.id === 2)?.result?.tools ?? [];
      const names = tools.map((tool) => tool.name).sort();
      const missing = requiredTools.filter((name) => !names.includes(name));
      if (code !== 0 || missing.length > 0) {
        fail(
          JSON.stringify(
            {
              framing,
              code,
              missing,
              tools: names,
              stderr: stderr.trim()
            },
            null,
            2
          )
        );
      }
      resolve({
        framing,
        toolCount: names.length
      });
    });

    send(child, framing, {
      jsonrpc: "2.0",
      id: 1,
      method: "initialize",
      params: {
        protocolVersion: framing === "line" ? "2025-06-18" : "2024-11-05",
        capabilities: {},
        clientInfo: {
          name: "codex-speak-mcp-stdio-check",
          version: "1"
        }
      }
    });
    send(child, framing, {
      jsonrpc: "2.0",
      id: 2,
      method: "tools/list",
      params: {}
    });
  });
}

function defaultCliPath() {
  const exe = process.platform === "win32" ? "codex-speak.exe" : "codex-speak";
  return path.join("target", "release", exe);
}

function send(child, framing, message) {
  const body = Buffer.from(JSON.stringify(message));
  if (framing === "line") {
    child.stdin.write(body);
    child.stdin.write("\n");
    return;
  }
  child.stdin.write(`Content-Length: ${body.length}\r\n\r\n`);
  child.stdin.write(body);
}

function parseResponses(stdout, framing, responses) {
  if (framing === "line") {
    return parseLineResponses(stdout, responses);
  }
  return parseHeaderResponses(stdout, responses);
}

function parseLineResponses(stdout, responses) {
  while (true) {
    const lineEnd = stdout.indexOf("\n");
    if (lineEnd < 0) {
      return stdout;
    }
    const line = stdout.slice(0, lineEnd).toString("utf8").trim();
    stdout = stdout.slice(lineEnd + 1);
    if (line.length === 0) {
      continue;
    }
    responses.push(JSON.parse(line));
  }
}

function parseHeaderResponses(stdout, responses) {
  while (true) {
    const headerEnd = stdout.indexOf("\r\n\r\n");
    if (headerEnd < 0) {
      return stdout;
    }

    const header = stdout.slice(0, headerEnd).toString("utf8");
    const match = header.match(/Content-Length:\s*(\d+)/i);
    if (!match) {
      fail(`Missing Content-Length header: ${header}`);
    }

    const length = Number(match[1]);
    const bodyStart = headerEnd + 4;
    if (stdout.length < bodyStart + length) {
      return stdout;
    }

    const body = stdout.slice(bodyStart, bodyStart + length).toString("utf8");
    responses.push(JSON.parse(body));
    stdout = stdout.slice(bodyStart + length);
  }
}

function fail(message) {
  console.error(`MCP stdio check failed: ${message}`);
  process.exit(1);
}
