/**
 * Update checking
 *
 * Queries the GitHub Releases API for the latest published release and
 * compares it against the running app version. Releases are published by
 * the CI workflow (`.github/workflows/release.yml`) with `vX.Y.Z` tags.
 *
 * Because releases are created as drafts (they only become visible after a
 * manual Publish), the check falls back to the repository tags — which are
 * pushed together with every version — so the app can still detect a newer
 * version before the release is officially published.
 */

import { isNewerVersion } from './version';
import { createLogger } from './logger';

const logger = createLogger('UPDATER');

export const GITHUB_REPO = 'chengvar-glitch/nexashell';
export const RELEASES_API_URL = `https://api.github.com/repos/${GITHUB_REPO}/releases/latest`;
export const TAGS_API_URL = `https://api.github.com/repos/${GITHUB_REPO}/tags`;
export const RELEASE_PAGE_URL = `https://github.com/${GITHUB_REPO}/releases/latest`;

export type UpdateCheckStatus = 'upToDate' | 'available' | 'error';

export interface UpdateCheckResult {
  status: UpdateCheckStatus;
  /** Latest version found, without the `v` prefix (when the check succeeded). */
  latestVersion?: string;
  /** URL to open for downloading the new release. */
  releaseUrl?: string;
}

interface GitHubRelease {
  tag_name?: string;
  html_url?: string;
}

interface GitHubTag {
  name?: string;
}

/** Highest version tag among all repo tags, or null when none are valid. */
async function fetchLatestVersionTag(): Promise<string | null> {
  const response = await fetch(TAGS_API_URL, {
    headers: { Accept: 'application/vnd.github+json' },
  });
  if (!response.ok) {
    throw new Error(`GitHub tags API responded with ${response.status}`);
  }
  const tags = (await response.json()) as GitHubTag[];
  return (
    tags
      .map(tag => (tag.name ?? '').replace(/^v/i, ''))
      .filter(Boolean)
      .reduce<string>(
        (best, candidate) =>
          isNewerVersion(candidate, best) ? candidate : best,
        ''
      ) || null
  );
}

/**
 * Check whether a newer version is available on GitHub.
 *
 * @param currentVersion The running app version (e.g. "1.10.2").
 */
export async function checkForUpdates(
  currentVersion: string
): Promise<UpdateCheckResult> {
  try {
    let latestVersion = '';
    let releaseUrl = RELEASE_PAGE_URL;

    // Prefer the latest published release; fall back to tags when the
    // release is still a draft (the `/releases/latest` endpoint only sees
    // published releases).
    try {
      const releaseResponse = await fetch(RELEASES_API_URL, {
        headers: { Accept: 'application/vnd.github+json' },
      });
      if (releaseResponse.ok) {
        const release = (await releaseResponse.json()) as GitHubRelease;
        const tag = (release.tag_name ?? '').replace(/^v/i, '');
        if (tag) {
          latestVersion = tag;
          releaseUrl = release.html_url || RELEASE_PAGE_URL;
        }
      }
    } catch {
      // Fall through to the tags endpoint below.
    }

    if (!latestVersion) {
      latestVersion = (await fetchLatestVersionTag()) ?? '';
    }
    if (!latestVersion) {
      throw new Error('No release or version tag found for the repo');
    }

    const available = isNewerVersion(latestVersion, currentVersion);
    logger.info('Update check result', {
      current: currentVersion,
      latest: latestVersion,
      available,
    });

    return {
      status: available ? 'available' : 'upToDate',
      latestVersion,
      releaseUrl,
    };
  } catch (err) {
    logger.error('Failed to check for updates', err);
    return { status: 'error' };
  }
}
