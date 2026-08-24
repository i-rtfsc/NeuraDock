import fs from 'node:fs';

function jsonVersion(path) {
  return JSON.parse(fs.readFileSync(path, 'utf8')).version;
}

function matchVersion(path, pattern, label) {
  const value = fs.readFileSync(path, 'utf8').match(pattern)?.[1];
  if (!value) throw new Error(`Could not read ${label} version from ${path}`);
  return value;
}

const expected = jsonVersion('package.json');
const versions = new Map([
  ['package.json', expected],
  ['apps/desktop/package.json', jsonVersion('apps/desktop/package.json')],
  ['tauri.conf.json', jsonVersion('apps/desktop/src-tauri/tauri.conf.json')],
  ['Cargo workspace', matchVersion('apps/desktop/src-tauri/Cargo.toml', /\[workspace\.package\][\s\S]*?^version = "([^"]+)"/m, 'Cargo workspace')],
  ['Cargo package', matchVersion('apps/desktop/src-tauri/Cargo.toml', /\[package\][\s\S]*?^version = "([^"]+)"/m, 'Cargo package')],
  ['README.md', matchVersion('README.md', /version-([0-9]+\.[0-9]+\.[0-9]+)/, 'README')],
  ['README_EN.md', matchVersion('README_EN.md', /version-([0-9]+\.[0-9]+\.[0-9]+)/, 'README_EN')],
]);

const mismatches = [...versions].filter(([, version]) => version !== expected);
if (mismatches.length > 0) {
  console.error(`Expected version ${expected}, found mismatches:`);
  for (const [source, version] of mismatches) console.error(`- ${source}: ${version}`);
  process.exit(1);
}

console.log(`Version ${expected} is consistent across project metadata.`);
