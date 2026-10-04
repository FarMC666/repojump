const vscode = require('vscode');
const { runStartup } = require('./startup.cjs');

exports.activate = async function (context) {
  const log = vscode.window.createOutputChannel('RepoJump Startup');
  context.subscriptions.push(log);
  try { await runStartup(vscode); }
  catch (error) { log.appendLine(String(error)); }
};
