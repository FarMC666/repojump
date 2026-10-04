const { test } = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { randomUUID } = require('node:crypto');
const { runStartup } = require('./startup.cjs');

function fixture(t, options = {}) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'repojump-helper-'));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const project = path.join(root, '代码 & (project); $');
  fs.mkdirSync(project);
  const requestId = randomUUID();
  const directory = path.join(root, requestId);
  fs.mkdirSync(directory);
  const requestPath = path.join(directory, 'request.json');
  const createdAt = Date.now();
  const request = { schemaVersion: 1, requestId, projectPath: project, action: 'gitGraph', createdAt, expiresAt: createdAt + 15_000, ...options.request };
  fs.writeFileSync(requestPath, JSON.stringify(request));
  const calls = [];
  const vscode = {
    workspace: {
      isTrusted: options.trusted !== false,
      workspaceFile: { scheme: 'file', fsPath: path.join(directory, 'Project.code-workspace') },
      workspaceFolders: [{ uri: { scheme: 'file', fsPath: options.project || project } }],
      getConfiguration: () => ({ inspect: () => ({ workspaceValue: requestPath }) }),
    },
    extensions: { getExtension: id => { assert.equal(id, 'mhutchie.git-graph'); return options.missing ? undefined : { activate: options.activate || (async () => {}) }; } },
    commands: {
      getCommands: async () => options.commands || ['git-graph.view'],
      executeCommand: async (...args) => { calls.push(args); if (options.executeError) throw new Error('command failed'); },
    },
    Uri: { file: fsPath => ({ scheme: 'file', fsPath }) },
  };
  return { directory, project, requestPath, request, vscode, calls, receipt: () => JSON.parse(fs.readFileSync(path.join(directory, 'result.json'), 'utf8')) };
}

test('opens Git Graph for the correct repository exactly once, including after reload', async t => {
  const f = fixture(t);
  await runStartup(f.vscode);
  assert.deepEqual(f.calls, [['git-graph.view', { rootUri: { scheme: 'file', fsPath: f.project } }]]);
  assert.deepEqual(f.receipt(), { requestId: f.request.requestId, code: null, detail: '' });
  assert.equal(fs.existsSync(f.requestPath), false);
  await runStartup(f.vscode);
  assert.equal(f.calls.length, 1);
});

test('concurrent activation atomically claims a request only once', async t => {
  const f = fixture(t);
  await Promise.all([runStartup(f.vscode), runStartup(f.vscode)]);
  assert.equal(f.calls.length, 1);
});

test('Windows workspace URI casing matches the native request path', { skip: process.platform !== 'win32' }, async t => {
  const f = fixture(t);
  f.vscode.workspace.workspaceFile.fsPath = f.vscode.workspace.workspaceFile.fsPath.toLowerCase();
  f.vscode.workspace.workspaceFolders[0].uri.fsPath = f.project.toLowerCase();
  await runStartup(f.vscode);
  assert.equal(f.calls.length, 1);
  assert.equal(f.receipt().code, null);
});

test('another project window does not consume the request', async t => {
  const f = fixture(t, { project: os.tmpdir() });
  await runStartup(f.vscode);
  assert.equal(f.calls.length, 0);
  assert.equal(fs.existsSync(f.requestPath), true);
});

test('expired, future, malformed and arbitrary command requests never execute', async t => {
  for (const request of [
    { createdAt: 0, expiresAt: 15_000 },
    { createdAt: Date.now() + 30_000, expiresAt: Date.now() + 45_000 },
    { requestId: 'other' }, { action: 'arbitrary.command' }, { schemaVersion: 2 },
  ]) {
    const f = fixture(t, { request });
    await runStartup(f.vscode);
    assert.equal(f.calls.length, 0);
    assert.equal(fs.existsSync(f.requestPath), true);
  }
});

test('ordinary folders and user-level configuration do not execute startup commands', async t => {
  const f = fixture(t);
  f.vscode.workspace.workspaceFile = undefined;
  await runStartup(f.vscode);
  f.vscode.workspace.workspaceFile = { scheme: 'file', fsPath: path.join(f.directory, 'Project.code-workspace') };
  f.vscode.workspace.getConfiguration = () => ({ inspect: () => ({ globalValue: f.requestPath }) });
  await runStartup(f.vscode);
  assert.equal(f.calls.length, 0);
});

test('missing or disabled Git Graph reports an error while keeping the project open', async t => {
  for (const options of [{ missing: true }, { activate: async () => { throw new Error('disabled'); } }, { commands: [] }]) {
    const f = fixture(t, options);
    await runStartup(f.vscode);
    assert.equal(f.receipt().code, 'gitGraphUnavailable');
    assert.equal(f.calls.length, 0);
  }
});

test('command failures and untrusted workspaces report distinct results', async t => {
  const failed = fixture(t, { executeError: true });
  await runStartup(failed.vscode);
  assert.equal(failed.receipt().code, 'startupContentFailed');
  const untrusted = fixture(t, { trusted: false });
  await runStartup(untrusted.vscode);
  assert.equal(untrusted.receipt().code, 'startupWorkspaceUntrusted');
  assert.equal(untrusted.calls.length, 0);
});

test('activation that outlives the deadline cannot execute the command later', async t => {
  let finish;
  const f = fixture(t, { activate: () => new Promise(resolve => { finish = resolve; }) });
  f.request.createdAt = Date.now() - 14_950;
  f.request.expiresAt = f.request.createdAt + 15_000;
  fs.writeFileSync(f.requestPath, JSON.stringify(f.request));
  await runStartup(f.vscode);
  assert.equal(f.receipt().code, 'startupTimeout');
  finish();
  await new Promise(resolve => setImmediate(resolve));
  assert.equal(f.calls.length, 0);
});
