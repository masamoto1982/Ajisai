<h3 id="presentation-profile">Presentation Profile</h3>

<h4 id="gui-current-host-contract">Current GUI contract</h4>

<p>Presentation ranks below the language semantics: a conforming Ajisai need not present any surface visually, and a host with no display still defines the four observation projections. What follows is the contract the current Web and Tauri GUI realizes, stated concretely rather than as a formal system.</p>

<p>The GUI has four named surfaces: Input, Output, Stack, and Dictionary. Desktop presentation uses two columns (Input or Output on the left; Stack or Dictionary on the right), while mobile presentation exposes one selected surface at a time.</p>

<p>The operations Run, Step, Abort, and Reset retain their current behavior. The desktop shortcuts are respectively <code>Shift+Enter</code>, <code>Ctrl+Enter</code>, <code>Escape</code>, and <code>Ctrl+Alt+Enter</code>; Format is <code>Shift+Alt+F</code> and Clear stack is <code>Shift+Alt+C</code>. Clear stack discards the values and leaves the dictionary — that is what distinguishes it from Reset. Output copy and focus behavior, Core/User dictionary sheets, search, deletion, canonical stack display, opt-in LaTeX display, nested-vector bracket coloring, modifier background indication, stack snapshots, and persistence of user words are compatibility requirements.</p>

<p>The Stack surface reads in ordinary reading order: top to bottom, left to right, with the top of the stack last. The top of the stack is marked, so the value the next Word reads is identifiable without counting, at any number of values and whatever the wrapping.</p>

<p><strong>Host commands.</strong> Some input typed into the Input surface and run is acted on by the host instead of being handed to the interpreter. A bare <code>RESET</code> discards the session and a bare <code>CLEAR</code> discards the stack; they are the typed spelling of controls that also exist as buttons and shortcuts, and they are deliberately <em>not</em> Words — the vocabulary holds nothing that throws away the values a program was handed or the dictionary it was defined in, and nothing in it should.</p>

<p><strong>Looking a Word up</strong> is the third, written <code>'ADD' ?</code> or <code>'ADD' LOOKUP</code>, in either the quoted form or a bare name. The host resolves the name against the active dictionary and answers without running anything: a Core Word's reference text is shown in the Output surface, where it is read, and a User Word's reconstructed <code>DEF</code> source is loaded into the Input surface, where the point of looking it up is to edit it and define it again. A name the dictionary does not hold is reported as an unknown word. This too is deliberately not a Word: reference prose is not a value, no program can do anything with the answer, and a Word whose entire result reaches the editor rather than the stack would be claiming otherwise. The lookup observes and changes nothing — the stack, the dictionary, and the output are the same afterwards — and it must identify the same canonical entry that hover, the Reference, and execution do.</p>

<p>A host command must be the whole input, and a User Word of its name takes precedence, so a host command can never shadow a definition. For a lookup the name that must not be shadowed is the mark, <code>?</code> or <code>LOOKUP</code>, and not the name being looked up: <code>'CLEAR' ?</code> asks about <code>CLEAR</code> whether or not the session defines it.</p>

<p>Three visibility rules are normative, because each ties what is shown to what the user is doing rather than to geometry:</p>

<ol>
<li><strong>Every surface stays reachable.</strong> A hidden surface is concealed, never destroyed, and some finite sequence of operations exposes it again from any state.</li>
<li><strong>Editing and execution are observable.</strong> Any state in which source text can be entered shows Input, and every Run shows Stack, so the produced stack is surfaced.</li>
<li><strong>At least one surface is always visible.</strong> The user is never shown nothing.</li>
</ol>

<p>Everything else — column geometry, the breakpoint at which the layout switches, gesture thresholds, tap counts — is tuning, not semantics, and has the same standing as the execution step limit: a host control rather than a language constraint.</p>

<p>The GUI consumes the current host protocol and does not independently decide exact-real equality, Word stack effects, dictionary resolution, absence metadata, or numeric representation. Web and Tauri platform adapters may supply capabilities, but may not change language observations. Existing accessibility names, focus paths, keyboard operation, and panel transitions are part of this GUI contract.</p>
