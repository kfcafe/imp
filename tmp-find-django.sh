#!/usr/bin/env bash
set -euo pipefail
cd evals/dirac-comparison/worktrees/datadict
commits=$(git rev-list --all -- django/forms/widgets.py django/contrib/admin/widgets.py django/contrib/postgres/forms/array.py django/forms/forms.py | head -5000)
for c in $commits; do
  ok=1
  for spec in \
    'django/forms/widgets.py 1bcfeba288' \
    'django/contrib/admin/widgets.py 7a0ccf42de' \
    'django/contrib/postgres/forms/array.py ae0c0c462b' \
    'django/forms/forms.py ce64f6286e'; do
    # Deliberately split the static two-field specs into path/blob words.
    # shellcheck disable=SC2086
    set -- $spec
    path=$1; blob=$2
    actual=$(git rev-parse "$c:$path" 2>/dev/null | cut -c1-10 || true)
    if [ "$actual" != "$blob" ]; then ok=0; break; fi
  done
  if [ $ok -eq 1 ]; then
    git show -s --format='%H %ci %s' "$c"
    exit 0
  fi
done
echo no-match
