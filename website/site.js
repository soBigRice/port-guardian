const demo = document.querySelector('[data-demo]');
const port = document.querySelector('.port-demo');
const status = document.querySelector('[data-status]');
const demoText = document.querySelector('[data-demo-text]');
const message = document.querySelector('[data-error]');

// This is a visual demo. It never scans or terminates a local process.
if (demo && port && status && demoText && message) {
  demo.addEventListener('click', () => {
    const available = port.dataset.state !== 'available';
    port.dataset.state = available ? 'available' : 'occupied';
    status.textContent = available ? status.dataset.available : status.dataset.occupied;
    demoText.textContent = available ? demo.dataset.reset : demo.dataset.action;
    message.textContent = available ? 'READY TO DEVELOP' : 'EADDRINUSE';
  });
}

// Static release links keep downloads usable without JS or if GitHub rate-limits
// this optional check. Refresh links and their displayed version as one snapshot.
async function refreshRelease() {
  try {
    const response = await fetch('https://api.github.com/repos/soBigRice/port-guardian/releases/latest', {
      headers: { Accept: 'application/vnd.github+json' },
      signal: AbortSignal.timeout(5000),
    });
    if (!response.ok) return;
    const release = await response.json();
    if (release.draft || release.prerelease || !Array.isArray(release.assets)) return;
    const mac = release.assets.find((asset) => /_universal\.dmg$/.test(asset.name));
    const windows = release.assets.find((asset) => /_x64-setup\.exe$/.test(asset.name));
    const downloadPrefix = 'https://github.com/soBigRice/port-guardian/releases/download/';
    if (!mac?.browser_download_url?.startsWith(downloadPrefix) || !windows?.browser_download_url?.startsWith(downloadPrefix) || typeof release.tag_name !== 'string') return;
    document.querySelector('[data-platform="mac"]').href = mac.browser_download_url;
    document.querySelector('[data-platform="windows"]').href = windows.browser_download_url;
    document.querySelector('[data-version]').textContent = release.tag_name;
  } catch {
    // Optional freshness lookup: the build-time verified downloads remain valid.
  }
}
void refreshRelease();
