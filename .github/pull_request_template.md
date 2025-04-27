### Motivation and Context
<!--
1. Why is this change required?
2. What problem does it solve / scenario it enables?
3. If it fixes or closes an issue, add “Fixes #<num>”.
-->

### Description
<!--
High-level overview of the approach and design.
If helpful, point to relevant modules / files / data flow.
-->

### Checklist
<!-- Tick all that apply before requesting review -->

- [ ] Code builds with **no errors or warnings**
      `cargo check --all-targets`
- [ ] **Unit tests pass**
      `cargo test --verbose`
- [ ] **Clippy** passes with warnings denied
      `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo fmt` has been run
- [ ] Documentation updated where needed
- [ ] BREAKING CHANGE? **No** / _describe impact_
- [ ] I didn’t break anyone 😊

### Additional Notes (optional)
<!-- Logs, screenshots, things reviewers should focus on, etc. -->
