// Builds the Stabled Client mod for every supported Minecraft version (/client/versions/*) and copies the jars into
// the launcher's bundled resources as resources/clients/stabled-client-<minecraft>.jar.
//   npm run bundle-client                 → all versions
//   npm run bundle-client -- 26.3 1.21.11  → only these
import { execFileSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, readdirSync, rmSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const client = join(root, 'client');
const all = readdirSync(join(client, 'versions')).filter((v) => existsSync(join(client, 'versions', v, 'gradle.properties')));
const wanted = process.argv.slice(2).length ? process.argv.slice(2) : all;
const gradlew = process.platform === 'win32' ? 'gradlew.bat' : './gradlew';
execFileSync(gradlew, [`-Pstabled.versions=${wanted.join(',')}`, 'build'], { cwd: client, stdio: 'inherit', shell: process.platform === 'win32' });

const target = join(root, 'launcher', 'src-tauri', 'resources', 'clients');
mkdirSync(target, { recursive: true });
rmSync(join(target, 'README.txt'), { force: true });
for (const v of wanted) {
  const jar = join(client, 'versions', v, 'build', 'libs', `stabled-client-1.0.0+mc${v}.jar`);
  if (!existsSync(jar)) throw new Error(`Missing build output for ${v}: ${jar}`);
  copyFileSync(jar, join(target, `stabled-client-${v}.jar`));
  console.log(`Bundled Stabled client for Minecraft ${v}`);
}
