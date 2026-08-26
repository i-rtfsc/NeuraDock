import { readFileSync } from 'node:fs';
import { globSync } from 'node:fs';

const makefile = readFileSync('Makefile', 'utf8');
const targets = new Set(
  [...makefile.matchAll(/^([a-zA-Z0-9_-]+):/gm)].map((match) => match[1]),
);
const files = ['README.md', ...globSync('docs/**/*.md')];
const missing = [];

for (const file of files) {
  const content = readFileSync(file, 'utf8');
  for (const match of content.matchAll(/(?:^|[`\n]\s*)make\s+([a-zA-Z0-9_-]+)/g)) {
    const target = match[1];
    if (!targets.has(target)) {
      const line = content.slice(0, match.index).split('\n').length;
      missing.push(`${file}:${line}: make ${target}`);
    }
  }
}

if (missing.length > 0) {
  console.error('Documented make targets missing from Makefile:');
  for (const entry of missing) console.error(`- ${entry}`);
  process.exit(1);
}

console.log('Documented make targets are valid.');
