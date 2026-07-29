<h3 id="presentation-profile">Presentation Profile</h3>

<h4 id="gui-current-host-contract">Current GUI contract</h4>

<p>The current Web and Tauri GUI is a frozen realization of this profile. It has four named surfaces: Input, Output, Stack, and Dictionary. Desktop presentation uses two columns (Input or Output on the left; Stack or Dictionary on the right), while mobile presentation exposes one selected surface at a time.</p>

<p>The operations Run, Step, Abort, and Reset retain their current behavior. The desktop shortcuts are respectively <code>Shift+Enter</code>, <code>Ctrl+Enter</code>, <code>Escape</code>, and <code>Ctrl+Alt+Enter</code>. Output copy and focus behavior, Core/User/Module dictionary sheets, search, deletion, import/export, canonical stack display, opt-in LaTeX display, nested-vector bracket coloring, modifier background indication, stack snapshots, and persistence of user words and import state are compatibility requirements.</p>

<p>The GUI consumes HostProtocolV1 and does not independently decide NIL versus Unknown, exact-real equality, Word stack effects, dictionary resolution, absence metadata, or numeric and tensor representation. Web and Tauri platform adapters may supply capabilities, but may not change language observations. Existing accessibility names, focus paths, keyboard operation, and panel transitions are part of this GUI contract.</p>

<p>The Core, Hosted, and Platform profiles classify <em>capabilities</em>. The Presentation Profile classifies <em>observation</em>: it governs how the four observation surfaces of Section 12.3 are revealed on a particular device. Like the Hosted and Platform profiles it ranks below the Core semantics — a conforming Ajisai need not present any surface visually — and a host with a display selects exactly one Presentation Profile.</p>

<p>A presentation profile is a <strong>labeled transition system</strong></p>

\[ M \;=\; (\, C,\; \Sigma,\; \rightarrow,\; c_0 \,) \]

<p>over the surface set \(A = \{\,\mathrm{Input},\ \mathrm{Output},\ \mathrm{Stack},\ \mathrm{Dictionary}\,\}\), where:</p>

<ul>
<li>a <strong>visibility configuration</strong> is the visible subset \(c \subseteq A\); the reachable configurations form \(C \subseteq \mathcal{P}(A)\);</li>
<li>\(\Sigma\) is the alphabet of abstract user operations (for example <code>show(a)</code>, <code>run</code>, <code>open-dictionary</code>, <code>advance</code>, <code>retreat</code>);</li>
<li>\(\rightarrow \,\subseteq\, C \times \Sigma \times C\) is the transition relation;</li>
<li>\(c_0 \in C\) is the initial configuration.</li>
</ul>

<p>The concrete \(C\), \(\Sigma\), initial configuration, and geometry are implementation freedom. The following <strong>invariants</strong> are normative; a presentation profile conforms if and only if it is a model of an LTS satisfying all of them.</p>

<ol>
<li><strong>Partition.</strong> For every \(c \in C\), each surface is either visible or hidden — \(c\) and \(A \setminus c\) partition \(A\). A hidden surface is concealed, never destroyed.</li>
<li><strong>Reachability.</strong> Every surface is exposable from everywhere: for each \(a \in A\) and each \(c \in C\) there is a finite operation sequence \(w \in \Sigma^{*}\) with \(c \xrightarrow{\,w\,} c'\) and \(a \in c'\). No surface can become permanently unreachable.</li>
<li><strong>Non-emptiness.</strong> Every reachable configuration shows at least one surface: \(c \neq \varnothing\) for all \(c \in C\). The user is never shown nothing.</li>
<li><strong>Determinism.</strong> \(\rightarrow\) is a partial function: \(c \xrightarrow{\sigma} c'\) and \(c \xrightarrow{\sigma} c''\) imply \(c' = c''\).</li>
<li><strong>Idempotent selection.</strong> Selecting an already-visible surface is a no-op: if \(a \in c\) then \(c \xrightarrow{\mathrm{show}(a)} c\).</li>
<li><strong>Semantic coupling.</strong> Three constraints tie visibility to intent rather than geometry:
  <ul>
  <li><em>Editing is observable</em> — any configuration in which source text can be entered has \(\mathrm{Input} \in c\).</li>
  <li><em>Execution is observable</em> — every <code>run</code> transition \(c \xrightarrow{\mathrm{run}} c'\) yields \(\mathrm{Stack} \in c'\), so the produced \((\textit{data}, \textit{role})\) stack of Section 12.3 is surfaced.</li>
  <li><em>Selection feeds editing</em> — any operation that inserts a selected dictionary word writes to the edit buffer \(B\), and the profile keeps the resulting \(\pi_{\mathrm{Input}}\) reachable (Invariant 2 applied to Input).</li>
  </ul>
</li>
</ol>

<p>Invariants 1–6 capture the behavior that defines the Ajisai editing experience while leaving every device-specific accident free. Whether surfaces are tiled in columns or shown one at a time, the breakpoint at which a layout switches, gesture thresholds, and tap counts are tuning, not semantics — the same standing as the execution step limit of Section 5.3, which is a runtime safety control rather than a language-semantic constraint. Two hosts satisfying Invariants 1–6 present the same language even when their screens differ.</p>

<p><strong>Degenerate profile.</strong> A host with no display realizes the empty presentation profile: \(M\) is trivial and no surface is ever tiled, yet the projections of Section 12.3 remain defined and are surfaced through host words instead of panels. This is why presentation is profile-grade rather than Core: the four projections are mandatory; the visibility LTS over them is host-conditional.</p>
