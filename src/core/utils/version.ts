/**
 * Version comparison utilities
 *
 * Compares dotted numeric version strings (e.g. "1.10.2", "v1.11.0").
 * Pre-release suffixes (e.g. "-beta.1") follow semver semantics: a version
 * with a pre-release suffix sorts before the same version without one.
 */

/**
 * Split a version into its numeric core and pre-release part.
 * "1.0.0-beta.1" -> ["1.0.0", "beta.1"]; "1.0.0" -> ["1.0.0", null].
 */
function splitCoreAndPre(version: string): [string, string | null] {
  const idx = version.indexOf('-');
  if (idx === -1) return [version, null];
  return [version.slice(0, idx), version.slice(idx + 1)];
}

/** Parse a dotted numeric string into integer parts (non-numeric -> 0). */
function toNumericParts(version: string): number[] {
  return version.split('.').map(part => {
    const n = Number.parseInt(part, 10);
    return Number.isNaN(n) ? 0 : n;
  });
}

function compareParts(a: number[], b: number[]): number {
  const len = Math.max(a.length, b.length);
  for (let i = 0; i < len; i++) {
    const na = a[i] ?? 0;
    const nb = b[i] ?? 0;
    if (na > nb) return 1;
    if (na < nb) return -1;
  }
  return 0;
}

/**
 * Compare two version strings.
 *
 * @returns 1 if `a` is newer than `b`, -1 if older, 0 if equal.
 */
export function compareVersions(a: string, b: string): number {
  const [coreA, preA] = splitCoreAndPre(a.replace(/^v/i, ''));
  const [coreB, preB] = splitCoreAndPre(b.replace(/^v/i, ''));

  const coreCmp = compareParts(toNumericParts(coreA), toNumericParts(coreB));
  if (coreCmp !== 0) return coreCmp;

  if (preA === null && preB === null) return 0;
  if (preA !== null && preB !== null) {
    return compareParts(toNumericParts(preA), toNumericParts(preB));
  }
  // Same core version: the final release is newer than a pre-release.
  return preA !== null ? -1 : 1;
}

/**
 * True when `latest` is strictly newer than `current`.
 */
export function isNewerVersion(latest: string, current: string): boolean {
  return compareVersions(latest, current) > 0;
}
