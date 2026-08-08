import { readFileSync } from 'node:fs';

const profile = JSON.parse(readFileSync('spec/tensor-profile-v0.1.json', 'utf8'));
const schema = JSON.parse(readFileSync('spec/tensor-profile.schema.json', 'utf8'));
const graphSchema = JSON.parse(readFileSync('spec/typed-graph-ir.schema.json', 'utf8'));
const coreWords = JSON.parse(readFileSync('spec/words.json', 'utf8'));
const prose = readFileSync('spec/tensor-profile-v0.1.md', 'utf8');
const runtimeDtypes = readFileSync('rust/src/tensor_profile/tensor.rs', 'utf8');
const referenceCpu = readFileSync('rust/src/tensor_profile/cpu.rs', 'utf8');
const reductions = readFileSync('rust/src/tensor_profile/reductions.rs', 'utf8');
const elementwise = readFileSync('rust/src/tensor_profile/elementwise.rs', 'utf8');
const select = readFileSync('rust/src/tensor_profile/select.rs', 'utf8');
const regrid = readFileSync('rust/src/tensor_profile/regrid.rs', 'utf8');
const graphOperators = readFileSync('rust/src/tensor_profile/graph_operators.rs', 'utf8');
const graphExample = JSON.parse(readFileSync('spec/examples/tiny-matmul.graph.json', 'utf8'));
const errors = [];
const fail = (message) => errors.push(message);

if (profile.schemaVersion !== schema.properties.schemaVersion.const) fail('schemaVersion does not match the profile schema');
if (!schema.properties.profile.pattern || !new RegExp(schema.properties.profile.pattern).test(profile.profile)) fail(`invalid profile identifier: ${profile.profile}`);
if (!prose.includes(profile.profile)) fail('normative prose does not name the machine-readable profile identifier');
if (!graphSchema.properties.profiles) fail('typed graph IR has no explicit profile selection');
if (profile.implicitCasts !== false) fail('exact Scalar to approximate Tensor conversion must remain explicit');
if (profile.ambientRng !== false) fail('ambient RNG is forbidden');
for (const dtype of [...profile.dtypes, profile.predicateDtype]) {
  const rustVariant = dtype[0].toUpperCase() + dtype.slice(1);
  if (!runtimeDtypes.includes(`    ${rustVariant},`)) fail(`runtime DType is missing ${dtype}`);
}
// The predicate dtype only stays outside the numeric domain if something
// mechanically keeps it there: `is_numeric` must exclude it, and both the
// graph validator and the reference backend must refuse it for arithmetic.
if (profile.dtypes.includes(profile.predicateDtype)) fail('the predicate dtype must not be listed as a numeric dtype');
if (!/pub const fn is_numeric\(self\) -> bool \{\s*matches!\(self, ([^)]*)\)/.test(runtimeDtypes)) fail('runtime DType::is_numeric is not a simple dtype list');
else {
  const numericVariants = runtimeDtypes.match(/pub const fn is_numeric\(self\) -> bool \{\s*matches!\(self, ([^)]*)\)/)[1].split('|').map((v) => v.trim().replace('Self::', '').toLowerCase());
  const predicateVariant = profile.predicateDtype;
  if (numericVariants.includes(predicateVariant)) fail('runtime DType::is_numeric does not exclude the predicate dtype');
  for (const dtype of profile.dtypes) if (!numericVariants.includes(dtype)) fail(`runtime DType::is_numeric omits the declared numeric dtype ${dtype}`);
}
// Exactness must be a property of the dtype, not a convention: the domain each
// operator is written over is declared, and the runtime must gate on it.
if (!/pub const fn is_exact\(self\) -> bool/.test(runtimeDtypes)) fail('runtime DType does not distinguish exact from approximate dtypes');
if (!referenceCpu.includes('pub(crate) fn require_approximate(')) fail('reference CPU backend does not gate approximate-only operators');
if (!referenceCpu.includes('pub(crate) fn require_exact(')) fail('reference CPU backend does not gate exact-only operators');
if (!graphOperators.includes('fn require_domain(')) fail('graph validation does not gate operators on their declared dtype domain');
if (!regrid.includes('pub fn tensor_regrid(')) fail('reference CPU backend is missing REGRID');
for (const operator of profile.operators) {
  if (operator.dtypeDomain === 'exact' && operator.differentiation.kind !== 'none') fail(`${operator.name} rounds onto a grid, so it cannot declare a VJP`);
}
if (!profile.dtypes.some((dtype) => dtype === 'q')) fail('the profile declares no exact dtype for the growth strategy to bound');
if (!/max_denominator_bits/.test(readFileSync('rust/src/tensor_profile/shape.rs', 'utf8'))) fail('the memory budget carries no denominator ceiling');
if (!referenceCpu.includes('pub(crate) fn require_numeric(')) fail('reference CPU backend does not gate numeric operators on dtype');
if (!graphOperators.includes('fn require_numeric(')) fail('graph validation does not gate numeric operators on dtype');

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
if (!referenceCpu.includes('pub fn tensor_exp(')) fail('reference CPU backend is missing EXP');
if (!referenceCpu.includes('pub fn tensor_log(')) fail('reference CPU backend is missing LOG');
if (!referenceCpu.includes('pub fn tensor_rsqrt(')) fail('reference CPU backend is missing RSQRT');
if (!select.includes('pub fn tensor_where(')) fail('reference CPU backend is missing WHERE');
if (!reductions.includes('pub fn reduce_sum(')) fail('reference CPU backend is missing REDUCE_SUM');
if (!reductions.includes('pub fn reduce_max(')) fail('reference CPU backend is missing REDUCE_MAX');
for (const operation of ['add', 'sub', 'mul', 'div']) {
  if (!elementwise.includes(`pub fn tensor_${operation}(`)) fail(`reference CPU backend is missing elementwise ${operation.toUpperCase()}`);
}

if (errors.length) {
  for (const error of errors) console.error(`[tensor-profile] ${error}`);
  process.exitCode = 1;
} else {
  console.log(`[tensor-profile] ${profile.profile}: ${names.size} unique operators; explicit casts/RNG, numeric bounds, VJPs, resources, and graph profile selection verified.`);
}
