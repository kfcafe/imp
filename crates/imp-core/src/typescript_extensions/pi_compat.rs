pub(super) const BUN_BRIDGE: &str = r#"
import { pathToFileURL } from 'node:url';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { writeFileSync } from 'node:fs';

const [,, action, entrypoint, payloadJson] = process.argv;
const payload = JSON.parse(payloadJson ?? 'null');
const tools = [];
const compatibility = {
  lifecycleEvents: new Set(),
  stubbedApis: new Set(),
  unsupportedApis: new Set(),
  customRenderers: new Set(),
};
const lifecycleHandlers = new Map();

const Type = {
  String: (options = {}) => ({ type: 'string', ...options }),
  Number: (options = {}) => ({ type: 'number', ...options }),
  Integer: (options = {}) => ({ type: 'integer', ...options }),
  Boolean: (options = {}) => ({ type: 'boolean', ...options }),
  Array: (items, options = {}) => ({ type: 'array', items, ...options }),
  Object: (properties = {}, options = {}) => ({ type: 'object', properties, ...options }),
  Optional: (schema) => ({ ...schema, __optional: true }),
  Union: (schemas, options = {}) => ({ anyOf: schemas, ...options }),
  Literal: (value, options = {}) => ({ const: value, ...options }),
};

const StringEnum = (values) => ({ enum: values });

const piModule = {
  Type,
  StringEnum,
  getAgentDir: () => process.cwd(),
  getSettingsListTheme: () => ({}),
  withFileMutationQueue: async (_path, fn) => fn(),
  createReadTool: () => unsupportedNativeTool('read'),
  createBashTool: () => unsupportedNativeTool('bash'),
  createEditTool: () => unsupportedNativeTool('edit'),
  createWriteTool: () => unsupportedNativeTool('write'),
};

const tuiModule = {
  Text: class Text { constructor(text) { this.text = text; } },
  Key: { ctrlShift: (key) => `ctrl+shift+${key}` },
  matchesKey: () => false,
  truncateToWidth: (s) => s,
  wrapTextWithAnsi: (s) => [s],
  Container: class Container {},
  SettingsList: class SettingsList {},
  SelectList: class SelectList {},
  DynamicBorder: class DynamicBorder {},
};

function unsupportedNativeTool(name) {
  compatibility.unsupportedApis.add(`create${name}Tool`);
  return {
    name,
    label: name,
    description: `${name} is not available through imp's Pi compatibility shim yet`,
    parameters: { type: 'object', properties: {} },
    async execute() { throw new Error(`create${name}Tool is not supported by imp TypeScript extensions yet`); },
  };
}

const originalImport = globalThis.__import ?? ((specifier) => import(specifier));
async function importWithShim(specifier) {
  if (specifier === '@mariozechner/pi-coding-agent' || specifier === '@mariozechner/pi-ai') return piModule;
  if (specifier === '@sinclair/typebox') return { Type };
  if (specifier === '@mariozechner/pi-tui') return tuiModule;
  return originalImport(specifier);
}

globalThis.__imp_import = importWithShim;

async function loadEntrypoint(path) {
  const source = await Bun.file(path).text();
  const rewritten = source
    .replaceAll('import { Type } from "@mariozechner/pi-coding-agent";', 'const { Type } = globalThis.__imp_pi;')
    .replaceAll("import { Type } from '@mariozechner/pi-coding-agent';", 'const { Type } = globalThis.__imp_pi;')
    .replaceAll('import { StringEnum } from "@mariozechner/pi-ai";', 'const { StringEnum } = globalThis.__imp_pi;')
    .replaceAll("import { StringEnum } from '@mariozechner/pi-ai';", 'const { StringEnum } = globalThis.__imp_pi;')
    .replaceAll('import { Type } from "@sinclair/typebox";', 'const { Type } = globalThis.__imp_typebox;')
    .replaceAll("import { Type } from '@sinclair/typebox';", 'const { Type } = globalThis.__imp_typebox;')
    .replaceAll('from "@mariozechner/pi-coding-agent"', 'from "data:text/javascript,export default globalThis.__imp_pi;"')
    .replaceAll("from '@mariozechner/pi-coding-agent'", "from 'data:text/javascript,export default globalThis.__imp_pi;'")
    .replaceAll('from "@mariozechner/pi-ai"', 'from "data:text/javascript,"')
    .replaceAll("from '@mariozechner/pi-ai'", "from 'data:text/javascript,'")
    .replaceAll('from "@sinclair/typebox"', 'from "data:text/javascript,"')
    .replaceAll("from '@sinclair/typebox'", "from 'data:text/javascript,'")
    .replaceAll('import { matchesKey, Text, truncateToWidth } from "@mariozechner/pi-tui";', 'const { matchesKey, Text, truncateToWidth } = globalThis.__imp_tui;')
    .replaceAll("import { matchesKey, Text, truncateToWidth } from '@mariozechner/pi-tui';", 'const { matchesKey, Text, truncateToWidth } = globalThis.__imp_tui;')
    .replaceAll('from "@mariozechner/pi-tui"', 'from "data:text/javascript,"')
    .replaceAll("from '@mariozechner/pi-tui'", "from 'data:text/javascript,'");
  const rewrittenPath = join(tmpdir(), `imp-ts-extension-entry-${process.pid}.ts`);
  writeFileSync(rewrittenPath, rewritten);
  return import(pathToFileURL(rewrittenPath).href);
}

globalThis.__imp_Type = Type;
globalThis.__imp_StringEnum = StringEnum;
globalThis.__imp_pi = piModule;
globalThis.__imp_typebox = { Type };
globalThis.__imp_tui = tuiModule;

function normalizeSchema(schema) {
  if (!schema || typeof schema !== 'object') return schema;
  if (schema.type === 'object' && schema.properties) {
    const required = [];
    const properties = {};
    for (const [key, value] of Object.entries(schema.properties)) {
      const normalized = normalizeSchema(value);
      if (!value.__optional) required.push(key);
      delete normalized.__optional;
      properties[key] = normalized;
    }
    return { ...schema, properties, required };
  }
  if (schema.anyOf) return { ...schema, anyOf: schema.anyOf.map(normalizeSchema) };
  if (schema.items) return { ...schema, items: normalizeSchema(schema.items) };
  return { ...schema };
}

const pi = {
  registerTool(def) {
    tools.push(def);
    if (typeof def.renderCall === 'function') compatibility.customRenderers.add(`${def.name}.renderCall`);
    if (typeof def.renderResult === 'function') compatibility.customRenderers.add(`${def.name}.renderResult`);
  },
  registerCommand() { compatibility.stubbedApis.add('registerCommand'); },
  on(event, handler) {
    compatibility.lifecycleEvents.add(event);
    const handlers = lifecycleHandlers.get(event) ?? [];
    handlers.push(handler);
    lifecycleHandlers.set(event, handlers);
  },
};

function extensionContext() {
  return {
    cwd: process.cwd(),
    hasUI: false,
    ui: {
      notify() { compatibility.stubbedApis.add('ui.notify'); },
      setStatus() { compatibility.stubbedApis.add('ui.setStatus'); },
      custom() {
        compatibility.unsupportedApis.add('ctx.ui.custom');
        throw new Error('ctx.ui.custom is not supported by imp TypeScript extensions yet');
      },
    },
    sessionManager: {
      getBranch() { compatibility.stubbedApis.add('sessionManager.getBranch'); return []; },
      getEntries() { compatibility.stubbedApis.add('sessionManager.getEntries'); return []; },
    },
  };
}

async function fireLifecycle(event) {
  const handlers = lifecycleHandlers.get(event) ?? [];
  for (const handler of handlers) {
    await handler({}, extensionContext());
  }
}

try {
  const mod = await loadEntrypoint(entrypoint);
  const init = mod.default ?? mod;
  if (typeof init !== 'function') throw new Error('extension default export is not a function');
  await init(pi);
  await fireLifecycle('session_start');

  if (action === 'inspect') {
    console.log(JSON.stringify(tools.map((tool) => ({
      name: tool.name,
      label: tool.label,
      description: tool.description,
      parameters: normalizeSchema(tool.parameters ?? { type: 'object', properties: {} }),
      compatibility: {
        lifecycleEvents: [...compatibility.lifecycleEvents],
        stubbedApis: [...compatibility.stubbedApis],
        unsupportedApis: [...compatibility.unsupportedApis],
        customRenderers: [...compatibility.customRenderers].filter((renderer) => renderer.startsWith(`${tool.name}.`)),
      },
    }))));
  } else if (action === 'execute') {
    const tool = tools.find((tool) => tool.name === payload.tool);
    if (!tool) throw new Error(`tool not registered: ${payload.tool}`);
    const result = await tool.execute('imp-ts-call', payload.params ?? {}, new AbortController().signal, () => {}, extensionContext());
    console.log(JSON.stringify(result));
  } else {
    throw new Error(`unknown bridge action: ${action}`);
  }
} catch (error) {
  console.error(error?.stack ?? String(error));
  process.exit(1);
}
"#;
