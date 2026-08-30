"use strict";

const vscode = require("vscode");
const { execFile } = require("child_process");
const path = require("path");

/** @type {Map<string, vscode.WebviewPanel>} */
const previewPanels = new Map();

/**
 * @param {vscode.ExtensionContext} context
 */
function activate(context) {
  context.subscriptions.push(
    vscode.commands.registerCommand("diagram.preview", () => previewActive()),
    vscode.commands.registerCommand("diagram.validate", () => validateActive()),
    vscode.commands.registerCommand("diagram.renderSvg", () => renderSvgActive()),
    vscode.commands.registerCommand("diagram.generateClass", () => generateFromSource("generate-class")),
    vscode.commands.registerCommand("diagram.generateTree", () => generateFromSource("generate-tree")),
    vscode.commands.registerCommand("diagram.generateCall", () => generateFromSource("generate-call")),
    vscode.workspace.onDidSaveTextDocument((doc) => {
      const cfg = vscode.workspace.getConfiguration("diagram");
      if (!cfg.get("autoPreviewOnSave", true)) {
        return;
      }
      const panel = previewPanels.get(doc.uri.fsPath);
      if (panel) {
        refreshPreview(panel, doc.uri.fsPath).catch((err) => {
          vscode.window.showErrorMessage(`diagram preview: ${err.message || err}`);
        });
      }
    })
  );
}

function deactivate() {}

function cliPath() {
  return vscode.workspace.getConfiguration("diagram").get("cliPath", "diagram");
}

function theme() {
  return vscode.workspace.getConfiguration("diagram").get("theme", "dark");
}

/**
 * @param {string} binary
 * @param {string[]} args
 * @returns {Promise<{ stdout: string, stderr: string }>}
 */
function runDiagram(binary, args) {
  return new Promise((resolve, reject) => {
    execFile(binary, args, { maxBuffer: 20 * 1024 * 1024 }, (err, stdout, stderr) => {
      if (err) {
        const msg = (stderr || err.message || String(err)).trim();
        reject(new Error(msg || `diagram exited with code ${err.code}`));
        return;
      }
      resolve({ stdout: String(stdout), stderr: String(stderr) });
    });
  });
}

function activeDiagramPath() {
  const editor = vscode.window.activeTextEditor;
  if (!editor) {
    throw new Error("Open a diagram file first");
  }
  const file = editor.document.uri.fsPath;
  if (!file) {
    throw new Error("Untitled buffers are not supported; save the file first");
  }
  return file;
}

async function previewActive() {
  let file;
  try {
    file = activeDiagramPath();
  } catch (e) {
    vscode.window.showErrorMessage(e.message);
    return;
  }

  const existing = previewPanels.get(file);
  if (existing) {
    existing.reveal(vscode.ViewColumn.Beside);
    await refreshPreview(existing, file);
    return;
  }

  const panel = vscode.window.createWebviewPanel(
    "diagramPreview",
    `Diagram: ${path.basename(file)}`,
    vscode.ViewColumn.Beside,
    { enableScripts: false, retainContextWhenHidden: true }
  );
  previewPanels.set(file, panel);
  panel.onDidDispose(() => {
    previewPanels.delete(file);
  });

  try {
    await refreshPreview(panel, file);
  } catch (e) {
    vscode.window.showErrorMessage(`diagram preview: ${e.message || e}`);
    panel.webview.html = errorHtml(e.message || String(e));
  }
}

/**
 * @param {vscode.WebviewPanel} panel
 * @param {string} file
 */
async function refreshPreview(panel, file) {
  const { stdout } = await runDiagram(cliPath(), [
    "render",
    file,
    "--theme",
    theme(),
  ]);
  const svg = stdout.trim();
  if (!svg.includes("<svg")) {
    throw new Error("render did not return SVG (is the diagram CLI installed?)");
  }
  panel.webview.html = previewHtml(svg, path.basename(file));
}

async function validateActive() {
  let file;
  try {
    file = activeDiagramPath();
  } catch (e) {
    vscode.window.showErrorMessage(e.message);
    return;
  }
  try {
    const { stdout } = await runDiagram(cliPath(), ["validate", file]);
    const body = stdout.trim() || "No output.";
    if (body.startsWith("Valid:")) {
      vscode.window.showInformationMessage(body);
    } else {
      vscode.window.showWarningMessage(body.split("\n")[0]);
    }
    const channel = vscode.window.createOutputChannel("diagram");
    channel.clear();
    channel.appendLine(body);
    channel.show(true);
  } catch (e) {
    vscode.window.showErrorMessage(`diagram validate: ${e.message || e}`);
  }
}

async function renderSvgActive() {
  let file;
  try {
    file = activeDiagramPath();
  } catch (e) {
    vscode.window.showErrorMessage(e.message);
    return;
  }
  const defaultName = path.basename(file).replace(/\.[^.]+$/, "") + ".svg";
  const uri = await vscode.window.showSaveDialog({
    defaultUri: vscode.Uri.file(path.join(path.dirname(file), defaultName)),
    filters: { SVG: ["svg"] },
  });
  if (!uri) {
    return;
  }
  try {
    await runDiagram(cliPath(), [
      "render",
      file,
      "--output",
      uri.fsPath,
      "--theme",
      theme(),
    ]);
    vscode.window.showInformationMessage(`Wrote ${uri.fsPath}`);
  } catch (e) {
    vscode.window.showErrorMessage(`diagram render: ${e.message || e}`);
  }
}

async function generateFromSource(subcommand) {
  let file;
  try {
    file = activeDiagramPath();
  } catch (e) {
    vscode.window.showErrorMessage(e.message);
    return;
  }
  const ext = path.extname(file).slice(1);
  if (!/^(rs|ts)$/i.test(ext)) {
    vscode.window.showErrorMessage(
      `Code generation needs a .rs or .ts file (got .${ext})`
    );
    return;
  }
  const stem = path.basename(file).replace(/\.[^.]+$/, "");
  const dir = path.dirname(file);
  const defaultName = `${stem}.${subcommand.split("-")[1]}.mmd`;
  const uri = await vscode.window.showSaveDialog({
    defaultUri: vscode.Uri.file(path.join(dir, defaultName)),
    filters: { Mermaid: ["mmd"], JSON: ["json"], DOT: ["dot"], D2: ["d2"] },
  });
  if (!uri) {
    return;
  }
  try {
    await runDiagram(cliPath(), [
      subcommand,
      file,
      "--output",
      uri.fsPath,
    ]);
    const channel = vscode.window.createOutputChannel("diagram");
    channel.clear();
    channel.appendLine(`Generated ${uri.fsPath}`);
    channel.show(true);
    vscode.window.showInformationMessage(`Generated ${uri.fsPath}`);
  } catch (e) {
    vscode.window.showErrorMessage(`diagram ${subcommand}: ${e.message || e}`);
  }
}

/**
 * @param {string} svg
 * @param {string} title
 */
function previewHtml(svg, title) {
  const safeTitle = escapeHtml(title);
  // SVG from our CLI is trusted local output; still strip script tags defensively.
  const safeSvg = svg.replace(/<script[\s\S]*?<\/script>/gi, "");
  return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta http-equiv="Content-Security-Policy" content="default-src 'none'; img-src data:; style-src 'unsafe-inline';" />
  <title>${safeTitle}</title>
  <style>
    html, body { margin: 0; padding: 0; background: transparent; }
    .wrap { padding: 12px; overflow: auto; }
    svg { max-width: 100%; height: auto; display: block; }
  </style>
</head>
<body>
  <div class="wrap">${safeSvg}</div>
</body>
</html>`;
}

/**
 * @param {string} message
 */
function errorHtml(message) {
  return `<!DOCTYPE html>
<html><body style="font-family:sans-serif;padding:16px;color:#c00;">
  <h3>Preview failed</h3>
  <pre>${escapeHtml(message)}</pre>
  <p>Ensure the <code>diagram</code> CLI is installed and <code>diagram.cliPath</code> is set correctly.</p>
</body></html>`;
}

/**
 * @param {string} s
 */
function escapeHtml(s) {
  return String(s)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

module.exports = { activate, deactivate };
