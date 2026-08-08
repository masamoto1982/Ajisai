import { readFileSync } from 'node:fs';

const profile = JSON.parse(readFileSync('spec/tensor-profile-v0.1.json', 'utf8'));
const schema = JSON.parse(readFileSync('spec/tensor-profile.schema.json', 'utf8'));
const graphSchema = JSON.parse(readFileSync('spec/typed-graph-ir.schema.json', 'utf8'));
const coreWords = JSON.parse(readFileSync('spec/words.json', 'utf8'));
const prose = readFileSync('spec/tensor-profile-v0.1.md', 'utf8');
const runtimeDtypes = readFileSync('rust/src/tensor_profile/tensor.rs', 'utf8');
const referenceCpu = readFileSync('rust/src/tensor_profile/cpu.rs', 'utf8');
const graphExample = JSON.parse(readFileSync('spec/examples/tiny-matmul.graph.json', 'utf8'));
const errors = [];
const fail = (message) => errors.push(message);

if (profile.schemaVersion !== schema.properties.schemaVersion.const) fail('schemaVersion does not match the profile schema');
if (!schema.properties.profile.pattern || !new RegExp(schema.properties.profile.pattern).test(profile.profile)) fail(`invalid profile identifier: ${profile.profile}`);
if (!prose.includes(profile.profile)) fail('normative prose does not name the machine-readable profile identifier');
if (!graphSchema.properties.profiles) fail('typed graph IR has no explicit profile selection');
if (profile.implicitCasts !== false) fail('exact Scalar to approximate Tensor conversion must remain explicit');
if (profile.ambientRng !== false) fail('ambient RNG is forbidden');
for (const dtype of profile.dtypes) {
  const rustVariant = dtype[0].toUpperCase() + dtype.slice(1);
  if (!runtimeDtypes.includes(`    ${rustVariant},`)) fail(`runtime DType is missing ${dtype}`);
}

const coreNames = new Set(coreWords.entries.map(({ name }) => name));
const names = new Set();
const semanticIds = new Set();
for (const operator of profile.operators) {
  const where = `${operator.name || '<unnamed>'}`;
  for (const field of schema.$defs.operator.required) if (!(field in operator)) fail(`${where} lacks ${field}`);
  if (names.has(operator.name)) fail(`duplicate operator name: ${operator.name}`);
  if (semanticIds.has(operator.semanticId)) fail(`duplicate semanticId: ${operator.semanticId}`);
  names.add(operator.name);
  semanticIds.add(operator.semanticId);
  if (coreNames.has(operator.name)) fail(`${operator.name} collides with frozen Core vocabulary`);
  if (!/^tensor\.[a-z0-9_]+\.v[1-9][0-9]*$/.test(operator.semanticId)) fail(`${where} has a non-versioned semanticId`);
  if (!['bitwise', 'bounded'].includes(operator.determinism)) fail(`${where} has invalid determinism`);
  if (operator.determinism === 'bitwise' && (operator.numeric.absoluteTolerance !== 0 || operator.numeric.relativeTolerance !== 0)) fail(`${where} is bitwise but declares non-zero tolerance`);
  if (operator.determinism === 'bounded' && operator.numeric.absoluteTolerance === 0 && operator.numeric.relativeTolerance === 0) fail(`${where} is bounded but declares no bound`);
  if (operator.differentiation.kind === 'vjp' && !operator.differentiation.rule) fail(`${where} is differentiable but has no VJP rule`);
  if (operator.differentiation.kind === 'none' && operator.differentiation.rule !== null) fail(`${where} is non-differentiable but names a rule`);
  if (!operator.resource.compute || !operator.resource.memory) fail(`${where} lacks resource complexity`);
}

for (const required of ['CAST', 'MATMUL', 'REDUCE_SUM', 'EXP', 'LOG', 'RANDOM_UNIFORM', 'SPLIT_KEY']) {
  if (!names.has(required)) fail(`minimum profile is missing ${required}`);
}
if (!graphExample.profiles.includes(profile.profile)) fail('graph example does not select the Tensor Profile');
const exampleOperators = new Set(graphExample.nodes.map((node) => node.operatorSemanticId));
for (const operator of exampleOperators) {
  if (!semanticIds.has(operator)) fail(`graph example uses unregistered operator ${operator}`);
}
if (!referenceCpu.includes('pub fn matmul(')) fail('reference CPU backend is missing MATMUL');

if (errors.length) {
  for (const error of errors) console.error(`[tensor-profile] ${error}`);
  process.exitCode = 1;
} else {
  console.log(`[tensor-profile] ${profile.profile}: ${names.size} unique operators; explicit casts/RNG, numeric bounds, VJPs, resources, and graph profile selection verified.`);
}
