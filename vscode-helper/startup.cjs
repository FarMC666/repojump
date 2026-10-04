const fs = require('node:fs');
const path = require('node:path');

const REQUEST_LIFETIME = 15_000;
const uuid = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/;
function identity(file) {
  const resolved = fs.realpathSync(file).replace(/\\/g, '/').replace(/\/+$/, '');
  return process.platform === 'win32' ? resolved.toLowerCase() : resolved;
}
function failure(code, detail = '') { return Object.assign(new Error(detail), { code }); }
async function withinDeadline(operation, expiresAt) {
  const remaining = expiresAt - Date.now();
  if (remaining <= 0) throw failure('startupTimeout');
  let timer;
  try {
    return await Promise.race([
      operation(),
      new Promise((_, reject) => { timer = setTimeout(() => reject(failure('startupTimeout')), remaining); }),
    ]);
  } finally { clearTimeout(timer); }
}

// Requests are carried by a unique managed workspace, never by global or project settings.
async function runStartup(vscode) {
  const workspace = vscode.workspace;
  const requestPath = workspace.getConfiguration('repojump').inspect('launchRequest')?.workspaceValue;
  if (typeof requestPath !== 'string' || !path.isAbsolute(requestPath) || workspace.workspaceFile?.scheme !== 'file') return;
  const folder = path.dirname(workspace.workspaceFile.fsPath);
  if (path.basename(requestPath) !== 'request.json' || !uuid.test(path.basename(folder))) return;
  let request;
  try {
    if (fs.statSync(requestPath).size > 16_384) return;
    if (identity(path.dirname(requestPath)) !== identity(folder)) return;
    request = JSON.parse(fs.readFileSync(requestPath, 'utf8'));
    const now = Date.now();
    if (request.schemaVersion !== 1 || request.requestId !== path.basename(folder) || request.action !== 'gitGraph'
      || !Number.isSafeInteger(request.createdAt) || !Number.isSafeInteger(request.expiresAt)
      || request.createdAt > now || request.expiresAt <= now
      || request.expiresAt - request.createdAt !== REQUEST_LIFETIME
      || typeof request.projectPath !== 'string' || !path.isAbsolute(request.projectPath)
      || workspace.workspaceFolders?.length !== 1 || workspace.workspaceFolders[0].uri.scheme !== 'file'
      || identity(workspace.workspaceFolders[0].uri.fsPath) !== identity(request.projectPath)) return;
    // An atomic rename ensures this launch can be consumed only once, including after reload.
    fs.renameSync(requestPath, path.join(folder, 'request.claimed.json'));
  } catch { return; }

  let code = null;
  let detail = '';
  try {
    if (!workspace.isTrusted) throw failure('startupWorkspaceUntrusted');
    const graph = vscode.extensions.getExtension('mhutchie.git-graph');
    if (!graph) throw failure('gitGraphUnavailable');
    try { await withinDeadline(() => graph.activate(), request.expiresAt); }
    catch (error) { throw failure(error.code === 'startupTimeout' ? 'startupTimeout' : 'gitGraphUnavailable', String(error)); }
    const commands = await withinDeadline(() => vscode.commands.getCommands(true), request.expiresAt);
    if (!commands.includes('git-graph.view')) throw failure('gitGraphUnavailable');
    await withinDeadline(() => vscode.commands.executeCommand('git-graph.view', { rootUri: vscode.Uri.file(request.projectPath) }), request.expiresAt);
  } catch (error) {
    code = ['startupWorkspaceUntrusted', 'gitGraphUnavailable', 'startupTimeout'].includes(error.code) ? error.code : 'startupContentFailed';
    detail = String(error);
  }
  const result = { requestId: request.requestId, code, detail: detail.slice(0, 4096) };
  const temporary = path.join(folder, 'result.tmp');
  fs.writeFileSync(temporary, JSON.stringify(result), { flag: 'wx' });
  fs.renameSync(temporary, path.join(folder, 'result.json'));
}

module.exports = { runStartup };
