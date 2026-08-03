# Code/data reflection design record (2026-08)

This is a non-normative design-decision record; `spec/` remains the authority for semantics.

`REFLECT` names a symmetric reflection rather than privileging either destination as `CODE` would. One involutive Word expresses one reversible capability, so two directional Words would unnecessarily enlarge Core. The explicit boundary justifies growing Core from 69 to 70: it adds metaprogramming without making ordinary Vector data executable.

`EXEC` remains CodeBlock-only. The public `AJISAI-CODE-1` Vector is deliberately distinct from the private persistence wire format, preserving snapshot compatibility. This change introduces neither a String parser nor a macro system. It keeps Vector a data domain and adopts structural/token equality for the involution, preserving lexical spelling without relying on display rendering.
