## Description

<!-- Provide a clear and concise description of your changes -->

## Type of Change

<!-- Mark the relevant option with an "x" -->

- [ ] Bug fix (non-breaking change that fixes an issue)
- [ ] New feature (non-breaking change that adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Code refactoring
- [ ] Test improvement
- [ ] CI/CD improvement

## Related Issue

<!-- Link to the issue this PR addresses -->

Fixes #(issue number)

## Changes Made

<!-- List the main changes in bullet points -->

- 
- 
- 

## Testing

<!-- Describe the tests you ran and how to reproduce them -->

### Test Environment
- OS: 
- Rust Version: 
- Orbit Version: 

### Test Cases
<!-- Describe what you tested -->

```bash
# Commands used for testing
orbit -s test.txt -d backup.txt
```

### Test Results
- [ ] All existing tests pass (`cargo test`)
- [ ] New tests added for new functionality
- [ ] Manual testing completed
- [ ] Tested on multiple platforms (if applicable)

## Checklist

<!-- Mark completed items with an "x" -->

### Code Quality
- [ ] My code follows the project's style guidelines
- [ ] I have performed a self-review of my code
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] My changes generate no new warnings
- [ ] I have run `cargo clippy` and addressed any issues

### Documentation
- [ ] I have updated the documentation accordingly
- [ ] I have updated CHANGELOG.md
- [ ] I have added/updated code comments (///)
- [ ] I have updated README.md if needed

### Testing
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing unit tests pass locally with my changes (`cargo test`)
- [ ] I have tested the changes manually
- [ ] I have tested edge cases

### Breaking Changes
- [ ] This PR does NOT introduce breaking changes
- [ ] OR: I have documented all breaking changes in MIGRATION_GUIDE.md
- [ ] OR: I have updated version number appropriately

## Performance Impact

<!-- If applicable, describe any performance implications -->

- [ ] No performance impact
- [ ] Performance improved (provide benchmarks)
- [ ] Performance degraded (explain why this is acceptable)

**Benchmarks:**
```
<!-- Paste benchmark results if applicable -->
```

## Screenshots

<!-- If applicable, add screenshots to demonstrate the changes -->

## Additional Notes

<!-- Any additional information that reviewers should know -->

## Reviewer Checklist

<!-- For maintainers reviewing this PR -->

- [ ] Code is clean and follows project conventions
- [ ] Tests are adequate and pass
- [ ] Documentation is updated
- [ ] No security concerns
- [ ] No performance regressions
- [ ] Breaking changes are properly documented

---

**By submitting this PR, I confirm that:**
- My contribution is made under the project's license (Apache License 2.0)
- I have read and understood the [Contributing Guidelines](CONTRIBUTING.md)
- I have read and agree to the [Code of Conduct](CODE_OF_CONDUCT.md)