import { spawn } from 'node:child_process';
import { copyFile, mkdir } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const build = spawn(process.execPath, [
  path.join(root, 'node_modules/@tauri-apps/cli/tauri.js'),
  'build', '--debug', '--no-bundle', '--features', 'e2e',
  '--config', 'src-tauri/tauri.e2e.conf.json',
], { stdio: 'inherit' });
const code = await new Promise((resolve, reject) => {
  build.on('exit', resolve);
  build.on('error', reject);
});
if (code !== 0) process.exit(code ?? 1);
const destination = path.join(root, 'target/ui-test-build');
await mkdir(destination, { recursive: true });
await copyFile(path.join(root, 'target/debug/app.exe'), path.join(destination, 'app.exe'));
console.log(`UI test executable: ${path.join(destination, 'app.exe')}`);
