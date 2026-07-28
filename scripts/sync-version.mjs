import { readFileSync, writeFileSync } from 'fs';

const cargo = readFileSync('src-tauri/Cargo.toml', 'utf-8');
const match = cargo.match(/^version = "(.+?)"/m);
if (!match) {
  process.exit(1);
}
const version = match[1];
const pkgPath = 'package.json';
const pkg = JSON.parse(readFileSync(pkgPath, 'utf-8'));
if (pkg.version !== version) {
  pkg.version = version;
  writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');
}