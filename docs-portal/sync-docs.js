import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const repoRootDir = path.resolve(__dirname, '..');
const wikiSrcDir = path.resolve(__dirname, '../docs/wiki');
const astroTargetDir = path.resolve(__dirname, './src/content/docs/wiki');
const astroRootIndexFile = path.resolve(__dirname, './src/content/docs/index.md');

const GITHUB_REPO_BASE = 'https://github.com/YntraAB/YntraPlatform/blob/main';

// 1. ADVANCED DYNAMIC CODEBASE AST SYMBOL INDEXER
function buildSymbolLineMap() {
  const symbolMap = new Map();
  const searchDirs = [
    path.join(repoRootDir, 'yntra-core/src'),
    path.join(repoRootDir, 'yntra-ui/src'),
  ];

  function scanDir(dir) {
    if (!fs.existsSync(dir)) return;
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        scanDir(fullPath);
      } else if (entry.isFile() && entry.name.endsWith('.rs')) {
        const relativePath = path.relative(repoRootDir, fullPath).replace(/\\/g, '/');
        const lines = fs.readFileSync(fullPath, 'utf8').split('\n');

        lines.forEach((line, idx) => {
          const lineNum = idx + 1;

          // Match Functions (pub, pub(crate), async, standard fn)
          const fnMatch = line.match(/(?:pub(?:\([^\)]+\)\s+)?(?:async\s+)?)?fn\s+([a-zA-Z0-9_]+)\s*[\(<]/);
          if (fnMatch) {
            const funcName = fnMatch[1];
            // Skip common Rust keywords / boilerplate
            if (!['new', 'default', 'fmt', 'clone', 'drop', 'from', 'into', 'main', 'test'].includes(funcName)) {
              let endLine = lineNum;
              let openBrackets = 0;
              let foundBracket = false;

              for (let j = idx; j < lines.length; j++) {
                const current = lines[j];
                for (const char of current) {
                  if (char === '{') {
                    openBrackets++;
                    foundBracket = true;
                  } else if (char === '}') {
                    openBrackets--;
                  }
                }
                if (foundBracket && openBrackets === 0) {
                  endLine = j + 1;
                  break;
                }
              }

              // Prefer public/outer declarations if symbol exists multiple times
              if (!symbolMap.has(funcName) || line.includes('pub ')) {
                symbolMap.set(funcName, {
                  file: relativePath,
                  startLine: lineNum,
                  endLine: Math.max(endLine, lineNum + 5),
                  type: 'function'
                });
              }
            }
          }

          // Match Structs / Enums / Traits / Statics
          const structMatch = line.match(/pub\s+(?:struct|enum|trait|type|static)\s+([a-zA-Z0-9_]+)/);
          if (structMatch) {
            const symbolName = structMatch[1];
            if (!symbolMap.has(symbolName)) {
              symbolMap.set(symbolName, {
                file: relativePath,
                startLine: lineNum,
                endLine: Math.min(lines.length, lineNum + 80),
                type: 'type'
              });
            }
          }
        });
      }
    }
  }

  searchDirs.forEach(scanDir);
  return symbolMap;
}

const symbolMap = buildSymbolLineMap();
console.log(`Dynamic AST Symbol Parser: Indexed ${symbolMap.size} codebase functions, types & statics.`);

// 2. MULTI-PASS AUTOMATIC LINK REPLACER FOR ALL CODEBASE SYMBOLS
function processMarkdownLinks(content) {
  // Pass 1: Convert raw file:/// URIs to GitHub URLs
  const fileUriRegex = /file:\/\/\/[cC]:\/Users\/hellich\/Desktop\/YntraPlatform\/([^\s\)]+)/g;
  let formatted = content.replace(fileUriRegex, (match, relativePath) => {
    return `${GITHUB_REPO_BASE}/${relativePath}`;
  });

  // Pass 2: Explicit {{func:function_name}} placeholders
  const funcPlaceholderRegex = /\{\{func:([a-zA-Z0-9_]+)\}\}/g;
  formatted = formatted.replace(funcPlaceholderRegex, (match, funcName) => {
    const symbolInfo = symbolMap.get(funcName);
    if (symbolInfo) {
      const url = `${GITHUB_REPO_BASE}/${symbolInfo.file}#L${symbolInfo.startLine}-L${symbolInfo.endLine}`;
      return `[\`${funcName}\`](${url})`;
    }
    return `\`${funcName}\``;
  });

  // Pass 3: Replace existing GitHub links for functions with dynamic line ranges
  symbolMap.forEach((info, symbol) => {
    const tableFnRegex = new RegExp(`\\[\`?${symbol}(?:\\(\\))?\`?\\]\\(https:\\/\\/github\\.com\\/[^\\s\\)]+\\)`, 'g');
    const dynamicUrl = `${GITHUB_REPO_BASE}/${info.file}#L${info.startLine}-L${info.endLine}`;
    formatted = formatted.replace(tableFnRegex, `[\`${symbol}\`](${dynamicUrl})`);
  });

  // Pass 4: Auto-link unlinked code symbols `func_name` or `func_name()` in text/tables (outside fenced code blocks)
  const segments = formatted.split(/(```[\s\S]*?```)/g);
  for (let i = 0; i < segments.length; i++) {
    // Only process text outside ``` code blocks ```
    if (!segments[i].startsWith('```')) {
      symbolMap.forEach((info, symbol) => {
        // Only replace symbols that are at least 4 chars long to avoid false positive short words
        if (symbol.length >= 4) {
          const dynamicUrl = `${GITHUB_REPO_BASE}/${info.file}#L${info.startLine}-L${info.endLine}`;
          // Match `symbol` or `symbol()` that is NOT inside an existing markdown link [...] (...)
          const backtickRegex = new RegExp(`(?<!\\[)\`(${symbol})(?:\\(\\))?\`(?!\\]|\\()`, 'g');
          segments[i] = segments[i].replace(backtickRegex, `[\`$1\`](${dynamicUrl})`);
        }
      });
    }
  }
  formatted = segments.join('');

  return formatted;
}

function copyAndFormatMarkdown(srcPath, destPath) {
  let content = fs.readFileSync(srcPath, 'utf8');

  // Strip existing YAML frontmatter if present
  if (content.trimStart().startsWith('---')) {
    const endIdx = content.indexOf('---', 3);
    if (endIdx !== -1) {
      content = content.substring(endIdx + 3).trim();
    }
  }

  content = processMarkdownLinks(content);

  // Extract first H1 heading for title
  const firstHeadingMatch = content.match(/^#\s+(.+)$/m);
  let title = firstHeadingMatch
    ? firstHeadingMatch[1].replace(/[:`"]/g, '').trim()
    : path.basename(srcPath, '.md');

  // Remove duplicate first H1 heading from body
  if (firstHeadingMatch) {
    content = content.replace(/^#\s+.+$/m, '').trim();
  }
  
  content = `---\ntitle: "${title}"\nhead:\n  - tag: script\n    attrs:\n      src: /target-blank.js\nsidebar:\n  hidden: false\n---\n\n` + content;

  fs.writeFileSync(destPath, content, 'utf8');
}

function copyDirRecursive(src, dest) {
  if (!fs.existsSync(src)) return;
  if (!fs.existsSync(dest)) {
    fs.mkdirSync(dest, { recursive: true });
  }

  const entries = fs.readdirSync(src, { withFileTypes: true });
  for (const entry of entries) {
    const srcPath = path.join(src, entry.name);
    const destPath = path.join(dest, entry.name);

    if (entry.isDirectory()) {
      copyDirRecursive(srcPath, destPath);
    } else if (entry.isFile() && entry.name.endsWith('.md')) {
      copyAndFormatMarkdown(srcPath, destPath);
    }
  }
}

console.log(`Syncing & formatting Wiki docs from ${wikiSrcDir} to ${astroTargetDir}...`);
copyDirRecursive(wikiSrcDir, astroTargetDir);

// Map Wiki README.md to Astro Root Index (/)
const wikiReadme = path.join(wikiSrcDir, 'README.md');
if (fs.existsSync(wikiReadme)) {
  let readmeContent = fs.readFileSync(wikiReadme, 'utf8');
  if (readmeContent.trimStart().startsWith('---')) {
    const endIdx = readmeContent.indexOf('---', 3);
    if (endIdx !== -1) {
      readmeContent = readmeContent.substring(endIdx + 3).trim();
    }
  }
  readmeContent = processMarkdownLinks(readmeContent);
  readmeContent = readmeContent.replace(/^#\s+.+$/m, '').trim();
  readmeContent = `---\ntitle: "Yntra Platform Technical Wiki"\nhead:\n  - tag: script\n    attrs:\n      src: /target-blank.js\ntemplate: splash\nhero:\n  tagline: Dynamic Modular Workspace Engine - Local-First Tripartite Architecture\n  actions:\n    - text: Explore Documentation\n      link: /wiki/01-getting-started/architecture-overview/\n      icon: right-arrow\n      variant: primary\n---\n\n` + readmeContent;
  fs.writeFileSync(astroRootIndexFile, readmeContent, 'utf8');
}

console.log('Wiki documentation & multi-pass symbol link indexing complete!');
