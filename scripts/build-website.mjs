import { readFile, writeFile, mkdir, copyFile, rm } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { resolve } from 'node:path';
import { content } from '../website/content.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const output = resolve(root, 'website/dist');
const baseUrl = 'https://sobigrice.github.io/port-guardian/';
const repository = 'https://github.com/soBigRice/port-guardian';
const headers = { Accept: 'application/vnd.github+json', 'User-Agent': 'port-guardian-website' };
if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;
const response = await fetch('https://api.github.com/repos/soBigRice/port-guardian/releases/latest', {
  headers, signal: AbortSignal.timeout(15000),
});
if (!response.ok) throw new Error(`Cannot verify published downloads: GitHub returned ${response.status}`);
const release = await response.json();
if (release.draft || release.prerelease || typeof release.tag_name !== 'string' || !Array.isArray(release.assets)) {
  throw new Error('Expected a published stable release.');
}
const mac = release.assets.find((asset) => /_universal\.dmg$/.test(asset.name));
const windows = release.assets.find((asset) => /_x64-setup\.exe$/.test(asset.name));
for (const asset of [mac, windows]) {
  if (!asset?.browser_download_url?.startsWith(`${repository}/releases/download/`) || !(asset.size > 0)) {
    throw new Error('The latest release must include a universal macOS DMG and Windows x64 EXE.');
  }
}
const template = await readFile(resolve(root, 'website/index.html'), 'utf8');
const escape = (value) => String(value).replace(/[&<>"']/g, (character) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[character]);

// Only this script's generated directory is replaced; sources and app builds stay intact.
await rm(output, { recursive: true, force: true });
await mkdir(resolve(output, 'en'), { recursive: true });
await mkdir(resolve(output, 'assets'), { recursive: true });
for (const [language, copy] of Object.entries(content)) {
  const english = language === 'en';
  const data = {
    ...copy, canonical: `${baseUrl}${english ? 'en/' : ''}`, zhUrl: baseUrl, enUrl: `${baseUrl}en/`,
    assetBase: english ? '../' : './', homeHref: './',
    zhHref: english ? '../' : './', enHref: english ? './' : './en/',
    macUrl: mac.browser_download_url, windowsUrl: windows.browser_download_url,
    version: release.tag_name,
    readmeUrl: `${repository}/blob/main/${english ? 'README_EN.md' : 'README.md'}#${english ? '-quick-start' : '-快速开始'}`,
  };
  const html = template.replace(/\{\{(\w+)\}\}/g, (_, key) => {
    if (key === 'zhCurrent') return english ? '' : 'aria-current="page"';
    if (key === 'enCurrent') return english ? 'aria-current="page"' : '';
    if (!(key in data)) throw new Error(`Missing ${language} content: ${key}`);
    return escape(data[key]);
  });
  if (/\{\{/.test(html)) throw new Error('Unresolved template token.');
  await writeFile(resolve(output, english ? 'en/index.html' : 'index.html'), html);
}
await copyFile(resolve(root, 'website/site.css'), resolve(output, 'site.css'));
await copyFile(resolve(root, 'website/site.js'), resolve(output, 'site.js'));
await copyFile(resolve(root, 'src-tauri/icons/128x128.png'), resolve(output, 'assets/icon.png'));
await copyFile(resolve(root, 'docs/design/project-workspace-implemented.png'), resolve(output, 'assets/app-preview.png'));
await writeFile(resolve(output, '.nojekyll'), '');
await writeFile(resolve(output, 'robots.txt'), `User-agent: *\nAllow: /\nSitemap: ${baseUrl}sitemap.xml\n`);
await writeFile(resolve(output, 'sitemap.xml'), `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9"><url><loc>${baseUrl}</loc></url><url><loc>${baseUrl}en/</loc></url></urlset>\n`);
console.log(`Built Chinese and English pages with verified ${release.tag_name} downloads → website/dist`);
