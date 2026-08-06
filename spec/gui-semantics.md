<h3 id="presentation-profile">Presentation Profile</h3>

<h4 id="gui-current-host-contract">Current GUI contract</h4>

<p>Presentation ranks below the language semantics: a conforming Ajisai need not present any surface visually, and a host with no display still defines the four observation projections. What follows is the contract the current Web and Tauri GUI realizes, stated concretely rather than as a formal system.</p>

<p>The GUI has four named surfaces: Input, Output, Stack, and Dictionary. Desktop presentation uses two columns (Input or Output on the left; Stack or Dictionary on the right), while mobile presentation exposes one selected surface at a time.</p>

<p>The operations Run, Step, Abort, and Reset retain their current behavior. The desktop shortcuts are respectively <code>Shift+Enter</code>, <code>Ctrl+Enter</code>, <code>Escape</code>, and <code>Ctrl+Alt+Enter</code>; Format is <code>Shift+Alt+F</code> and Clear stack is <code>Shift+Alt+C</code>. Clear stack discards the values and leaves the dictionary — that is what distinguishes it from Reset. Output copy and focus behavior, Core/User dictionary sheets, search, deletion, canonical stack display, opt-in LaTeX display, nested-vector bracket coloring, modifier background indication, stack snapshots, and persistence of user words are compatibility requirements.</p>

<p>The Stack surface reads in ordinary reading order: top to bottom, left to right, with the top of the stack last. The top of the stack is marked, so the value the next Word reads is identifiable without counting, at any number of values and whatever the wrapping.</p>

<p><strong>Host commands.</strong> A bare <code>RESET</code> or <code>CLEAR</code>, typed alone into the Input surface and run, is acted on by the host instead of being handed to the interpreter: <code>RESET</code> discards the session and <code>CLEAR</code> discards the stack. They are the typed spelling of controls that also exist as buttons and shortcuts, and they are deliberately <em>not</em> Words — the vocabulary holds nothing that throws away the values a program was handed or the dictionary it was defined in, and nothing in it should. The name must be the whole input, and a User Word of the same name takes precedence, so a host command can never shadow a definition.</p>

<p>Three visibility rules are normative, because each ties what is shown to what the user is doing rather than to geometry:</p>

<ol>
<li><strong>Every surface stays reachable.</strong> A hidden surface is concealed, never destroyed, and some finite sequence of operations exposes it again from any state.</li>
<li><strong>Editing and execution are observable.</strong> Any state in which source text can be entered shows Input, and every Run shows Stack, so the produced stack is surfaced.</li>
<li><strong>At least one surface is always visible.</strong> The user is never shown nothing.</li>
</ol>

<p>Everything else — column geometry, the breakpoint at which the layout switches, gesture thresholds, tap counts — is tuning, not semantics, and has the same standing as the execution step limit: a host control rather than a language constraint.</p>

<p>The GUI consumes the current host protocol and does not independently decide exact-real equality, Word stack effects, dictionary resolution, absence metadata, or numeric representation. Web and Tauri platform adapters may supply capabilities, but may not change language observations. Existing accessibility names, focus paths, keyboard operation, and panel transitions are part of this GUI contract.</p>
