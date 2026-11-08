const { spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');

const chrome = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const src = 'C:/Users/oli/Desktop/canvasgen_rs/tools/triangle_candidates.html';
const tmp = path.join(os.tmpdir(), 'triangle_candidates.html');
fs.copyFileSync(src, tmp);
const r = spawnSync(chrome, [
  '--headless=new', '--no-sandbox', '--hide-scrollbars', '--use-angle=d3d11',
  '--user-data-dir=' + path.join(os.tmpdir(), 'chrome-tri-' + Date.now()),
  '--virtual-time-budget=5000', '--dump-dom', 'file:///' + tmp.replace(/\\/g, '/'),
], { encoding: 'utf8', timeout: 90000 });
const m = (r.stdout || '').match(/data-out="([^"]+)"/);
if (!m) { console.error('no output', (r.stdout || '').slice(0, 300), (r.stderr || '').slice(0, 300)); process.exit(1); }
const json = m[1].replace(/&quot;/g, '"').replace(/&amp;/g, '&');
const urls = JSON.parse(json);
fs.writeFileSync('C:/Users/oli/Desktop/canvasgen_rs/tools/triangle_candidates.json', JSON.stringify(urls));
console.log('wrote', urls.length, 'candidates');
