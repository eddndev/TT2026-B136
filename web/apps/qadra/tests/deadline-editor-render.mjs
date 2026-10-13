import { readFile } from 'node:fs/promises';
import { resolve, dirname } from 'node:path';
import { pathToFileURL } from 'node:url';
import { compile } from 'svelte/compiler';
import { render } from 'svelte/server';
export async function componentMarkup(name, props) {
  async function moduleUrl(path) {
    const source = await readFile(path, 'utf8');
    let code = compile(source, { filename: path, generate: 'server' }).js.code;
    const imports = [...code.matchAll(/from ['"]([^'"]+)['"]/g)];
    for (const match of imports) {
      const target = match[1];
      const url = target.endsWith('.svelte')
        ? await moduleUrl(resolve(dirname(path), target))
        : target.startsWith('.')
          ? pathToFileURL(resolve(dirname(path), target)).href
          : import.meta.resolve(target);
      code = code.replace(match[0], `from ${JSON.stringify(url)}`);
    }
    return `data:text/javascript;base64,${Buffer.from(code).toString('base64')}`;
  }
  const path = new URL(`../src/components/${name}.svelte`, import.meta.url).pathname;
  const component = await import(await moduleUrl(path));
  return render(component.default, { props }).body;
}
