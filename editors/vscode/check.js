#!/usr/bin/env node
"use strict";

const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");

const root = __dirname;

const pkg = JSON.parse(fs.readFileSync(path.join(root, "package.json"), "utf8"));
const required = ["name", "displayName", "main", "engines", "contributes", "activationEvents"];
for (const key of required) {
  if (!(key in pkg)) {
    console.error(`package.json missing ${key}`);
    process.exit(1);
  }
}

const commands = (pkg.contributes.commands || []).map((c) => c.command);
for (const cmd of ["diagram.preview", "diagram.validate", "diagram.renderSvg"]) {
  if (!commands.includes(cmd)) {
    console.error(`missing command ${cmd}`);
    process.exit(1);
  }
}

const mainPath = path.join(root, pkg.main);
if (!fs.existsSync(mainPath)) {
  console.error(`main entry missing: ${pkg.main}`);
  process.exit(1);
}

execFileSync(process.execPath, ["--check", mainPath], { stdio: "inherit" });

const src = fs.readFileSync(mainPath, "utf8");
for (const name of ["activate", "deactivate", "diagram.preview", "diagram.validate"]) {
  if (!src.includes(name)) {
    console.error(`extension.js missing reference to ${name}`);
    process.exit(1);
  }
}

console.log("editors/vscode: package.json and extension.js OK");
