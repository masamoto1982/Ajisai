<h3 id="presentation-profile">Presentation Profile</h3>

<h4 id="gui-current-host-contract">Current GUI contract</h4>

<p>Presentation ranks below the language semantics: a conforming Ajisai need not present any surface visually, and a host with no display still defines the four observation projections. What follows is the contract the current Web and Tauri GUI realizes, stated concretely rather than as a formal system.</p>

<p>The GUI has four named surfaces: Input, Output, Stack, and Dictionary. Desktop presentation uses two columns (Input or Output on the left; Stack or Dictionary on the right), while mobile presentation exposes one selected surface at a time.</p>

<p>The operations Run, Step, Abort, and Reset retain their current behavior. The desktop shortcuts are respectively <code>Shift+Enter</code>, <code>Ctrl+Enter</code>, <code>Escape</code>, and <code>Ctrl+Alt+Enter</code>; Format is <code>Shift+Alt+F</code>, Stack clear is <code>Ctrl+Alt+S</code>, Editor clear is <code>Ctrl+Alt+E</code>, and Lookup is <code>Ctrl+Alt+L</code>. Stack clear discards the stack's values and leaves the dictionary — that is what distinguishes it from Reset. Editor clear discards only the Input surface's typed text, leaving the stack, the dictionary, and everything else untouched. Output copy and focus behavior, Core/User dictionary sheets, search, deletion, canonical stack display, opt-in LaTeX display, nested-vector bracket coloring, modifier background indication, stack snapshots, and persistence of user words are compatibility requirements.</p>

<p>The Stack surface reads in ordinary reading order: top to bottom, left to right, with the top of the stack last. The top of the stack is marked, so the value the next Word reads is identifiable without counting, at any number of values and whatever the wrapping.</p>

<p><strong>Operations without a typed spelling.</strong> Reset, Stack clear, Editor clear, and Lookup exist only as shortcuts (Stack clear and Editor clear also as buttons); none of them can be typed into the Input surface and run. This is deliberate in both directions: the vocabulary holds nothing that throws away the values a program was handed, the text it was typed as, or the dictionary it was defined in, and the Input surface holds nothing but the program being written. A name typed there — including <code>RESET</code>, <code>STACK-CLEAR</code>, or <code>EDITOR-CLEAR</code> — resolves as an ordinary program, exactly like any other name, and is a User Word or an unknown word like any other.</p>

<p><strong>Lookup</strong> resolves the word at the current cursor position in the Input surface against the active dictionary and answers without running anything, always in the Output surface: a Core Word's reference text and a User Word's reconstructed <code>DEF</code> source are shown the same way, as prose to read rather than text loaded back for editing. The cursor can be anywhere in a program still being written, so nothing about a lookup may overwrite the Input surface. A name the dictionary does not hold is reported as an unknown word. This too is deliberately not a Word: reference prose is not a value, no program can do anything with the answer, and a Word whose entire result reached the output rather than the stack would be claiming otherwise. The lookup observes and changes nothing — the stack, the dictionary, and the Input surface are the same afterwards — and it must identify the same canonical entry that hover, the Reference, and execution do.</p>

<p>Three visibility rules are normative, because each ties what is shown to what the user is doing rather than to geometry:</p>

<ol>
<li><strong>Every surface stays reachable.</strong> A hidden surface is concealed, never destroyed, and some finite sequence of operations exposes it again from any state.</li>
<li><strong>Editing and execution are observable.</strong> Any state in which source text can be entered shows Input, and every Run shows Stack, so the produced stack is surfaced.</li>
<li><strong>At least one surface is always visible.</strong> The user is never shown nothing.</li>
</ol>

<p>Everything else — column geometry, the breakpoint at which the layout switches, gesture thresholds, tap counts — is tuning, not semantics, and has the same standing as the execution step limit: a host control rather than a language constraint.</p>

<p>The GUI consumes the current host protocol and does not independently decide exact-real equality, Word stack effects, dictionary resolution, absence metadata, or numeric representation. Web and Tauri platform adapters may supply capabilities, but may not change language observations. Existing accessibility names, focus paths, keyboard operation, and panel transitions are part of this GUI contract.</p>
