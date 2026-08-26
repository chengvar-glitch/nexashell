/**
 * Remote path detection Composable
 *
 * Centralizes remote working-directory resolution for SFTP upload targeting.
 * OSC 7 / OSC 9;9 sequences (reported by iTerm2 and other mainstream
 * terminals) are treated as the highest-priority trusted source. The buffer
 * scanning heuristics (prompt / pwd / cd) act only as a fallback when OSC has
 * not yet provided a trusted absolute path.
 */

import { ref } from 'vue';
import type { Terminal } from '@xterm/xterm';
import { createLogger } from '@/core/utils/logger';

const logger = createLogger('REMOTE_PATH');

/**
 * Normalize an absolute path by collapsing `.` and `..` segments.
 * Returns the path unchanged when it does not start with `/`.
 */
export function normalizeRemotePath(path: string): string {
  if (!path.startsWith('/')) return path;
  const parts = path.split('/');
  const stack: string[] = [];
  for (const part of parts) {
    if (!part || part === '.') continue;
    if (part === '..') {
      stack.pop();
    } else {
      stack.push(part);
    }
  }
  return '/' + stack.join('/');
}

export function resolveRelativeCd(
  relativePath: string,
  basePath: string,
  homePath: string
): string {
  if (!relativePath) return '';
  if (relativePath === '-') return '';
  if (relativePath === '.') return basePath || '';
  if (relativePath === '..') {
    return basePath ? normalizeRemotePath(`${basePath}/..`) : '';
  }
  if (relativePath === '~') {
    return homePath ? normalizeRemotePath(homePath) : '';
  }
  if (relativePath.startsWith('~/')) {
    return homePath
      ? normalizeRemotePath(`${homePath}/${relativePath.slice(2)}`)
      : '';
  }
  if (!basePath) return '';
  return normalizeRemotePath(`${basePath}/${relativePath}`);
}

export function useRemotePath() {
  const currentRemotePath = ref('.');
  const remoteHomeDir = ref('');
  const lastPathDetectionSource = ref<string>('none'); // Track how we detected the path
  const lastKnownAbsolutePath = ref('');
  const hasOscPath = ref(false);

  /**
   * Resolve the current remote working directory.
   *
   * OSC 7 / OSC 9;9 sequences have the highest priority. When a trusted
   * absolute path from OSC is already available (lastKnownAbsolutePath with
   * hasOscPath), return it directly without letting buffer heuristics
   * overwrite it with a possible wrong guess.
   */
  const detectRemotePath = async (
    getTerminal: () => Terminal | null
  ): Promise<void> => {
    // Highest priority: OSC-provided trusted absolute path
    if (hasOscPath.value && lastKnownAbsolutePath.value) {
      currentRemotePath.value = lastKnownAbsolutePath.value;
      lastPathDetectionSource.value = 'osc';
      logger.info('Path detection complete', {
        finalPath: currentRemotePath.value,
        source: 'osc',
      });
      return;
    }

    // Reset path for fresh detection on each drag
    let detectedPath = '';
    let detectionSource = 'none';
    let hasRecentRelativeCd = false; // Track if user did cd .., cd -, etc
    let promptGuess = '';
    let cdGuess = '';
    let relativeCdGuess = '';
    let pwdOutputGuess = '';

    const terminal = getTerminal();

    // Try to guess path from terminal buffer (most accurate for interactive shell)
    if (terminal) {
      const buffer = terminal.buffer.active;
      if (!buffer) {
        logger.warn('Terminal buffer not available');
      } else {
        logger.debug('Terminal buffer available', {
          bufferLength: buffer.length,
          cursorX: buffer.cursorX,
          cursorY: buffer.cursorY,
          baseY: buffer.baseY,
        });

        // 1. Scan back from current cursor position for prompts, cd commands, and pwd output
        const maxScanLines = 100;
        const startLine = buffer.baseY + buffer.cursorY;
        const endLine = Math.max(0, startLine - maxScanLines);

        // Build a map of lines to avoid scanning too many empty lines
        const lines: Array<{ index: number; text: string }> = [];
        for (let i = startLine; i >= endLine; i--) {
          const line = buffer.getLine(i)?.translateToString(true).trim();
          if (line) {
            lines.push({ index: i, text: line });
          }
        }

        logger.debug('Buffer scan started', {
          cursorY: buffer.cursorY,
          baseY: buffer.baseY,
          startLine,
          endLine,
          totalLines: buffer.length,
          nonEmptyLines: lines.length,
        });

        // Process lines in reverse chronological order (most recent first)
        for (const { index, text } of lines) {
          const line = text;
          // eslint-disable-next-line no-control-regex
          const cleanLine = line.replace(/\x1b\[[0-9;]*m/g, '').trim();

          // Check for relative cd commands (these invalidate old absolute paths)
          // Also track cd history for intelligent directory inference
          if (line.includes('cd ')) {
            const relativeCdMatch = line.match(/cd\s+(\.\.|[-]|\.)/);
            const absoluteCdMatch = line.match(/cd\s+(['"]?)([^\s'"]+)\1/);

            if (relativeCdMatch) {
              if (!hasRecentRelativeCd) {
                hasRecentRelativeCd = true;
                logger.debug('Detected relative cd command', {
                  command: relativeCdMatch[0],
                  lineIndex: index,
                });
              }
              if (!relativeCdGuess) {
                relativeCdGuess = relativeCdMatch[1];
              }
            } else if (absoluteCdMatch && absoluteCdMatch[2]) {
              let path = absoluteCdMatch[2];
              if (path.endsWith('/') && path !== '/') {
                path = path.slice(0, -1);
              }
              if (path.startsWith('/')) {
                logger.debug('Recorded cd history', { path, index });
              } else {
                if (!hasRecentRelativeCd) {
                  hasRecentRelativeCd = true;
                  logger.debug('Detected relative cd command', {
                    command: `cd ${path}`,
                    lineIndex: index,
                  });
                }
                if (!relativeCdGuess) {
                  relativeCdGuess = path;
                }
              }
            }
          }

          // Pattern A: Match pwd output - absolute path on a line by itself
          if (!pwdOutputGuess && line.startsWith('/')) {
            // pwd output should:
            // 1. Start with /
            // 2. Contain at least one more /
            // 3. NOT contain spaces (commands can have output with spaces)
            // 4. NOT contain [ or (
            // 5. NOT contain @
            if (
              cleanLine.startsWith('/') &&
              cleanLine.includes('/') &&
              !cleanLine.includes(' ') &&
              !cleanLine.includes('[') &&
              !cleanLine.includes('(') &&
              !cleanLine.includes('@')
            ) {
              pwdOutputGuess = cleanLine;
              logger.debug('Found pwd output', {
                line: cleanLine,
                lineIndex: index,
              });
            }
          }

          // Pattern B: Match Prompt to get terminal "hint"
          // Most recent prompt is most important
          if (!promptGuess) {
            // CentOS style: [root@host /current/path]#
            const centosMatch = line.match(/\[.*@.*\s+(.*)\][#$]/);
            // Ubuntu style: user@host:/path$
            const ubuntuMatch = line.match(/.*@.*:(.*)[#$]/);
            // Zsh style: user@host path %
            const zshMatch = line.match(/.*@.*\s+([^ ]+)\s+%/);

            const hint = centosMatch?.[1] || ubuntuMatch?.[1] || zshMatch?.[1];
            if (
              line.includes('@') &&
              (line.includes('#') || line.includes('$'))
            ) {
              logger.debug('Prompt candidates found', {
                line: cleanLine,
                centosMatch: centosMatch?.[1],
                ubuntuMatch: ubuntuMatch?.[1],
                zshMatch: zshMatch?.[1],
                hint,
              });
            }
            // Accept paths: /, /path, or relative names
            // Reject: ~ or . (as they are generic)
            if (hint && hint !== '~' && hint !== '.') {
              promptGuess = hint.trim();
              logger.debug('Found prompt hint', {
                hint: promptGuess,
                lineIndex: index,
              });
            }
          }

          // Pattern C: Match recent cd commands (including paths with trailing slashes)
          if (!cdGuess && line.includes('cd ')) {
            // Match: cd /path, cd "/path", cd '/path', etc (with optional trailing /)
            const cdMatch = line.match(/cd\s+(['"]?)([^\s'"]+)\1/);
            logger.debug('cd command scan', {
              line: cleanLine,
              hascdMatch: !!cdMatch,
              cdMatch: cdMatch ? [cdMatch[1], cdMatch[2]] : null,
            });
            if (cdMatch && cdMatch[2]) {
              let path = cdMatch[2].trim();
              // Remove trailing slash for consistency
              if (path.endsWith('/') && path !== '/') {
                path = path.slice(0, -1);
              }
              if (path.startsWith('/')) {
                cdGuess = path;
                logger.debug('Found cd command', {
                  path: cdGuess,
                  lineIndex: index,
                });
              }
            }
          }

          // Early exit if we found everything
          if (pwdOutputGuess && promptGuess && cdGuess) break;
        }

        // 2. Intelligent path resolution with strict priority
        // KEY INSIGHT: The most recent prompt is ALWAYS more reliable than historical pwd output
        // because it reflects the current state of the shell
        logger.debug('Path candidates', { pwdOutputGuess, promptGuess, cdGuess });

        // Intelligent priority logic:
        // 1. Absolute paths are always preferred over relative
        // 2. Most recent data (prompt) is preferred over historical (pwd output)
        // 3. Never use relative paths from Prompt alone - they're unreliable after cd .. or cd -

        if (promptGuess && promptGuess.startsWith('/')) {
          // Highest priority: Prompt shows absolute path directly
          detectedPath = promptGuess;
          detectionSource = 'prompt-absolute';
        } else if (pwdOutputGuess && pwdOutputGuess.startsWith('/')) {
          // Second: pwd output (most reliable source of truth)
          detectedPath = pwdOutputGuess;
          detectionSource = 'pwd-output';
        } else if (cdGuess && cdGuess.startsWith('/')) {
          // Third: cd command with absolute path
          // But verify it matches the current prompt
          const cdLastComponent = cdGuess.split('/').pop();
          if (promptGuess && cdLastComponent === promptGuess) {
            // MATCH: Prompt confirms this is the right path (e.g., cd /a/b + prompt "b")
            detectedPath = cdGuess;
            detectionSource = 'cd-command-verified';
            logger.debug('cd-command-verified: cd path matches prompt', {
              cdPath: cdGuess,
              cdLastComponent,
              currentPrompt: promptGuess,
            });
          } else if (promptGuess && !promptGuess.startsWith('/')) {
            // CHECK: Prompt is relative but might still match cd
            // Example: cd /usr/local/sankuai executed, now prompt shows "sankuai"
            // This is VALID - prompt shows dir name that matches cd's last component
            if (cdLastComponent === promptGuess) {
              // They DO match! Use the cd path
              detectedPath = cdGuess;
              detectionSource = 'cd-command-verified-from-relative-prompt';
              logger.debug('cd path matches relative prompt', {
                cdPath: cdGuess,
                cdLastComponent,
                currentPrompt: promptGuess,
              });
            } else {
              // MISMATCH: cd last component != prompt
              // This means user likely did cd .. or some relative operation
              // Example: cd /usr/local/sankuai was executed, but now prompt shows "local"
              // This means cd .. happened - we MUST handle this case specially
              hasRecentRelativeCd = true;
              logger.debug('Mismatch detected: cd != prompt', {
                cdPath: cdGuess,
                cdLastComponent,
                currentPrompt: promptGuess,
                reason: 'Likely relative cd operation (cd .., cd -, etc)',
              });
              // Don't try to reconstruct path here - let the later inference logic handle it
              // It has better context to determine if this is cd .., cd -, etc.
            }
          } else {
            // Use cd path as-is if there's no conflicting prompt
            detectedPath = cdGuess;
            detectionSource = 'cd-command';
          }
        }
        // NOTE: We do NOT use promptGuess as a fallback if it's not absolute
        // Relative prompts (e.g., "local" or "sankuai") after cd .. are unreliable
        // Better to use probed path or home directory as fallback
      }
    }

    logger.info('Buffer scan complete', {
      detectedPath,
      detectionSource,
      hasRecentRelativeCd,
      allCandidates: {
        promptGuess,
        pwdOutputGuess,
        cdGuess,
      },
    });

    // CRITICAL IMPROVEMENT: If we have a relative cd but detectedPath is empty,
    // try to use cd command history to infer the current directory
    // This handles: cd /usr/local/sankuai -> cd .. -> prompt shows "local"
    if (hasRecentRelativeCd && !detectedPath && cdGuess && promptGuess) {
      logger.info('Using cd history to infer path after relative cd', {
        lastAbsoluteCd: cdGuess,
        currentPrompt: promptGuess,
      });

      // If the last absolute cd's parent directory name matches prompt,
      // then we're in that parent directory
      const parentPath = cdGuess.substring(0, cdGuess.lastIndexOf('/'));
      const parentDirName = parentPath.split('/').pop() || parentPath;

      if (parentDirName === promptGuess) {
        // Match! We're in the parent directory of the last cd
        detectedPath = parentPath || '/';
        detectionSource = 'inferred-parent-from-cd-and-prompt';
        hasRecentRelativeCd = false; // Mark as resolved
        logger.debug('Inferred parent directory', {
          lastCd: cdGuess,
          parentPath,
          parentDirName,
          currentPrompt: promptGuess,
          inferred: detectedPath,
        });
      } else if (
        cdGuess === `/${promptGuess}` ||
        cdGuess.endsWith(`/${promptGuess}`)
      ) {
        // The prompt matches the last component of the cd command (cd /a/b -> cd .. -> prompt "a")
        // This shouldn't happen if we have cd .., but handle it anyway
        const parts = cdGuess.split('/').filter(p => p);
        if (parts.length > 1) {
          const reconstructed = '/' + parts.slice(0, -1).join('/');
          detectedPath = reconstructed;
          detectionSource = 'inferred-sibling-from-cd-and-prompt';
          hasRecentRelativeCd = false;
          logger.debug('Inferred sibling directory', {
            inferred: detectedPath,
          });
        }
      }
    }

    if (!detectedPath && relativeCdGuess) {
      const resolved = resolveRelativeCd(
        relativeCdGuess,
        lastKnownAbsolutePath.value,
        remoteHomeDir.value
      );
      if (resolved) {
        detectedPath = resolved;
        detectionSource = 'relative-cd-resolved';
        hasRecentRelativeCd = false;
        logger.debug('Resolved relative cd with last known path', {
          relativeCd: relativeCdGuess,
          basePath: lastKnownAbsolutePath.value,
          resolved,
        });
      }
    }

    // Final fallback
    if (!detectedPath) {
      detectedPath = lastKnownAbsolutePath.value || remoteHomeDir.value || '.';
      detectionSource = lastKnownAbsolutePath.value
        ? 'fallback-last-known'
        : 'fallback-home';
    }

    // Update the UI with the detected path
    currentRemotePath.value = detectedPath;
    lastPathDetectionSource.value = detectionSource;
    if (currentRemotePath.value.startsWith('/')) {
      lastKnownAbsolutePath.value = normalizeRemotePath(currentRemotePath.value);
    }
    logger.info('Path detection complete', {
      finalPath: currentRemotePath.value,
      source: detectionSource,
    });
  };

  return {
    currentRemotePath,
    remoteHomeDir,
    lastKnownAbsolutePath,
    lastPathDetectionSource,
    hasOscPath,
    detectRemotePath,
  };
}
