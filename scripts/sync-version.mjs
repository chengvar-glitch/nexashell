import { readFileSync, writeFileSync } from 'fs';

const CARGO_PATH = 'src-tauri/Cargo.toml';
const PKG_PATH = 'package.json';

/**
 * Read the crate version from src-tauri/Cargo.toml.
 * Handles both a literal `[package] version` and a workspace-delegated
 * version (`version.workspace = true` with a version in [workspace.package]).
 * @returns {{ version: string, workspace: boolean }}
 * @throws {Error} with a clear message when the version cannot be determined.
 */
function readCargoVersion() {
  let cargo;
  try {
    cargo = readFileSync(CARGO_PATH, 'utf-8');
  } catch (err) {
    throw new Error(`Could not read "${CARGO_PATH}": ${err.message}`);
  }

  // Return the body of a top-level `[section]` (up to the next `[section]`),
  // or '' if the section is absent.
  const sectionBody = (name) => {
    const esc = name.replace(/\./g, '\\.');
    const headerRe = new RegExp(`^\\s*\\[${esc}\\]\\s*\\n?`, 'm');
    const header = headerRe.exec(cargo);
    if (!header) return '';
    const bodyStart = header.index + header[0].length;
    const next = cargo.slice(bodyStart).search(/^\s*\[[a-z]/m);
    const bodyEnd = next === -1 ? cargo.length : bodyStart + next;
    return cargo.slice(bodyStart, bodyEnd);
  };

  const pkgText = sectionBody('package');

  // Literal `version = "x.y.z"` in [package].
  const literal = pkgText.match(/^\s*version\s*=\s*"([^"]+)"\s*$/m);
  if (literal) {
    return { version: literal[1], workspace: false };
  }

  // `version.workspace = true`: the version lives in [workspace.package].
  if (/^\s*version\s*\.\s*workspace\s*=\s*true\s*$/m.test(pkgText)) {
    const wsText = sectionBody('workspace.package');
    const wsVersion = wsText.match(/^\s*version\s*=\s*"([^"]+)"\s*$/m);
    if (wsVersion) {
      return { version: wsVersion[1], workspace: true };
    }
    throw new Error(
      'src-tauri/Cargo.toml uses `version.workspace = true` under [package], ' +
        'but no [workspace.package] section with a `version = "x.y.z"` was found. ' +
        'Add a version to [workspace.package] so it can be synced to package.json.'
    );
  }

  throw new Error(
    'Could not determine the crate version from src-tauri/Cargo.toml. ' +
      'Expected `version = "x.y.z"` under [package] (or `version.workspace = true` ' +
      'with a version defined in [workspace.package]).'
  );
}

function readPackageJson() {
  try {
    return JSON.parse(readFileSync(PKG_PATH, 'utf-8'));
  } catch (err) {
    throw new Error(`Could not parse "${PKG_PATH}": ${err.message}`);
  }
}

function main() {
  const { version, workspace } = readCargoVersion();
  if (workspace) {
    console.log(
      `[sync-version] Resolved workspace version "${version}" from [workspace.package].`
    );
  }

  const pkg = readPackageJson();
  if (pkg.version === version) {
    console.log(
      `[sync-version] package.json is already at version "${version}". Nothing to do.`
    );
    return;
  }

  pkg.version = version;
  try {
    writeFileSync(PKG_PATH, JSON.stringify(pkg, null, 2) + '\n');
  } catch (err) {
    throw new Error(`Could not write "${PKG_PATH}": ${err.message}`);
  }
  console.log(`[sync-version] Updated package.json version to "${version}".`);
}

try {
  main();
} catch (err) {
  console.error(`[sync-version] ${err.message}`);
  process.exit(1);
}
