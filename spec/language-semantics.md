
<p class="doc-nav"><a href="https://github.com/masamoto1982/Ajisai#readme">README</a> · <a href="docs/index.html">Reference</a> · <a href="https://masamoto1982.github.io/Ajisai/">Playground</a></p>

<h1 id="ajisai-language-semantics">Ajisai Language Semantics</h1>

<p>Status: <strong>Canonical</strong><br>
Version: <strong>2026-07-28</strong></p>

<p>
This document defines the correspondence between Ajisai source programs and observable values, states, effects, and diagnoses. It is intentionally a compact semantic kernel: differences between individual Words belong to the machine-readable vocabulary registry, not to parallel prose definitions.
</p>

<nav class="toc">
<h2>Contents</h2>
<ol>
<li><a href="#lang-authority">Authority and Compatibility</a></li>
<li><a href="#lang-source">Source and Desugaring</a></li>
<li><a href="#lang-values">Value Domains</a></li>
<li><a href="#lang-machine">Machine State and Evaluation</a></li>
<li><a href="#lang-modifiers">Stack and Modifiers</a></li>
<li><a href="#lang-failure">Partiality and Failure</a></li>
<li><a href="#lang-collections">Collections and Higher-order Evaluation</a></li>
<li><a href="#lang-dictionary">Dictionary, Modules, and Effects</a></li>
<li><a href="#lang-observation">Observation and Host Protocol</a></li>
<li><a href="#lang-conformance">Conformance</a></li>
</ol>
</nav>

<h2 id="lang-authority">1. Authority and Compatibility</h2>

<h3 id="lang-authority-sources">LANG.AUTHORITY.SOURCES — Normative sources</h3>

<p>
This Language Semantics is authoritative for program meaning. <code>spec/gui-semantics.md</code> is authoritative for presentation. <code>spec/host-protocol-v1.schema.json</code> is authoritative for their compatibility boundary. <code>SPECIFICATION.html</code> is generated from those sources and is not edited directly.
</p>

<p>
The semantic-family registry classifies shared Word behavior. The generated Word manifest records the complete visible vocabulary until each family is migrated to the single Word schema. Neither implementation layout nor explanatory text can override an observable contract.
</p>

<h3 id="lang-authority-identity">LANG.AUTHORITY.IDENTITY — Language identity</h3>

<p>
Ajisai identity is the correspondence from normalized source to the ordered observation of stack, output, dictionary state, hosted effects, and structured diagnosis. Two implementations are semantically equivalent when that correspondence agrees for every conforming program.
</p>

<p>
The current 224 Word, alias, and surface-form inventory is preserved. Compaction changes where meaning is recorded, not the language surface or its results.
</p>

<h3 id="lang-authority-freedom">LANG.AUTHORITY.FREEDOM — Implementation freedom</h3>

<p>
AST, IR, dispatch, caching, scheduling, storage layout, numeric representation, and optimization are unobservable. An implementation may change them when all observations and HostProtocolV1 payload meanings remain unchanged.
</p>

<p>
Internal exact-real representation, Rust enum names, debug strings, allocation identity, GUI colors, and private serialization are not semantic discriminants.
</p>

<h3 id="lang-authority-legacy">LANG.AUTHORITY.LEGACY — Legacy correspondence</h3>

<p>
<code>spec/legacy-clause-map.json</code> maps every heading in the pre-compaction integrated specification to a kernel clause. <code>spec/legacy/integrated-specification-2026-07-15.html</code> preserves the prior normative wording for audit. The map preserves traceability; this document remains the active authority.
</p>

<h2 id="lang-source">2. Source and Desugaring</h2>

<h3 id="lang-source-text">LANG.SOURCE.TEXT — Source domain</h3>

<p>
A program is Unicode text tokenized by the sealed Ajisai lexical grammar. Tokens distinguish literals, canonical Words, aliases, modifier forms, code blocks, vectors, definitions, deletion, module operations, and reserved syntax.
</p>

<p>
Malformed delimiters, malformed literals, reserved tokens, invalid names, and invalid definition forms are source errors. They do not denote NIL.
</p>

<h3 id="lang-source-normalize">LANG.SOURCE.NORMALIZE — Name normalization</h3>

<p>
Word lookup is case-insensitive through the existing canonical normalization. A visible symbolic alias resolves to exactly the same canonical Word contract and executor as its English name.
</p>

<p>
Normalization does not merge distinct value tags, invent dictionary entries, or change string and code-block contents.
</p>

<h3 id="lang-source-desugar">LANG.SOURCE.DESUGAR — Surface forms</h3>

<p>
Desugaring is deterministic and semantics-preserving. Modifier punctuation, sealed control sugar, conversion forms, import forms, and other registered surface syntax lower to their existing canonical concepts before evaluation.
</p>

<p>
If (D) is desugaring and (O) observation, then (O(p)=O(D(p))) for every well-formed program (p). Sugar cannot introduce a new value domain or failure category.
</p>

<h3 id="lang-source-code">LANG.SOURCE.CODE — Code values</h3>

<p>
A code block is a tagged value containing source for later evaluation. Producing, storing, displaying, and evaluating code are distinct operations. Evaluation occurs only through a Word whose contract requests it.
</p>

<p>
Quoted code is not eagerly executed. Nested code and vector delimiters retain their existing parse boundaries.
</p>

<h2 id="lang-values">3. Value Domains</h2>

<h3 id="lang-values-disjoint">LANG.VALUES.DISJOINT — Tagged domains</h3>

<p>
Values form a disjoint tagged sum. Scalar, Boolean truth, String, Vector, Record, NIL, CodeBlock, ProcessHandle, SupervisorHandle, and other registered protocol kinds are never equal merely because their encodings resemble one another.
</p>

<p>
In particular FALSE is not scalar zero, TRUE is not scalar one, UNKNOWN is not NIL, and an absent value is not an ERROR.
</p>

<h3 id="lang-values-exact">LANG.VALUES.EXACT — Exact scalars</h3>

<p>
Every admitted scalar denotes an exact real in the current exact rational and multiquadratic domain. Arithmetic performs no intermediate rounding. A representation is conforming when refinement and comparison observations agree with the denoted value.
</p>

<p>
The internal exact-real representation is unobservable. Canonical continued-fraction display is derived from a value; it is not evidence of the runtime storage form.
</p>

<h3 id="lang-values-truth">LANG.VALUES.TRUTH — Three-valued truth</h3>

<p>
Truth values are TRUE, FALSE, and UNKNOWN. UNKNOWN represents logical undecidability under the applicable comparison observation and follows the registered Kleene three-valued truth tables.
</p>

<p>
UNKNOWN carries <code>truthValue = unknown</code> on protocol surfaces and never carries absence metadata. Exhausting comparison water yields UNKNOWN rather than NIL or ERROR.
</p>

<h3 id="lang-values-nil">LANG.VALUES.NIL — Diagnostic absence</h3>

<p>
NIL is a value representing absence from a well-formed partial operation. Its protocol semantics may carry reason, origin, recoverability, and structured diagnosis. These fields are observable when present.
</p>

<p>
NIL reason identifies why production failed; origin identifies where the absence arose; recoverability describes permitted recovery; diagnosis supplies stable machine-readable evidence and next checks.
</p>

<h3 id="lang-values-collections">LANG.VALUES.COLLECTIONS — Structured values</h3>

<p>
A Vector is an ordered finite collection and a Record is a tagged keyed collection under their existing contracts. Shape is semantic even when storage is flattened, sparse, shared, or lazily materialized.
</p>

<p>
Strings and encoded values retain their interpretation role and encoding contract; they are not guessed from numeric contents.
</p>

<h3 id="lang-values-roles">LANG.VALUES.ROLES — Interpretation roles</h3>

<p>
A value is observed as data paired with an interpretation role. Roles include unassigned, raw number, interval, text, truth value, timestamp, and nil. Rendering derives from the pair rather than from GUI heuristics.
</p>

<p>
Changing a display role cannot change exact value identity, stack behavior, dictionary resolution, or failure classification.
</p>

<h2 id="lang-machine">4. Machine State and Evaluation</h2>

<h3 id="lang-machine-state">LANG.MACHINE.STATE — State</h3>

<p>
A machine state contains the data stack, dictionary and module state, output stream, ordered hosted-effect sequence, child-runtime state, execution controls, and diagnostic context needed by observable contracts.
</p>

<p>
Host-only caches, allocation arenas, compiled plans, and counters are not semantic state unless explicitly exposed as diagnostic protocol fields.
</p>

<h3 id="lang-machine-transformers">LANG.MACHINE.TRANSFORMERS — Programs</h3>

<p>Each executable token denotes a partial state transformer. A program denotes left-to-right composition of those transformers after desugaring and name resolution.</p>

<p>Successful composition is deterministic relative to the initial semantic state and ordered host responses. Optimization may reassociate internal work only when the observable sequence is unchanged.</p>

<h3 id="lang-machine-word-contract">LANG.MACHINE.WORD — Word contracts</h3>

<p>A canonical Word contract selects a semantic family and supplies its differences: stack arity, consumption, NIL policy, projection condition and reason, error conditions, purity, determinism, capabilities, effects, interpretation role, clause links, documentation, and executor key.</p>

<p>The executor must refine its contract. Aliases and documentation are projections of the same canonical entry, not independent semantic authorities.</p>

<h3 id="lang-machine-order">LANG.MACHINE.ORDER — Evaluation order</h3>

<p>Token evaluation, output emission, dictionary mutation, hosted effects, and diagnosis events preserve their existing observable order. Parallel or hedged execution may occur only behind an order-preserving commit boundary.</p>

<p>Cancelled speculative work contributes no semantic effect.</p>

<h3 id="lang-machine-limits">LANG.MACHINE.LIMITS — Work limits</h3>

<p>Execution-step, recursion, comparison, and materialization limits retain distinct meanings. A malformed program raises its registered ERROR; step or recursion exhaustion raises its registered limit ERROR; comparison exhaustion yields UNKNOWN; a well-formed generative space miss projects to reasoned NIL.</p>

<h2 id="lang-modifiers">5. Stack and Modifiers</h2>

<h3 id="lang-stack-order">LANG.STACK.ORDER — Stack observation</h3>

<p>The stack is an ordered sequence with a distinguished top. Canonical stack collection and lossless stack snapshot are separate observations and retain their HostProtocolV1 shapes.</p>

<p>A display transformation cannot reorder, coerce, drop, or invent stack values.</p>

<h3 id="lang-modifiers-axes">LANG.MODIFIERS.AXES — Orthogonal axes</h3>

<p>TOP versus STAK selects the operand region. EAT versus KEEP selects consumption. These axes are orthogonal; choosing one never implies a value on the other.</p>

<p>The default is TOP with EAT. Existing punctuation and named forms desugar to one point in the Cartesian product ({TOP,STAK}\times\{EAT,KEEP}).</p>

<h3 id="lang-modifiers-application">LANG.MODIFIERS.APPLICATION — Application</h3>

<p>A Word first selects operands according to its target axis, validates the registered contract, computes or projects the result, and commits consumption according to its consumption axis. ERROR does not masquerade as a successful NIL projection.</p>

<p>KEEP retains selected operands in their existing order; EAT applies the Word contract's existing consumption behavior.</p>

<h2 id="lang-failure">6. Partiality and Failure</h2>

<h3 id="lang-failure-trichotomy">LANG.FAILURE.TRICHOTOMY — Value, absence, misuse</h3>

<p>Every attempted operation distinguishes success, well-formed partial failure, and malformed use. Success yields its registered outputs. Well-formed partial failure yields reasoned NIL. Malformed use yields ERROR.</p>

<p>An implementation must not convert malformed use to NIL and must not raise ERROR merely because a registered partial projection has no value.</p>

<h3 id="lang-failure-project">LANG.FAILURE.PROJECT — Projection</h3>

<p>A projection condition is a semantic predicate over well-formed inputs. When it holds, the family produces NIL with the Word contract's reason and origin while retaining diagnosis and recoverability.</p>

<p>Division by zero, domain exclusion, unavailable hosted input, and space exhaustion remain distinct reasons where registered.</p>

<h3 id="lang-failure-error">LANG.FAILURE.ERROR — Errors</h3>

<p>Arity failure, nonconforming type or shape, malformed source, forbidden capability use, invalid dictionary operation, and exhausted hard execution controls raise their existing ERROR category.</p>

<p>ERROR terminates or propagates according to the existing execution contract and is never a truth value or stack NIL.</p>

<h3 id="lang-failure-passthrough">LANG.FAILURE.PASSTHROUGH — NIL passthrough</h3>

<p>For a passthrough family, an input NIL flows to the registered output position without changing reason, origin, recoverability, or diagnosis. Passthrough does not re-run a projection condition and does not relabel the absence.</p>

<p>A non-passthrough Word handles NIL only as explicitly stated by its family or Word contract.</p>

<h3 id="lang-failure-recovery">LANG.FAILURE.RECOVERY — Recovery</h3>

<p>Recovery Words may inspect or recover absence only through the published absence contract. Recovery does not erase the historical diagnosis from already emitted traces.</p>

<h2 id="lang-collections">7. Collections and Higher-order Evaluation</h2>

<h3 id="lang-collections-shape">LANG.COLLECTIONS.SHAPE — Shape rules</h3>

<p>Collection families validate scalar, vector, tensor, record, and broadcast shapes before element evaluation. An incompatible shape is ERROR unless a Word contract explicitly defines a partial projection.</p>

<p>Output shape is determined by the family's registered shape rule and not by storage representation.</p>

<h3 id="lang-collections-lift">LANG.COLLECTIONS.LIFT — Element lifting</h3>

<p>A lifted scalar family applies its scalar law element-wise after shape validation. Each lane preserves exactness, truth, NIL, and ERROR distinctions under the family's lifting policy.</p>

<p>Vectorization cannot turn an ERROR lane into NIL or collapse UNKNOWN into absence.</p>

<h3 id="lang-collections-higher">LANG.COLLECTIONS.HIGHER — Higher-order evaluation</h3>

<p>MAP, FILTER, FOLD, control Words, and other higher-order families evaluate CodeBlocks using their existing stack isolation, order, arity, and result-shape contracts.</p>

<p>Element visitation and effect order remain observable where a supplied block can emit output, mutate permitted dictionary state, or request hosted effects.</p>

<h3 id="lang-collections-budget">LANG.COLLECTIONS.BUDGET — Materialization</h3>

<p>Generative families honor the existing materialization ceiling. A well-formed request that cannot materialize within that ceiling yields recoverable NIL with <code>spaceExhausted</code>; malformed dimensions or shapes remain ERROR.</p>

<h2 id="lang-dictionary">8. Dictionary, Modules, and Effects</h2>

<h3 id="lang-dictionary-resolution">LANG.DICTIONARY.RESOLUTION — Deterministic lookup</h3>

<p>Name resolution is a deterministic function of the normalized name and current dictionary/import state. Core, User, and Module tiers retain their existing precedence, protection, dependency, and visibility rules.</p>

<p>LOOKUP, Hover, Reference, and execution must identify the same canonical entry.</p>

<h3 id="lang-dictionary-mutation">LANG.DICTIONARY.MUTATION — User Words</h3>

<p>Definition, replacement, deletion, recursion, identity, content addressing, import, export, and restoration retain their existing validation and observation contracts.</p>

<p>A dictionary mutation commits atomically or raises ERROR without a partial visible mutation.</p>

<h3 id="lang-dictionary-modules">LANG.DICTIONARY.MODULES — Modules</h3>

<p>Module catalogs distinguish availability from import state. Full and partial imports preserve module, public-word, selected-word, and sample state through HostProtocolV1 tuple order.</p>

<p>Import and unimport change resolution state; they do not rewrite source or canonical Word identity.</p>

<h3 id="lang-effects-hosted">LANG.EFFECTS.HOSTED — Hosted effects</h3>

<p>A hosted Word requests a declared capability and contributes an effect to an ordered effect sequence. The host supplies the response without redefining language-side validation, stack effects, or diagnosis.</p>

<p>NOW, CSPRNG, serial, standard I/O, audio, files, persistence, and other hosted capabilities retain their existing availability and failure contracts.</p>

<h3 id="lang-effects-child">LANG.EFFECTS.CHILD — Child runtime</h3>

<p>Child handles, states, spawn, await, status, kill, monitor, and supervise retain their observable lifecycle contracts. Scheduling is free only where stack, output, effect, handle, and diagnosis observations cannot distinguish it.</p>

<h2 id="lang-observation">9. Observation and Host Protocol</h2>

<h3 id="lang-observation-projections">LANG.OBSERVATION.PROJECTIONS — Observable surfaces</h3>

<p>Ajisai exposes Input, Output, Stack, and Dictionary projections. Presentation may tile or select them according to GUI Semantics, but their language-side contents are determined here.</p>

<p>Output is ordered text/effect observation; Stack is ordered typed values; Dictionary is resolved catalog and user/module state; Input is normalized source and host editing state where applicable.</p>

<h3 id="lang-observation-protocol">LANG.OBSERVATION.PROTOCOL — HostProtocolV1</h3>

<p>HostProtocolV1 fixes execute, step, reset, stack collection and snapshot, user/core/module metadata, lookup, catalog, import state, and the structured ExecuteResult, Value, semantics, absence, and diagnosis payloads.</p>

<p>Within V1 only optional fields may be added. Existing field deletion, rename, semantic change, and tuple reorder or reshape are forbidden. Breaking changes coexist as V2 while V1 remains usable.</p>

<h3 id="lang-observation-firewall">LANG.OBSERVATION.FIREWALL — Semantic firewall</h3>

<p>External consumers branch only on published protocol axes. They do not branch on Rust types, debug strings, display text, internal numeric/tensor form, or incidental GUI state.</p>

<p>The GUI never infers NIL versus UNKNOWN, exact equality, stack effect, resolution, diagnosis, or recoverability.</p>

<h3 id="lang-observation-diagnosis">LANG.OBSERVATION.DIAGNOSIS — Diagnosis</h3>

<p>Structured diagnosis includes when, where, why, summary, evidence, and next checks, plus registered optional evidence such as an agreed comparison prefix. Human wording may evolve only where machine-field meaning is preserved.</p>

<h2 id="lang-conformance">10. Conformance</h2>

<h3 id="lang-conformance-corpus">LANG.CONFORMANCE.CORPUS — Executable correspondence</h3>

<p>The conformance corpus links each case to one or more clause IDs and compares native, WASM, and Python observations. Required comparison includes stack, output, dictionary state, ordered effects, and structured diagnosis payloads.</p>

<p>A port is Ajisai only when it preserves the source-to-observation correspondence for the supported profile.</p>

<h3 id="lang-conformance-families">LANG.CONFORMANCE.FAMILIES — Family laws</h3>

<p>Every semantic family has law tests for arity, modifiers, NIL policy, projection, ERROR boundaries, shape, purity, determinism, capability, and effects as applicable. Every Word contract has at least one conformance path to its family and clause IDs.</p>

<h3 id="lang-conformance-gui">LANG.CONFORMANCE.GUI — GUI boundary</h3>

<p>GUI contract tests are blocking. They freeze DOM labels, keyboard operations, panel transitions, persistence formats, dictionary content, canonical and LaTeX rendering, and protocol-only semantic decisions without changing production GUI files during compaction phases 0 through 2.</p>

<h3 id="lang-conformance-change">LANG.CONFORMANCE.CHANGE — Change discipline</h3>

<p>A semantic change begins in one authoritative contract, regenerates all derived surfaces, updates clause-linked conformance cases, and demonstrates unchanged observations unless the change is explicitly versioned.</p>

<!-- INCLUDE:14-ai-first-implementation-rules -->

<!-- INCLUDE:15-test-discipline -->

<!-- INCLUDE:presentation-profile -->
