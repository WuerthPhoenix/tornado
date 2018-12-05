# Contributing

## Bug reports and Feature requests
TBD

## Source Code
Please verify that all your committed code is formatted with cargo fmt.
The common used format is already defined in `rustfmt.toml` and should not be overridden.

### Pre-Commit Hook
You can automate this check by using the provided `pre-commit` hook which will fail if the
project is not in a correctly formatted state.
You can easily enable it (given that you don't have other pre-commit hooks enabled) by
running the following command from the git root.
```
ln -s ../../src/scripts/devel/pre-commit.sh .git/hooks/pre-commit
```
### Building & Testing
TBD
