#!/usr/bin/env bash
set -euo pipefail
examples=(
  examples/hello.phnl
  examples/conditions.phnl
  examples/factorial.phnl
  examples/functions.phnl
  examples/scopes.phnl
  examples/strings.phnl
)
for example in "${examples[@]}"; do
  echo "Checking executable example: $example"
  cargo run --quiet -- run "$example" >/dev/null
done
echo "Checking that the invalid type example is rejected"
if cargo run --quiet -- check examples/type_error.phnl >/dev/null 2>&1; then
  echo "Expected examples/type_error.phnl to fail semantic checking" >&2
  exit 1
fi
echo "All Phoneal example checks passed."
