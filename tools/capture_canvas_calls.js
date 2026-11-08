// capture_canvas_calls.js -- run the real hsw proof and log every canvas 2D
// property set / method call it performs.
const fs = require('fs');
const path = require('path');
const ROOT = 'C:/Users/oli/Desktop/hcaptcha_research';
const { makeSandbox } = require(path.join(ROOT, 'node_hsw/hsw_lib.js'));
const ENV = JSON.parse(fs.readFileSync(path.join(ROOT, 'proof_emulation/env827.json'), 'utf8'));
let INSTALLER = fs.readFileSync(path.join(ROOT, 'proof_emulation/env_installer.js'), 'utf8');

// 1. trace array + logger
INSTALLER = INSTALLER.replace(
  'const ENV = globalThis.__ENV;',
  'const ENV = globalThis.__ENV;\n  globalThis.__canvasTrace = [];\n' +
  '  const __clog = (kind, name, args) => { try { globalThis.__canvasTrace.push({kind, name, args}); } catch(e){} };'
);

// 2. property setters log
INSTALLER = INSTALLER.replace(
  "set:mkFn('set '+k,1,null,(v)=>{state[k]=v})",
  "set:mkFn('set '+k,1,null,(v)=>{state[k]=v;__clog('set',k,[v])})"
);

// 3. noop methods log their arguments
INSTALLER = INSTALLER.replace(
  'const noop = (n)=>mkFn(n,0,null,function(){});',
  'const noop = (n)=>mkFn(n,0,null,function(){__clog("call",n,Array.prototype.slice.call(arguments))});'
);

// 4. measureText / createLinearGradient / toDataURL log
INSTALLER = INSTALLER.replace(
  "mkFn('measureText',1,null,()=>({width:8,actualBoundingBoxAscent:6,actualBoundingBoxDescent:2,actualBoundingBoxLeft:0,actualBoundingBoxRight:8}))",
  "mkFn('measureText',1,null,(s)=>{__clog('call','measureText',[s]);return {width:8,actualBoundingBoxAscent:6,actualBoundingBoxDescent:2,actualBoundingBoxLeft:0,actualBoundingBoxRight:8}})"
);
INSTALLER = INSTALLER.replace(
  "mkFn('createLinearGradient',4,null,()=>makeGradient())",
  "mkFn('createLinearGradient',4,null,function(){globalThis.__gradCoords=Array.prototype.slice.call(arguments);__clog('call','createLinearGradient',globalThis.__gradCoords);return makeGradient()})"
);
INSTALLER = INSTALLER.replace(
  "mkFn('createRadialGradient',6,null,()=>makeGradient())",
  "mkFn('createRadialGradient',6,null,function(){globalThis.__gradCoords=Array.prototype.slice.call(arguments);__clog('call','createRadialGradient',globalThis.__gradCoords);return makeGradient()})"
);
INSTALLER = INSTALLER.replace(
  "const i = (globalThis.__canvasIdx = (globalThis.__canvasIdx||0)+1) - 1;\n      return q[i % (q.length||1)] || 'data:image/png;base64,';",
  "const i = (globalThis.__canvasIdx = (globalThis.__canvasIdx||0)+1) - 1;\n      const u = q[i % (q.length||1)] || 'data:image/png;base64,';\n      __clog('toDataURL', 'toDataURL', [i, u.length, cv.width, cv.height]);\n      return u;"
);

// 5. gradient stops
INSTALLER = INSTALLER.replace(
  "const g={}; wireProto(g,'CanvasGradient','CanvasGradient');\n    g.addColorStop = mkFn('addColorStop',2,null,function(){});",
  "const g={}; wireProto(g,'CanvasGradient','CanvasGradient');\n    g.__id = (globalThis.__gid=(globalThis.__gid||0)+1);\n    g.__coords = globalThis.__gradCoords;\n    try{ g.__stack = new Error().stack; }catch(e){}\n    __clog('gradNew', g.__id, [g.__coords, g.__stack]);\n    g.addColorStop = mkFn('addColorStop',2,null,function(o,c){__clog('gradStop', g.__id, [o,c]); g.__last=[o,c];});"
);

INSTALLER = INSTALLER.replace(
  "cv.getContext = mkFn('getContext',1,null,function(type,attrs){",
  "cv.getContext = mkFn('getContext',1,null,function(type,attrs){__clog('getContext','getContext',[type, cv.width, cv.height]);"
);

const preload = 'globalThis.__ENV=' + JSON.stringify(ENV) + ';\n' + INSTALLER;

const sb = makeSandbox({
  src: path.join(ROOT, 'assets/hsw_82745a9a.js'),
  feats: ['audio', 'rtc', 'fetch', 'protos', 'chrome'],
  preloadCode: preload,
});

(async () => {
  const jwt = fs.readFileSync('C:/Users/oli/Desktop/hsw_rust/assets/test.jwt', 'utf8').trim();
  const obj = JSON.parse(fs.readFileSync('C:/Users/oli/Desktop/hsw_rust/assets/test.profile.json', 'utf8'));
  await sb.hsw('IiI=.eyJzIjowLCJmIjowLCJjIjowfQ==.');
  const n = await sb.hsw(jwt, obj);
  const trace = sb.__canvasTrace || [];
  fs.writeFileSync('canvas_calls.json', JSON.stringify(trace, null, 1));
  console.error('n_len', n.length, 'canvas_events', trace.length);
  const sets = {};
  const calls = {};
  for (const e of trace) {
    if (e.kind === 'set') sets[e.name] = e.args[0];
    else calls[e.name] = (calls[e.name] || 0) + 1;
  }
  console.error('PROPS', JSON.stringify(sets, null, 1));
  console.error('CALLS', JSON.stringify(calls, null, 1));
  // print text/draw calls in order
  for (const e of trace) {
    if (e.kind === 'call' && ['fillText', 'strokeText', 'fillRect', 'strokeRect', 'arc', 'moveTo', 'lineTo', 'bezierCurveTo', 'quadraticCurveTo', 'ellipse', 'rect', 'drawImage', 'measureText'].includes(e.name)) {
      console.error('EV', e.name, JSON.stringify(e.args).slice(0, 300));
    }
  }
  process.exit(0);
})().catch((e) => { console.error('ERR', e && e.stack || e); process.exit(1); });
