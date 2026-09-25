
<p class="doc-nav"><a href="https://github.com/masamoto1982/Ajisai#readme">README</a> · <a href="docs/index.html">Reference</a> · <a href="https://masamoto1982.github.io/Ajisai/">Playground</a></p>

<h1 id="ajisai-language-semantics">Ajisai Language Semantics</h1>

<p>Status: <strong>Canonical</strong><br>
Release stage: <strong>Alpha</strong>. No compatibility promise is in force.</p>

<p>
This document defines the correspondence between Ajisai source programs and observable values, states, effects, and diagnoses. It is a compact semantic kernel: differences between individual Words belong to the machine-readable vocabulary registry, not to parallel prose definitions.
</p>

<p>
Ajisai is built from ten concepts. Everything below is one of them, or a consequence of one of them.
</p>

<ol>
<li>Exact rational arithmetic, closed under square roots, with no rounding.</li>
<li>Three outcomes: a value, a reasoned absence, or an error.</li>
<li>A stack of values and vectors of values.</li>
<li>Code blocks, evaluated only when a Word asks for it.</li>
<li>One consumption rule: a Word consumes what it reads, and <code>BIND</code> names a value for reuse.</li>
<li>A two-tier dictionary — sealed Core, user-defined User — with content-addressed identity.</li>
<li>A machine-readable contract for every Word.</li>
<li>A pre-execution check of user declarations against those contracts.</li>
<li>One host protocol, which is the only way anything outside the language observes it.</li>
<li>An executable conformance corpus that decides whether an implementation is Ajisai.</li>
</ol>

<nav class="toc">
<h2>Contents</h2>
<ol>
<li><a href="#lang-authority">Authority and Compatibility</a></li>
<li><a href="#lang-source">Source and Desugaring</a></li>
<li><a href="#lang-values">Value Domains</a></li>
<li><a href="#lang-machine">Machine State and Evaluation</a></li>
<li><a href="#lang-stack">Stack and Consumption</a></li>
<li><a href="#lang-failure">Partiality and Failure</a></li>
<li><a href="#lang-collections">Collections and Higher-order Evaluation</a></li>
<li><a href="#lang-dictionary">Dictionary and Effects</a></li>
<li><a href="#lang-contract">Contracts and Static Checking</a></li>
<li><a href="#lang-observation">Observation and Host Protocol</a></li>
<li><a href="#lang-conformance">Conformance</a></li>
</ol>
</nav>

<h2 id="lang-authority">1. Authority and Compatibility</h2>

<h3 id="lang-authority-sources">LANG.AUTHORITY.SOURCES — Normative sources</h3>

<p>Nine sources are authoritative, each for a different part of the language:</p>

<div class="ref-table-wrap"><table class="ref-table"><thead><tr><th>Source</th><th>Authoritative for</th></tr></thead><tbody><tr><td>This Language Semantics</td><td>Program meaning</td></tr><tr><td><code>spec/grammar.json</code></td><td>The lexical grammar — the total correspondence from source text to a token sequence or one named source-error condition</td></tr><tr><td><code>spec/words.json</code></td><td>The vocabulary</td></tr><tr><td><code>spec/identity.json</code></td><td>When two things are the same — the one law, and what each level that decides it can deliver</td></tr><tr><td><code>spec/termination.json</code></td><td>Why every evaluation is finite — the recursion sites, the measure each one decreases, and the invariant the argument rests on</td></tr><tr><td><code>spec/outcomes.json</code></td><td>The outcome space — the closed list of NIL reasons and ERROR categories a contract's names resolve into, including those no contract reaches (<code>literal</code>) and those belonging to no single Word (arity, source, dictionary, resource)</td></tr><tr><td><code>spec/semantic-families.json</code></td><td>The laws Words share</td></tr><tr><td><code>spec/gui-semantics.md</code></td><td>Presentation</td></tr><tr><td><code>spec/host-protocol.schema.json</code></td><td>The boundary between them</td></tr></tbody></table></div>

<p><code>SPECIFICATION.html</code> renders the two prose sources — this document and <code>spec/gui-semantics.md</code> — and is not edited directly. The seven data sources are read directly by the implementation and its gates rather than through it: a source is authoritative because this table says so, not by appearing in that document.</p>

<p>
Neither implementation layout nor explanatory text can override an observable contract. A document that is not in the table above defines nothing; <code>docs/dev/</code> holds design notes and history on those terms.
</p>

<h3 id="lang-authority-present">LANG.AUTHORITY.PRESENT — Present-tense description</h3>

<p>
Ajisai has three reading surfaces: the <strong>README</strong>, the <strong>Reference</strong>, and this <strong>Specification</strong>. Each describes the language as it currently is. A definition, an example, or an explanation may rest only on concepts the language has, and every Word it names must exist in the vocabulary registry.
</p>

<p>
Superseded designs, migration history, and the reasoning behind a change are recorded outside these three surfaces, in notes that define nothing. A negative statement belongs on a reading surface only when it constrains an implementation — "an implementation must not convert malformed use to NIL" is such a constraint — and not when its only content is a contrast with a design the language does not have.
</p>

<h3 id="lang-authority-identity">LANG.AUTHORITY.IDENTITY — Language identity</h3>

<p>
Ajisai identity is the correspondence from normalized source to the ordered observation of stack, output, dictionary state, and structured diagnosis. Two implementations are semantically equivalent when that correspondence agrees for every conforming program.
</p>

<div class="ref-table-wrap"><table class="ref-table"><thead><tr><th>Count</th><th>What</th></tr></thead><tbody><tr><td>80</td><td>Canonical Words — the vocabulary (<code>docs/word-manifest.json</code> is the count of record)</td></tr><tr><td>7</td><td>Alias spellings — the 7 symbolic surface forms of those Words, none counted as vocabulary</td></tr><tr><td>49</td><td>Semantic Kernel Words, within the 80 — carry the language's semantic identity</td></tr><tr><td>31</td><td>Standard Words, within the 80 — carry its practical surface</td></tr></tbody></table></div>

<p>
Kernel and Standard are both ordinary Core Words in one flat dictionary, reached by their plain names, with contracts, laws, and conformance held to the same standard. Growth is not the goal: a proposed Word that is expressible as a user definition over the existing vocabulary does not belong in Core — unless expressing it that way costs asymptotically more than the same work done in the kernel, in which case what the definition demonstrates is a gap in the vocabulary rather than the absence of one.
</p>

<h3 id="lang-authority-freedom">LANG.AUTHORITY.FREEDOM — Implementation freedom</h3>

<div class="ref-table-wrap"><table class="ref-table"><thead><tr><th>Category</th><th>Examples</th></tr></thead><tbody><tr><td>Unobservable — free to change when all observations and host protocol payload meanings stay unchanged</td><td>AST, IR, dispatch, caching, storage layout, numeric representation, optimization</td></tr><tr><td>Not a semantic discriminant</td><td>Internal exact-real representation, Rust enum names, debug strings, allocation identity, GUI colors, private serialization</td></tr></tbody></table></div>

<p>In particular an implementation may execute a Word by any route it likes, provided the route is unobservable.</p>

<h2 id="lang-source">2. Source and Desugaring</h2>

<h3 id="lang-source-text">LANG.SOURCE.TEXT — Source domain</h3>

<p>
A program is Unicode text tokenized by the sealed Ajisai lexical grammar. Tokens distinguish literals, canonical Words, aliases, code blocks, vectors, definitions, and deletion.
</p>

<p>
A decimal literal carries at least one digit on both sides of the point: <code>0.5</code> and <code>5.0</code> are numbers, <code>.5</code> and <code>5.</code> are not. The point is therefore never the first or last character of a number, so a lone <code>.</code> is unambiguously a name rather than a truncated literal.
</p>

<p>
This numeric grammar is the single definition of what text denotes a number. A conversion from Text to a Scalar accepts exactly the lexemes a source literal accepts and projects everything else to NIL, so the language has one numeric grammar rather than one per entry point.
</p>

<p>
Whitespace is the <em>sole</em> token delimiter and is otherwise insignificant; a line break is whitespace like any other, so a program written across lines and the same program written flat are one program, with one Word identity (LANG.DICTIONARY.MUTATION). A line break ends a comment, which is the only thing it ends: <code>#</code> at the start of a word runs to the end of its line. No other character splits a token, the delimiters included: each of <code>[</code> and <code>]</code> must stand alone like every other word, so <code>[ 1 2 3 ]</code> is well-formed and <code>[1 2 3]</code> is a source error asking for the missing space, and <code>#</code> glued to a preceding lexeme is just part of that one name, not a comment. A quoted string is the one delimiter of its own: it may hold whitespace internally (<code>'hello world'</code>), but its closing <code>'</code> must itself be followed by whitespace or end of input to close.
</p>

<p>
Malformed delimiters, malformed literals, invalid names, and invalid definition forms are source errors. They do not denote NIL.
</p>

<h3 id="lang-source-normalize">LANG.SOURCE.NORMALIZE — Name normalization</h3>

<p>
Word lookup is case-insensitive through the canonical normalization. A symbolic alias resolves to exactly the same canonical Word contract and executor as its English name.
</p>

<p>
Normalization does not merge distinct value tags, invent dictionary entries, or change string and code-block contents.
</p>

<h3 id="lang-source-desugar">LANG.SOURCE.DESUGAR — Surface forms</h3>

<p>
Desugaring is deterministic and semantics-preserving. Aliases and the registered delimiter forms lower to canonical concepts before evaluation.
</p>

<p>
If \(D\) is desugaring and \(O\) observation, then \(O(p)=O(D(p))\) for every well-formed program \(p\). Sugar cannot introduce a new value domain or failure category.
</p>

<h3 id="lang-source-code">LANG.SOURCE.CODE — Code values</h3>

<p>
Code is a Vector holding source for later evaluation — not a distinct domain from data, but the same Vector domain (LANG.VALUES.DISJOINT) read as executable by a Word whose contract requests it. <code>[ ]</code> is the sole bracket, for both: <code>[ 1 2 + ]</code> is equally a data literal and, wherever a Word's contract requests it, executable code. A value's construction history is not part of it (LANG.VALUES.DENOTATION), so nothing about how a Vector came to exist marks it as "code" or "data" ahead of use. Evaluated as code, a Vector's elements run in order: a Symbol names a Word, and every other element pushes itself exactly as it is held — a NIL with its reason, a Record, an exact irrational — whether or not any source text denotes it.
</p>

<p>
Producing, storing, displaying, and evaluating a Vector as code are distinct operations. Quoted code is not eagerly executed: evaluation occurs only through a Word whose contract requests it — <code>EXEC</code> and the higher-order family — never merely by building or holding the value; <code>CONTRACT</code> reads a block without evaluating it. Branching is not among them: <code>SELECT</code> chooses between two values the program has already built, so a branch evaluates nothing and skips nothing. A bare name written where a Vector element is being collected denotes a Symbol (LANG.VALUES.VECTOR) — data until something executes it — so a name is not resolved merely by appearing inside a literal.
</p>

<h3 id="lang-source-frame">LANG.SOURCE.FRAME — What a block sees</h3>
<p>A block has no stack discipline of its own: the Word that evaluates it decides what the block reaches and what it may leave, and the block's text does not say which rule applies. The difference is whether a <code>+</code> written inside it finds two operands or none. Two rules cover every case.</p>

<div class="ref-table-wrap"><table class="ref-table"><thead><tr><th>Rule</th><th>Applies to</th><th>Frame holds</th><th>On leaving</th></tr></thead><tbody><tr><td>Whole-stack</td><td>A user Word's body (<code>DEF</code>) and <code>EXEC</code></td><td>The whole stack</td><td>Leaves whatever it pushes, however many values</td></tr><tr><td>Isolated frame</td><td><code>MAP</code> <code>FILTER</code> — the current element · <code>FOLD</code> <code>SCAN</code> — the accumulator and the current element</td><td>A fixed number of values (one, except <code>FOLD</code>'s two)</td><td>Must leave exactly one; leaving none is ERROR, and anything below the top goes with the frame</td></tr></tbody></table></div>
<p>A block written inside another block is data where it is written, evaluated only when the Word receiving it runs it, under that Word's rule and not the enclosing block's. This holds regardless of what name the block writes: a name naming the word being defined, reached only through such a nested block, is still a reference to that word for LANG.DICTIONARY.MUTATION's acyclicity rule to see, even though nothing here evaluates it.</p>
<p>A frame also holds <strong>local bindings</strong>. <code>BIND</code> consumes a value and a name and makes the name that value for the rest of the frame, however many times it is written; several names destructure a Vector of the same length. The isolation above is of the stack, not of names, so the blocks a Word evaluates read the frame they were written in — and a Word call does not: a body reads its own bindings and its operands and nothing of its caller's, so what a Word means never depends on where it is called. A binding ends with its frame, and a name is a Word or a binding and never both.</p>

<h2 id="lang-values">3. Value Domains</h2>

<h3 id="lang-values-disjoint">LANG.VALUES.DISJOINT — Tagged domains</h3>

<p>
Values form a disjoint tagged sum of exactly seven domains: Scalar, Boolean, String, Vector, Record, NIL, and Symbol. A Record is a keyed correspondence (LANG.RECORDS.STRUCTURE); it is not a Vector, a Vector of pairs is not a Record, and nothing converts between them implicitly. A Symbol is a bare name, data until something executes it (LANG.SOURCE.CODE); it is its own domain, reachable standalone (<code>[ ADD ] 0 GET</code> leaves the Symbol <code>ADD</code> on the stack) as well as nested inside a Vector. Two values are never equal merely because their encodings resemble one another: a Symbol is not the String of the same spelling.
</p>

<p>
In particular FALSE is not scalar zero, TRUE is not scalar one, and an absent value is not an ERROR.
</p>

<h3 id="lang-values-denotation">LANG.VALUES.DENOTATION — Identity is denotation</h3>

<p>A value is what it denotes, never how it was made. Two values are one value exactly when they denote the same thing, whatever operations produced each of them: \(\sqrt{8}\) and \(\sqrt{2}+\sqrt{2}\) are one value, and two NILs carrying one reason are one value. Construction history is not part of a value and cannot be read back out of one.</p>

<p>An internal representation is therefore unobservable, and a display is derived from the value rather than from the source that produced it. A procedure that decides identity answers about denotation, and where it cannot decide it answers that it cannot, never "different": comparison over the exact field decides (LANG.VALUES.EXACT), and the content identity of LANG.DICTIONARY.MUTATION decides only one direction. <code>spec/identity.json</code> states the law and what each level delivers.</p>

<h3 id="lang-values-exact">LANG.VALUES.EXACT — Exact scalars</h3>

<p>A scalar is an exact real. The domain is fixed by a condition rather than by a list: it is a field — closed under addition, subtraction, multiplication and division — whose equality and order an implementation decides in finite time through a normal form it exhibits. Comparison over this field is accordingly <strong>total</strong>: every comparison of two of its scalars yields TRUE or FALSE in finite time, not as a separate guarantee but as the half of the condition that fixes it.</p>

<p>The field \(\mathbb{Q}(\sqrt{d_1},\dots,\sqrt{d_k})\) generated over the rationals by square roots of non-negative rationals, in multiquadratic normal form \(\sum_d c_d\sqrt{d}\) with rational \(c_d\), is the witness that meets the condition. Integer, fraction, decimal, and scientific-notation literals are source forms for exact rationals, and <code>SQRT</code> generates the rest from a rational radicand: it is what builds the field rather than an operation the field is closed under. <code>POW</code> answers inside the field — an integer exponent is repeated multiplication or division, and an exponent <code>p/2</code> over a non-negative rational base is a power of its square root; every other exponent leaves the field and projects <code>domainMiss</code> — and <code>GCD</code> and <code>RATIO</code> read the integers and the reduced numerator and denominator that every rational carries. Arithmetic performs no intermediate rounding, and coefficients are arbitrary-precision, so a coefficient grows to whatever size the value requires. Rounding happens only where a program names it and only into text: <code>FORMAT</code> renders a scalar as decimal text with a stated number of digits, a tie rounding away from zero as <code>ROUND</code> rounds, and the field's decidable order settles every digit, an irrational's last one included. A real with no exhibited normal form is not an Ajisai value: widening the domain means exhibiting a new normal form that meets the condition, not amending the condition.</p>

<h3 id="lang-values-truth">LANG.VALUES.TRUTH — Three-valued truth</h3>

<p>
The Boolean domain has exactly three truth values: TRUE, FALSE, and UNKNOWN. TRUE and FALSE are the two Boolean data values; UNKNOWN is not a fourth variant but NIL (LANG.VALUES.NIL) read in truth position — a Word with nothing to decide from produces NIL, and NIL standing where a truth value is expected reads as UNKNOWN. <code>AND</code> and <code>NOT</code> compose all three values by the strong Kleene tables below, and so does every connective written from them — a disjunction is <code>a NOT b NOT AND NOT</code>; composition is not passthrough, so an absent operand yields UNKNOWN only where the other operand does not already settle the result by itself.
</p>
<div class="ref-table-wrap"><table class="ref-table"><thead><tr><th><code>AND</code></th><th>TRUE</th><th>FALSE</th><th>UNKNOWN</th></tr></thead><tbody><tr><td>TRUE</td><td>TRUE</td><td>FALSE</td><td>UNKNOWN</td></tr><tr><td>FALSE</td><td>FALSE</td><td>FALSE</td><td>FALSE</td></tr><tr><td>UNKNOWN</td><td>UNKNOWN</td><td>FALSE</td><td>UNKNOWN</td></tr></tbody></table></div>
<div class="ref-table-wrap"><table class="ref-table"><thead><tr><th><code>NOT</code></th><th>TRUE</th><th>FALSE</th><th>UNKNOWN</th></tr></thead><tbody><tr><td></td><td>FALSE</td><td>TRUE</td><td>UNKNOWN</td></tr></tbody></table></div>

<p>
FALSE dominates <code>AND</code> even against an UNKNOWN operand, decided by the definite operand alone; where neither operand settles it, the output is UNKNOWN and carries whichever operand's absence reason applies — the left operand's, when both are absent (LANG.FAILURE.PASSTHROUGH's left-to-right rule). UNKNOWN enters the truth domain one way: through an absent operand read in truth position. Every comparison over the exact domain decides in finite time and yields TRUE or FALSE: totality is a domain property of the field (LANG.VALUES.EXACT), not a limit this clause imposes. Misuse still lives outside this domain: an operation that is malformed raises ERROR, which is never a truth value. The host protocol observes TRUE and FALSE as <code>boolean</code> nodes carrying <code>semantics.truthValue</code> <code>"true"</code> or <code>"false"</code>, and UNKNOWN as the NIL it is — a <code>nil</code> node carrying its absence reason (LANG.OBSERVATION.PROTOCOL); a consumer reads these rather than display text (LANG.OBSERVATION.FIREWALL).
Being read in truth position adds an observation; it takes none away. UNKNOWN is still the NIL it is: <code>NIL?</code> answers TRUE for it, <code>NIL-REASON</code> reports the reason it arrived with, a fallback can be chosen in its place, and a passthrough Word passes it on — so <code>1 0 DIV TRUE AND</code> is an UNKNOWN whose reason is still <code>divisionByZero</code>. An implementation that reports UNKNOWN as present, or that drops its reason, contradicts LANG.VALUES.NIL.
</p>

<h3 id="lang-values-nil">LANG.VALUES.NIL — Diagnostic absence</h3>

<p>
NIL is a value representing absence from a well-formed partial operation. It carries a <strong>reason</strong>: a stable, machine-readable identifier for why production failed. The reason is observable through <code>NIL-REASON</code> and through the protocol. The reason space has two layers: the closed set of identifiers <code>spec/outcomes.json</code> registers, and one of them, <code>userDeclared</code>, which a program reaches by <code>ABSENT</code> and which carries the Text the program gave as its parameter — that Text is what <code>NIL-REASON</code> answers for it.
</p>

<p>The reason is the entire observable content of a NIL, the declared Text of a <code>userDeclared</code> NIL included; so two <code>ABSENT</code> NILs are the same value exactly when their Texts are equal. An implementation may emit richer diagnostics on the host channel, and no program behavior may depend on them.</p>

<h3 id="lang-values-vector">LANG.VALUES.VECTOR — Vectors</h3>

<p>
A Vector is an ordered finite collection of values. Indexing is 0-origin and negative indices count from the end. Vectors nest, and nesting expresses ragged and grouped data.
</p>

<p>
Vector length is semantic even when storage is flattened, shared, or lazily materialized. Order and length are a Vector's whole observable structure, and a nested Vector is an element like any other.
</p>

<p>
Inside a Vector literal a name denotes a Symbol (LANG.VALUES.DISJOINT): data until something executes it, and building the literal is not itself execution, so no dictionary lookup occurs there. <code>[ FOO ]</code> is the one-element Vector holding the Symbol <code>FOO</code> whether or not <code>FOO</code> is a defined Word, so a Vector literal denotes the same value under every dictionary state. Only <code>TRUE</code>, <code>FALSE</code> and <code>NIL</code> denote values rather than a Symbol carrying their name. A consequence worth stating: a misspelled name inside a Vector literal is a Symbol element, not an error.
</p>

<h3 id="lang-records-structure">LANG.RECORDS.STRUCTURE — Records</h3>

<p>A Record is a keyed correspondence: a sequence of distinct keys, each any value, paired position by position with a sequence of values. Those two sequences are its whole observable structure — <code>KEYS</code> and <code>VALUES</code> read them back, aligned — so two Records are one value exactly when their key sequences and their value sequences are (LANG.VALUES.DENOTATION), and key order is observable: <code>WITH</code> replaces a present key in place and appends an absent one, and <code>MERGE</code> keeps the left operand's order, takes the right operand's value where both hold a key, and appends the right's remaining keys. <code>RECORD</code> builds one from a Vector of keys and a Vector of values, and raises two ERRORs rather than making a choice for the program — a key left without a value under it is the length mismatch between the two sequences, and a repeated key is <code>duplicateKey</code>. Reading or removing an absent key (<code>AT</code>, <code>WITHOUT</code>) projects <code>notFound</code>; <code>HAS?</code> asks presence alone, so a stored NIL is told apart from an absent key. A Record in a <code>leaf</code> or <code>truth</code> operand lifts the Word over its values, keys kept (LANG.COLLECTIONS.LIFT); in a <code>data</code> operand of a Word that reads a Vector it raises the operand ERROR the contract declares, and a Record Word given anything else raises <code>nonRecord</code>. <code>TALLY</code> and <code>GROUP</code> answer Records, keyed by the distinct elements and the keys they bundle by. <code>JSON-DECODE</code> reads JSON text into these domains — an object is a Record, an array a Vector, a number the exact rational it spells, <code>null</code> a NIL — projecting <code>invalidEncoding</code> for text that is not one JSON value and <code>spaceExhausted</code> for nesting past the machine's bound; <code>JSON-ENCODE</code> writes the inverse, spelling a rational with no finite decimal as its lexeme inside a string rather than rounding it, and projects <code>domainMiss</code> for a value with no JSON image.</p>

<h2 id="lang-machine">4. Machine State and Evaluation</h2>

<h3 id="lang-machine-state">LANG.MACHINE.STATE — State</h3>

<p>
A machine state contains the data stack, the dictionary, the output stream, and the execution controls needed by observable contracts.
</p>

<p>
Host-only caches, allocation arenas, compiled plans, and counters are not semantic state.
</p>

<h3 id="lang-machine-transformers">LANG.MACHINE.TRANSFORMERS — Programs</h3>

<p>Each executable token denotes a partial state transformer. A program denotes left-to-right composition of those transformers after desugaring and name resolution.</p>

<p>Execution is deterministic relative to the initial state. Optimization may reassociate internal work only when the observable sequence is unchanged.</p>

<h3 id="lang-machine-word-contract">LANG.MACHINE.WORD — Word contracts</h3>

<p>A canonical Word contract selects a semantic family and supplies its differences: stack arity, NIL policy, projection condition and reason, error conditions, purity, determinism, effects, clause links, documentation, and executor key. Determinism classifies what the result is relative to: <em>deterministic</em> from operands alone, <em>state-relative</em> when the wider stack, frame bindings, or dictionary also decide it (LANG.MACHINE.STATE) — a pure Word can still be state-relative, since purity (LANG.EFFECTS.OUTPUT) asks only whether the same stack and dictionary always yield the same result — or <em>host-relative</em> when the host's own rendering, capture, or discard of the effect also decides it (LANG.EFFECTS.OUTPUT).</p>

<p>The executor must refine its contract. Aliases and documentation are projections of the same canonical entry, not independent semantic authorities.</p>

<h3 id="lang-machine-order">LANG.MACHINE.ORDER — Evaluation order</h3>

<p>Token evaluation, output emission, and dictionary mutation preserve their observable order.</p>

<h3 id="lang-machine-limits">LANG.MACHINE.LIMITS — Work limits</h3>

<p>A host bounds a run along several axes, and what distinguishes them is not which resource they meter but which of the three outcomes (LANG.FAILURE.TRICHOTOMY) exhausting one produces:</p>

<div class="ref-table-wrap"><table class="ref-table"><thead><tr><th>Ceiling</th><th>Bounds</th><th>On exhaustion</th></tr></thead><tbody><tr><td>Execution-step limit</td><td>Total work across the run</td><td>ERROR, category <code>executionLimitExceeded</code></td></tr><tr><td>Materialization ceiling</td><td>Size of one generated collection</td><td>NIL, reason <code>spaceExhausted</code>, for an otherwise well-formed request</td></tr><tr><td>Value and work ceilings</td><td>One value or input (source bytes, digits in a numeric literal, integer width, algebraic term count), or the numeric and collection work the run has accumulated</td><td>ERROR, category <code>resourceLimitExceeded</code></td></tr></tbody></table></div>

<p>The middle row differs in kind: a collection too large to build is a well-formed request the host declines, so it projects (LANG.FAILURE.PROJECT), while the other rows are the host refusing to continue at all. That split and each row's category are normative; how many ceilings a host divides the last row into, and every numeric value, is implementation freedom. These are safety controls, not semantic constraints — termination already follows from LANG.DICTIONARY.ACYCLIC, so they bound cost rather than decide it.</p>

<h2 id="lang-stack">5. Stack and Consumption</h2>

<h3 id="lang-stack-order">LANG.STACK.ORDER — Stack observation</h3>

<p>The stack is an ordered sequence with a distinguished top. A Word takes its operands from the top and returns its results to the top.</p>

<p>A display transformation cannot reorder, coerce, drop, or invent stack values.</p>

<h3 id="lang-stack-consumption">LANG.STACK.CONSUMPTION — Consumption</h3>

<p>A Word consumes the operands it reads: they leave the stack, and its results take their place. Nothing modifies this, and a Word whose result is empty is no exception — <code>BIND</code>, <code>DEF</code> and <code>DEL</code> consume their operands too. A value used more than once is named with <code>BIND</code> and read by that name as often as it is needed: <code>5 'N' BIND N N 1 +</code> leaves <code>5 6</code>. Because every call consumes exactly what it reads, writing a User Word's body in place of the Word never changes which operands are consumed, which is what lets a program be expanded into Core Words alone.</p>

<p>A Word selects operands from the top of the stack, validates its registered contract, computes or projects the result, and then consumes its operands. ERROR does not masquerade as a successful NIL projection.</p>

<h2 id="lang-failure">6. Partiality and Failure</h2>

<h3 id="lang-failure-trichotomy">LANG.FAILURE.TRICHOTOMY — Value, absence, misuse</h3>

<p>Every attempted operation ends in exactly one of three categories:</p>

<div class="ref-table-wrap"><table class="ref-table"><thead><tr><th>Category</th><th>Outcome</th></tr></thead><tbody><tr><td>Success</td><td>Its registered outputs</td></tr><tr><td>Well-formed partial failure</td><td>NIL with a reason</td></tr><tr><td>Malformed use</td><td>ERROR</td></tr></tbody></table></div>

<p>An implementation must not convert malformed use to NIL and must not raise ERROR merely because a registered partial projection has no value. Recovery operates on absence alone: a program can choose a fallback in place of a NIL, while an ERROR propagates and halts evaluation. A program declares either outcome itself: <code>ABSENT</code> makes a NIL whose reason is the Text it is given, and <code>FAIL</code> raises an ERROR (category <code>declaredFailure</code>) whose message is the Text it is given. A declared ERROR is an ERROR in full — no Word catches it — so the trichotomy is closed against the program as well as against the implementation.</p>

<h3 id="lang-failure-project">LANG.FAILURE.PROJECT — Projection</h3>

<p>A projection condition is a semantic predicate over well-formed inputs. When it holds, the Word produces NIL with the reason its contract registers.</p>

<p>Division by zero, domain exclusion, out-of-range indexing, a value not found where one is sought, failed parsing, and space exhaustion are among the reasons a projection condition keeps distinct — <code>spec/outcomes.json</code> (LANG.AUTHORITY.SOURCES) is the exhaustive list, this clause only fixes how the shared ones behave. A Word's own contract names the reasons that Word projects for, which is a subset: a reason exists in the outcome space whether or not any contract currently reaches it.</p>

<h3 id="lang-failure-error">LANG.FAILURE.ERROR — Errors</h3>

<p>Arity failure, nonconforming type, malformed source, invalid dictionary operation, and an exhausted execution-step limit raise their registered ERROR category.</p>

<p>ERROR halts evaluation and propagates. It is never a truth value and never a stack value.</p>

<h3 id="lang-failure-passthrough">LANG.FAILURE.PASSTHROUGH — NIL passthrough</h3>

<p>What a Word does with a NIL operand follows from what it does with that operand, and from nothing else. Each operand position has one role, declared per Word as <code>stack.operands</code> in <code>spec/words.json</code>. A <strong>data</strong> or <strong>leaf</strong> operand is read (a leaf as one Scalar, String or Boolean, lifted over a container, LANG.COLLECTIONS.LIFT): an absent one is the result, flowing to the output position without changing its reason; the primitive does not run, no projection condition is re-run or relabels it, and when several data operands are absent the leftmost is the result. An <strong>element</strong> is carried without being read (a value stored, bound, printed, encoded or inspected, an accumulator, a needle compared by equality): a NIL there is an ordinary value (LANG.VALUES.NIL). A <strong>control</strong> operand directs the Word rather than being data it transforms — a block it evaluates, a name it binds, defines, deletes or looks up, or a message it raises: it cannot be absent, so a NIL there is malformed use, raising the condition the Word declares for any other operand that does not belong there, and this is decided before any data operand passes through. A <strong>truth</strong> operand reads a NIL as UNKNOWN (LANG.VALUES.TRUTH).</p>

<p>A Word's <code>nilPolicy</code> summarises its roles and is never a choice of its own, so two Words that treat an operand alike treat a NIL there alike. A Word of data-dependent arity declares no roles and states its NIL handling in its contract.</p>

<h3 id="lang-failure-recovery">LANG.FAILURE.RECOVERY — Recovery</h3>

<p>Recovery is a phrase, not a form of its own. <code>NIL?</code> consumes a value and answers whether it is absent, which is what <code>SELECT</code> reads as its truth operand, so a subject named once and read twice chooses between itself and a fallback: <code>subject 'S' BIND fallback S S NIL? SELECT</code> leaves the subject when it is present and the fallback when it is not, the fallback an ordinary operand computed before the choice like every other operand. The question is asked of the whole value: a Vector holding an absent lane is present, so a lane recovered inside a Vector is recovered there rather than around it.</p>

<p>Recovery does not erase absence from already emitted output.</p>

<h2 id="lang-collections">7. Collections and Higher-order Evaluation</h2>

<h3 id="lang-collections-lift">LANG.COLLECTIONS.LIFT — Element lifting</h3>

<p>A Word applies element-wise wherever it reads an operand as one value: a <code>leaf</code> operand, read as one Scalar, String or Boolean, and a <code>truth</code> operand (LANG.FAILURE.PASSTHROUGH). Given a Vector there, it answers a Vector of its answers for the elements; given a Record, a Record of its answers under the unchanged keys (LANG.RECORDS.STRUCTURE). Every Word lifts by this one rule, arithmetic, comparison, logic and text alike, and a one-element Vector is a Vector there, never its element. A scalar combines with every element of a vector, however that vector is nested, including a ragged one. A Record lifts first: any other operand combines with each of its values, two Records combine value by value when their key sequences are equal, and two whose key sequences differ are a <code>shapeMismatch</code> ERROR.</p>

<p>Two vectors combine by pairing their axes. A vector whose nesting is rectangular has a shape: the lengths of its axes, outermost first. The two shapes are aligned at their innermost axis, and an axis the shorter shape does not reach counts as length 1. Paired axes combine when their lengths are equal, and when one of them is 1 that operand's single lane is reused across the other's length; the result carries the longer length on that axis. This makes a one-element vector combine with a vector of any length, and <code>[ 1 2 3 ] [ 10 ] *</code> is <code>[ 10/1 20/1 30/1 ]</code>. Any other pairing is ERROR, and so is any pairing of two vectors where either one is ragged. A program reads this shape with <code>SHAPE</code>, which answers a rectangular vector's axis lengths and, for a ragged one, the reasoned absence <code>domainMiss</code> — a ragged vector has no shape; <code>RESHAPE</code> regroups a vector's leaves, in order, under a shape whose product is their count; <code>FLATTEN</code> collapses every axis into one; <code>DEPTH</code> answers how deeply a value nests, a leaf being 0. The last two cannot be written as user definitions: nesting depth is not known in advance, and a language with no recursion and no unbounded loop cannot walk a structure of unknown depth (LANG.DICTIONARY.ACYCLIC).</p>

<p>Each lane preserves the exactness, truth, NIL, and ERROR distinctions of the scalar law. Vectorization cannot turn an ERROR lane into NIL.</p>

<h3 id="lang-collections-higher">LANG.COLLECTIONS.HIGHER — Higher-order evaluation</h3>

<p><code>MAP</code>, <code>FILTER</code>, <code>FOLD</code>, and <code>SCAN</code> evaluate their code operand (LANG.SOURCE.CODE) once per visited element, in index order, with the block's stack effect isolated to its own operands. A block reaches an inner axis by nesting: <code>[ [ f ] MAP ] MAP</code> applies <code>f</code> one level down.</p>

<p><code>FOLD</code> and <code>SCAN</code> are one walk over the elements, carrying an accumulator the block rewrites at each one; they differ in which accumulators the walk answers with. <code>FOLD</code> answers the last, so a walk with nothing to visit answers the seed it was given. <code>SCAN</code> answers every one of them, one per visited element and the seed not among them, so a walk with nothing to visit answers no elements and an absent collection answers that same absence. This is the one shape a computation carrying state from one element to the next can take, because a Word cannot call itself (LANG.DICTIONARY.ACYCLIC) and there is no unbounded loop.</p>

<p>Element visitation order is observable where a supplied block can emit output or mutate dictionary state. The block's result is what it leaves on top when it finishes, taken as it stands, whatever domain it is in: a Vector of one element is a Vector of one element (LANG.VALUES.DISJOINT), so <code>[ 1 2 ] [ 1 COLLECT ] MAP</code> answers <code>[ [ 1 ] [ 2 ] ]</code> and no Word unwraps a result on the grounds of its length. A block that finishes having left nothing raises the <code>blockContractViolation</code> its caller's contract registers; leaving more than the result is ordinary and the rest is discarded, because a block may push a bound name or a value it computed along the way, and only its top is the result.</p>

<h3 id="lang-collections-budget">LANG.COLLECTIONS.BUDGET — Materialization</h3>

<p><code>RANGE</code> and <code>FILL</code> honor the materialization ceiling. A well-formed request that cannot materialize within it yields NIL with reason <code>spaceExhausted</code>; malformed dimensions remain ERROR.</p>

<h2 id="lang-dictionary">8. Dictionary and Effects</h2>

<h3 id="lang-dictionary-resolution">LANG.DICTIONARY.RESOLUTION — Deterministic lookup</h3>

<p>The dictionary has two tiers, and those two are the whole of it. <strong>Core</strong> holds the 80 canonical Words and is sealed: a Core name cannot be redefined or deleted. <strong>User</strong> holds definitions made by <code>DEF</code>. Resolution is a deterministic function of the normalized name and the current dictionary, and User never shadows Core. The host's lookup, hover, the Reference, and execution must identify the same canonical entry.</p>

<h3 id="lang-dictionary-mutation">LANG.DICTIONARY.MUTATION — User Words</h3>

<p><code>DEF</code> binds a name to a code block; <code>DEL</code> removes a User Word and fails with ERROR if any other definition still depends on it. A dictionary mutation commits atomically or raises ERROR with no partial visible mutation.</p>

<p>Every Word has a <strong>content identity</strong>: a digest over its normalized definition and the identities of the Words it calls, so a change to a dependency changes the identity of everything that depends on it, and what a Word or its dependencies are named does not reach it. Equal identities mean one Word, which is what lets a host deduplicate by content; unequal identities mean nothing, since two definitions can denote one function and normalize differently (LANG.VALUES.DENOTATION). <code>DIGEST</code> answers that identity for a Symbol naming a Word, and for any other value the digest of its denotation, under the same asymmetry.</p>

<h3 id="lang-dictionary-acyclic">LANG.DICTIONARY.ACYCLIC — No definition may name itself</h3>

<p>The User dictionary's reference graph is acyclic: <code>DEF</code> raises ERROR rather than commit a definition that names the word being defined, directly or through a chain of other User words, however that name is reached (LANG.SOURCE.FRAME). That check reads the Symbols a body writes, and it is complete because a Word runs only when a Symbol names it: no Word turns text into a Symbol, and a String is not code (LANG.SOURCE.CODE), so a call can never be computed. No Word can call itself, so repetition is only the bounded higher-order Words (<code>MAP</code>, <code>FILTER</code>, <code>FOLD</code>, <code>SCAN</code>) over an already-finite Vector, and every evaluation is structurally finite: termination follows from the dictionary's shape alone, and the execution-step ceiling (LANG.MACHINE.LIMITS) is left to bound cost, not decide it. Ajisai is therefore total and not Turing-complete — a deliberate trade, whose price is that a computation which must run until it converges has to be re-expressed over a finite Vector the program builds first, and whose return is that the language denotes without domain theory: no divergence, so no bottom, no continuity obligation and no fixed-point construction. <code>spec/termination.json</code> carries the argument in full.</p>

<h3 id="lang-effects-output">LANG.EFFECTS.OUTPUT — Output</h3>

<p>Output is the only effect that leaves the machine. <code>PRINT</code> consumes its operand and appends it to the ordered output stream. No other Word emits output, so the output stream is the whole of what a host observes a program doing.</p>

<p>A host may render, capture, or discard the output stream, but may not reorder it or change language-side validation.</p>

<p>A Word may have exactly one other effect, and it stays inside the machine: <code>DEF</code> and <code>DEL</code> change the dictionary, under LANG.DICTIONARY.MUTATION. Output emission and dictionary mutation are therefore the two effects, and LANG.MACHINE.ORDER orders both of them against token evaluation.</p>

<p>Every other Word changes nothing: given the same stack and dictionary it produces the same result. A Word that evaluates a supplied code block has the effects of that block and no others, so it is pure exactly when the block is.</p>

<h2 id="lang-contract">9. Contracts and Static Checking</h2>

<h3 id="lang-contract-registry">LANG.CONTRACT.REGISTRY — Machine-readable contracts</h3>

<p>Every Core Word's contract is a machine-readable record in <code>spec/words.json</code>, conforming to <code>spec/words.schema.json</code>. The record is the single place a Word's arity, NIL policy, projection reason, error conditions, purity, and documentation are stated.</p>

<p>Prose that restates a contract is a projection of that record and carries no independent authority. <code>CONTRACT</code> answers the record from inside the language, as a Record keyed by those fields, for a Symbol naming a Core Word; for a User Word, or for a block of code, it answers the inferred contract of LANG.CONTRACT.CHECK in one shape, and for a Symbol naming nothing it projects <code>notFound</code>.</p>

<h3 id="lang-contract-check">LANG.CONTRACT.CHECK — Pre-execution check</h3>

<p>A user definition may carry a declaration of its own arity, purity, and NIL behavior. <code>ajisai check --contract</code> verifies that declaration against the Core contracts of the Words it calls, <strong>without running the program</strong>. <code>CONTRACT</code> reaches the same inference from inside the language, over a Vector of code (LANG.SOURCE.CODE) or the name of a User Word: it never evaluates its operand, so calling it carries none of the operand's own effects. Its answer is a Record whose <code>confidence</code> and <code>gaps</code> carry the three outcomes below as data.</p>

<p>The check is deliberately <strong>conservative and partial</strong>. It reports exactly three outcomes per declaration, each the trichotomy of LANG.FAILURE.TRICHOTOMY applied at check time rather than at run time:</p>

<div class="ref-table-wrap"><table class="ref-table"><thead><tr><th>Check-time outcome</th><th>Run-time counterpart</th></tr></thead><tbody><tr><td><em>Verified</em></td><td>A value — the inferred contract itself</td></tr><tr><td><em>Cannot verify</em></td><td>A reasoned absence</td></tr><tr><td><em>Violated</em></td><td>An error</td></tr></tbody></table></div>

<p>Anything outside the syntactic fragment the inference analyzes — a higher-order body whose block is not statically known, or dynamic control — is reported as <em>cannot verify</em> and is never silently passed. A tool that reports <em>verified</em> for an unanalyzable body is nonconforming.</p>
<p>The correspondence classifies outcomes, not mechanisms, and does not make the check evaluate the program: division by zero, a failed parse and an out-of-range index already share one outcome category while sharing no mechanism, and an inference that could not decide joins that list on the same terms.</p>

<h2 id="lang-observation">10. Observation and Host Protocol</h2>

<h3 id="lang-observation-projections">LANG.OBSERVATION.PROJECTIONS — Observable surfaces</h3>

<p>Ajisai exposes four projections: Input, Output, Stack, and Dictionary. Presentation may tile or select them according to the Presentation Profile, but their language-side contents are determined here.</p>

<div class="ref-table-wrap"><table class="ref-table"><thead><tr><th>Projection</th><th>Content</th></tr></thead><tbody><tr><td>Output</td><td>The ordered text observation</td></tr><tr><td>Stack</td><td>The ordered typed values</td></tr><tr><td>Dictionary</td><td>The resolved Core and User catalog</td></tr><tr><td>Input</td><td>Normalized source and host editing state, where applicable</td></tr></tbody></table></div>

<h3 id="lang-observation-protocol">LANG.OBSERVATION.PROTOCOL — The host protocol</h3>

<p>One recursive node shape observes every stack value, rendered by a single serializer shared across hosts (<code>spec/host-protocol.schema.json</code>): a <code>type</code> naming the value's domain (LANG.VALUES.DISJOINT's seven domains as seven wire types — <code>nil</code>, <code>boolean</code>, <code>number</code>, <code>string</code>, <code>vector</code>, <code>symbol</code>, and <code>record</code>, the last carrying its aligned key and value sequences as two arrays of nodes), a <code>value</code> shaped by that type, and an optional <code>semantics</code> bag carrying a Boolean's truth, a NIL's absence reason, and exact-irrational markers. Every field is derived from the value itself, never from the Word that produced it (LANG.VALUES.DENOTATION). The CLI/MCP agent surface wraps an array of these nodes in its own JSON envelope (<code>docs/dev/agent-cli-output-contract.md</code>, non-canonical); the WASM/GUI boundary exposes the same nodes directly from its own methods, with no enclosing envelope of its own. It is the only channel through which anything outside the language observes the stack.</p>

<p>No version field exists on this node shape today: the CLI envelope around it carries its own plain integer <code>schemaVersion</code>, and the WASM boundary carries no version signal at all, so a breaking change to the shape this clause fixes would be silent until one is added. Existing field deletion, rename, semantic change, and tuple reorder or reshape remain forbidden regardless: an implementation offers exactly one protocol.</p>

<h3 id="lang-observation-firewall">LANG.OBSERVATION.FIREWALL — Semantic firewall</h3>

<p>External consumers branch only on published protocol axes. They do not branch on Rust types, debug strings, display text, internal numeric form, or incidental GUI state.</p>

<p>The GUI never infers exact equality, stack effect, resolution, or absence reasons on its own.</p>

<h3 id="lang-observation-diagnosis">LANG.OBSERVATION.DIAGNOSIS — Diagnosis</h3>

<p>An ERROR carries a stable category identifier and a human-readable message; a NIL carries its reason. Those identifiers are the machine-readable surface, and human wording may evolve only where they are preserved.</p>

<h2 id="lang-conformance">11. Conformance</h2>

<h3 id="lang-conformance-corpus">LANG.CONFORMANCE.CORPUS — Executable correspondence</h3>

<p>The conformance corpus links each case to one or more clause IDs and compares stack, output, dictionary state, and diagnosis identifiers. An implementation is Ajisai when it preserves the source-to-observation correspondence for every case.</p>

<p>The corpus is the decision procedure for the question "is this Ajisai?". No prose answers it.</p>

<h3 id="lang-conformance-families">LANG.CONFORMANCE.FAMILIES — Family laws</h3>

<p>Every semantic family has law tests for arity, consumption, NIL policy, projection, ERROR boundaries, lifting, purity, and effects as applicable. Every Word contract has at least one conformance path to its family and clause IDs.</p>

<h3 id="lang-conformance-change">LANG.CONFORMANCE.CHANGE — Change discipline</h3>

<p>A semantic change begins in one authoritative source, regenerates all derived surfaces, updates clause-linked conformance cases, and demonstrates unchanged observations unless the change is explicitly versioned.</p>

<!-- INCLUDE:presentation-profile -->
