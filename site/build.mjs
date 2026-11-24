// Builds the FastEmu site into site/public/.
//
//   npm ci
//   node build.mjs
//
// It renders docs/*.md with a shared template, copies the landing page and
// the logo, writes the 404 page, sitemap.xml and robots.txt, then checks that
// every internal link and anchor in the output resolves. A broken link fails
// the build, so a pull request cannot publish one.

import { cpSync, existsSync, mkdirSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Marked } from 'marked';

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, '..');
const out = join(here, 'public');
const SITE = 'https://fastemu.functionx.dev';
const REPO = 'https://github.com/fzn0x/FastEmu';

/** The guides in reading order. The first heading of each file is its title. */
const GUIDES = ['getting-started', 'playing', 'settings', 'compatibility', 'building', 'building-android', 'faq'];

const escapeHtml = (s) => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
const stripTags = (s) => s.replace(/<[^>]+>/g, '').replace(/&amp;/g, '&').replace(/&lt;/g, '<').replace(/&gt;/g, '>').replace(/&quot;/g, '"').replace(/&#39;/g, "'");
const slug = (text) => stripTags(text).toLowerCase().replace(/[^a-z0-9\s-]/g, '').trim().replace(/\s+/g, '-');

const write = (path, data) => {
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, data);
};

// Markdown renderer: headings get ids. Tables are wrapped after rendering so
// they scroll on their own on phones.
const marked = new Marked({
  gfm: true,
  renderer: {
    heading({ tokens, depth }) {
      const html = this.parser.parseInline(tokens);
      const id = slug(html);
      return `<h${depth} id="${id}">${html}</h${depth}>\n`;
    },
  },
});

function renderGuide(name) {
  const source = readFileSync(join(root, 'docs', `${name}.md`), 'utf8');
  const html = marked.parse(source).replaceAll('<table>', '<div class="table-wrap"><table>').replaceAll('</table>', '</table></div>');
  const title = stripTags((html.match(/<h1[^>]*>(.*?)<\/h1>/s) ?? [, name])[1]);
  const description = shorten(stripTags((html.match(/<p>(.*?)<\/p>/s) ?? [, ''])[1]).replace(/\s+/g, ' '), 200);
  return { name, html, title, description };
}

/** Cuts `text` to at most `max` characters at a word boundary. */
function shorten(text, max) {
  if (text.length <= max) return text;
  const cut = text.slice(0, max - 3);
  return `${cut.slice(0, cut.lastIndexOf(' ')).replace(/[\s.,;:]+$/, '')}...`;
}

const head = ({ title, description, path }) => `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(title)}</title>
<meta name="description" content="${escapeHtml(description)}">
<meta name="theme-color" content="#0a0a0a">
<link rel="icon" type="image/svg+xml" href="/favicon.svg">
${path ? `<link rel="canonical" href="${SITE}${path}">` : '<meta name="robots" content="noindex">'}
<link rel="stylesheet" href="/docs.css">
</head>
<body>
<header class="top">
  <a class="brand" href="/" aria-label="FastEmu home"><img src="/assets/logo-dark.svg" alt="FastEmu" width="130" height="29"></a>
  <nav aria-label="Main">
    <a href="/docs/">Docs</a>
    <a href="${REPO}">GitHub</a>
    <a class="btn" href="${REPO}/releases/latest">Download</a>
  </nav>
</header>
`;

const footer = `<footer>
  <p>FastEmu is free software under the <a href="${REPO}/blob/main/LICENSE">GPL-3.0-or-later</a> license.
  <a href="${REPO}">GitHub</a></p>
</footer>
</body>
</html>
`;

function sidebar(guides, current) {
  const items = guides
    .map((g) => `<li><a href="/docs/${g.name}/"${g.name === current ? ' aria-current="page"' : ''}>${escapeHtml(g.title)}</a></li>`)
    .join('\n');
  return `<nav class="side" aria-label="Guides"><p class="side-title">Guides</p><ul>\n${items}\n</ul></nav>`;
}

function guidePage(guide, guides) {
  const i = guides.indexOf(guide);
  const prev = guides[i - 1];
  const next = guides[i + 1];
  const pager = `<nav class="pager" aria-label="Pages">
${prev ? `<a class="prev" href="/docs/${prev.name}/"><span>Previous</span>${escapeHtml(prev.title)}</a>` : '<span></span>'}
${next ? `<a class="next" href="/docs/${next.name}/"><span>Next</span>${escapeHtml(next.title)}</a>` : '<span></span>'}
</nav>`;
  const path = `/docs/${guide.name}/`;
  return `${head({ title: `${guide.title} | FastEmu docs`, description: guide.description, path })}
<div class="layout">
${sidebar(guides, guide.name)}
<main class="doc">
<article>
${guide.html}
</article>
<p class="edit"><a href="${REPO}/edit/main/docs/${guide.name}.md">Edit this page on GitHub</a></p>
${pager}
</main>
</div>
${footer}`;
}

function docsIndex(guides) {
  const cards = guides
    .map((g) => `<li><a href="/docs/${g.name}/"><strong>${escapeHtml(g.title)}</strong><span>${escapeHtml(g.description)}</span></a></li>`)
    .join('\n');
  return `${head({ title: 'Documentation | FastEmu', description: 'Guides for installing, playing and building FastEmu, the Java ME emulator.', path: '/docs/' })}
<div class="layout">
${sidebar(guides, '')}
<main class="doc">
<article>
<h1 id="documentation">Documentation</h1>
<p>Guides for FastEmu, the Java ME emulator written in Rust.</p>
<ul class="index">
${cards}
</ul>
</article>
</main>
</div>
${footer}`;
}

function notFoundPage() {
  return `${head({ title: 'Page not found | FastEmu', description: 'This page does not exist.', path: null })}
<main class="lost">
<h1>Page not found</h1>
<p>The page you asked for is not here. Try the <a href="/docs/">documentation</a> or go back to the <a href="/">home page</a>.</p>
</main>
${footer}`;
}

function sitemap(paths) {
  const urls = paths.map((p) => `  <url><loc>${SITE}${p}</loc></url>`).join('\n');
  return `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls}\n</urlset>\n`;
}

/** Lists every file under a directory, as paths relative to it. */
function walk(dir, base = dir) {
  return readdirSync(dir).flatMap((f) => {
    const p = join(dir, f);
    return statSync(p).isDirectory() ? walk(p, base) : [p.slice(base.length + 1).replaceAll('\\', '/')];
  });
}

/** Fails when an internal link or anchor in the output points nowhere. */
function checkLinks() {
  const problems = [];
  const files = walk(out).filter((f) => f.endsWith('.html'));
  const ids = new Map();
  const idsOf = (file) => {
    if (!ids.has(file)) {
      const html = readFileSync(join(out, file), 'utf8');
      ids.set(file, new Set([...html.matchAll(/\sid="([^"]+)"/g)].map((m) => m[1])));
    }
    return ids.get(file);
  };
  for (const file of files) {
    const html = readFileSync(join(out, file), 'utf8');
    // Relative links (`settings.md`, `#install`) resolve against the page's
    // own URL, as a browser would, so they are checked too. Absolute links
    // to the site itself are checked like root-relative ones.
    const page = new URL(file.replace(/(^|\/)index\.html$/, '$1'), `${SITE}/`);
    for (const [, href] of html.matchAll(/(?:href|src|data-play)="([^"]+)"/g)) {
      const url = new URL(href.replaceAll('&amp;', '&'), page);
      if (url.origin !== SITE) continue;
      const hash = decodeURIComponent(url.hash.slice(1));
      const clean = decodeURIComponent(url.pathname);
      const target = clean.endsWith('/') ? `${clean.slice(1)}index.html` : clean.slice(1);
      if (!existsSync(join(out, target))) {
        problems.push(`${file}: ${href} does not exist`);
      } else if (hash && target.endsWith('.html') && !idsOf(target).has(hash)) {
        problems.push(`${file}: ${href} has no such anchor`);
      }
    }
  }
  if (problems.length) {
    console.error(problems.join('\n'));
    throw new Error(`${problems.length} broken link(s)`);
  }
  return files.length;
}

function build() {
  rmSync(out, { recursive: true, force: true });
  mkdirSync(out, { recursive: true });

  // Landing page and the logo.
  for (const f of ['index.html', 'landing.css', 'landing.js', 'docs.css']) {
    cpSync(join(here, 'src', f), join(out, f));
  }
  for (const f of ['logo-dark.svg', 'logo-light.svg', 'logo.svg', 'icon.svg']) {
    cpSync(join(root, 'assets', f), join(out, 'assets', f));
  }
  cpSync(join(root, 'assets', 'icon.svg'), join(out, 'favicon.svg'));
  // Frames of real games for the landing page's cards, and their licenses.
  cpSync(join(here, 'src', 'games'), join(out, 'assets', 'games'), { recursive: true });

  // Guides.
  const guides = GUIDES.map(renderGuide);
  for (const g of guides) write(join(out, 'docs', g.name, 'index.html'), guidePage(g, guides));
  write(join(out, 'docs', 'index.html'), docsIndex(guides));

  const listed = new Set(readdirSync(join(root, 'docs')).filter((f) => f.endsWith('.md')).map((f) => f.slice(0, -3)));
  const missing = [...listed].filter((n) => !GUIDES.includes(n));
  if (missing.length) throw new Error(`docs/${missing[0]}.md is not in GUIDES in build.mjs`);

  write(join(out, '404.html'), notFoundPage());
  write(join(out, 'robots.txt'), `User-agent: *\nAllow: /\n\nSitemap: ${SITE}/sitemap.xml\n`);
  write(join(out, 'sitemap.xml'), sitemap(['/', '/docs/', ...guides.map((g) => `/docs/${g.name}/`)]));

  const pages = checkLinks();
  console.log(`built ${pages} pages into ${out}`);
}

build();
