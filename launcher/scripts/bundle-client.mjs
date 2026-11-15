// Builds the Stabled Client mod (/client) and copies the jar into the launcher's bundled resources.
import { execFileSync } from 'node:child_process';
import { copyFileSync, readdirSync, mkdirSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const client = join(root, 'client');
const gradlew = process.platform === 'win32' ? 'gradlew.bat' : './gradlew';
execFileSync(gradlew, ['build', '--no-configuration-cache'], { cwd: client, stdio: 'inherit', shell: process.platform === 'win32' });
const libs = join(client, 'build', 'libs');
const jar = readdirSync(libs).find((f) => f.startsWith('stabled-client-') && f.endsWith('.jar') && !f.includes('sources'));
if (!jar) throw new Error('Stabled client jar not found in ' + libs);
const target = join(root, 'launcher', 'src-tauri', 'resources', 'stabled-client.jar');
mkdirSync(dirname(target), { recursive: true });
copyFileSync(join(libs, jar), target);
console.log(`Bundled ${jar} -> ${target}`);
