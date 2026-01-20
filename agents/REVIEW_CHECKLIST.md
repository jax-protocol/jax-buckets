# Code Review Checklist

Quick checks for PR review, in addition to RUST_PATTERNS.md.

## API Surface
- [ ] CLI flags are minimal and well-designed
- [ ] Infrastructure config is at init time, not daemon time
- [ ] No unnecessary env var support
- [ ] Related params combined (URLs vs separate fields)

## Module Organization
- [ ] New code follows existing patterns (check similar modules)
- [ ] Files have single responsibility (< 200 lines typical)
- [ ] Setup logic in dedicated modules, not mixed with state

## Dead Code
- [ ] All public methods have callers
- [ ] No `#[allow(dead_code)]` without justification
- [ ] No speculative abstractions (Deref, From, etc. without use)

## Documentation Sync
- [ ] PROJECT_LAYOUT.md updated for new files/modules
- [ ] Issue tickets updated if implementation differs from spec
- [ ] Downstream tickets checked for outdated references
