// Cleans the compiled Tailwind stylesheet before the design sync, without
// changing how anything renders. Claude Design's token scan treats custom
// properties declared in simple class rules as design tokens, and flags theme
// variables whose type it cannot infer. Tailwind's internals (--tw-*, easings,
// animations) are neither, so:
//   1. Utility rules that set custom properties get their selector wrapped in
//      :is(), which keeps the same matches and specificity but is no longer a
//      plain class selector.
//   2. Theme and @layer properties variables with no inferable type are marked
//      with a /* @kind other */ comment, as Claude Design asks.
// Usage: node .design-sync/postprocess-css.mjs <file.css>
import { readFileSync, writeFileSync } from 'node:fs';
import { createRequire } from 'node:module';

const require = createRequire(new URL('../.ds-sync/package.json', import.meta.url));
const postcss = require('postcss');

const file = process.argv[2];
const root = postcss.parse(readFileSync(file, 'utf8'));
const UNTYPED = /^--(tw-|ease|animate|default-transition|scroll-fade|shimmer)/;

const layerOf = (node) => {
  for (let p = node.parent; p; p = p.parent) {
    if (p.type === 'atrule' && p.name === 'layer') return p.params;
  }
  return null;
};

let wrapped = 0;
let marked = 0;
root.walkRules((rule) => {
  const vars = rule.nodes.filter((n) => n.type === 'decl' && n.prop.startsWith('--'));
  if (vars.length === 0) return;
  const layer = layerOf(rule);

  if (layer === 'utilities') {
    // Nested rules use `&` and inherit their parent's selector; pseudo-elements
    // are not allowed inside :is().
    if (rule.parent.type === 'rule' || rule.selector.includes('::')) return;
    rule.selectors = rule.selectors.map((s) => (s.startsWith(':is(') ? s : `:is(${s})`));
    wrapped++;
    return;
  }

  if (layer === 'theme' || layer === 'properties') {
    for (const decl of vars) {
      if (!UNTYPED.test(decl.prop)) continue;
      const next = decl.next();
      if (next?.type === 'comment' && next.text === '@kind other') continue;
      decl.after(postcss.comment({ text: '@kind other' }));
      marked++;
    }
  }
});

writeFileSync(file, root.toString());
console.error(`postprocess-css: ${wrapped} utility rule(s) wrapped in :is(), ${marked} variable(s) marked @kind other`);
