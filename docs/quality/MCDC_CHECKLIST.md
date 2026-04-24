# MC/DC-like Review Checklist

Use this checklist when modifying non-trivial boolean logic (especially QL-A / QL-B paths).

- [ ] Have all atomic boolean conditions in the decision been listed?
- [ ] Is there evidence each condition can independently affect the outcome?
- [ ] Are short-circuit paths covered by tests or reasoned analysis?
- [ ] Are both true/false outcomes exercised for critical decisions?
- [ ] Are boundary/guard combinations represented (null/empty/limit/error)?
- [ ] Have added tests been linked in the traceability matrix where applicable?
