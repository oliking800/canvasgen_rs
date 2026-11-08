// Render the canvas test in headless Chrome and dump pixels via --dump-dom
// (the page writes the data URL into the DOM).
const { spawnSync } = require('child_process');
const fs = require('fs');
const path = require('path');
const os = require('os');

const chrome = 'C:/Program Files/Google/Chrome/Application/chrome.exe';
const src = 'C:/Users/oli/Desktop/canvasgen_rs/tools/chrome_shadow_test.html';
const html = fs.readFileSync(src, 'utf8').replace(
  '</script>',
  `document.body.setAttribute('data-png', c.toDataURL());</script>`
);
const tmp = path.join(os.tmpdir(), 'canvas_shadow_test.html');
fs.writeFileSync(tmp, html);

const r = spawnSync(chrome, [
  '--headless=new', '--disable-gpu', '--no-sandbox', '--hide-scrollbars',
  '--virtual-time-budget=2000', '--dump-dom', 'file:///' + tmp.replace(/\\/g, '/'),
], { encoding: 'utf8', timeout: 60000 });

const m = (r.stdout || '').match(/data-png="data:image\/png;base64,([^"]+)"/);
if (!m) {
  console.error('no data url', r.stdout.slice(0, 500), r.stderr.slice(0, 500));
  process.exit(1);
}
fs.writeFileSync('C:/Users/oli/Desktop/canvasgen_rs/samples/chrome_shadow_test.png',
                 Buffer.from(m[1], 'base64'));
console.log('wrote chrome_shadow_test.png');
