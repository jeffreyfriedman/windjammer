#!/bin/bash
# Add #[cfg_attr(tarpaulin, ignore)] to all test functions in files that spawn subprocesses

set -e

cd "$(dirname "$0")/.."

count=0

for file in tests/*.rs; do
  # Check if file spawns subprocesses
  if grep -q 'Command::new.*cargo\|compile_and_verify\|compile_should_succeed\|compile_and_run\|compile_fixture' "$file" 2>/dev/null; then
    echo "Processing: $file"
    
    # Find test functions without tarpaulin ignore
    grep -n '^fn test_' "$file" | while IFS=: read -r line func; do
      prev_line=$((line - 1))
      
      # Check if already has tarpaulin ignore
      if ! sed -n "${prev_line}p" "$file" | grep -q 'cfg_attr(tarpaulin'; then
        # Check if line before is #[test]
        if sed -n "${prev_line}p" "$file" | grep -q '^#\[test\]'; then
          echo "  Adding ignore to: $func"
          
          # Insert #[cfg_attr(tarpaulin, ignore)] after #[test]
          sed -i '' "${prev_line}a\\
#[cfg_attr(tarpaulin, ignore)]
" "$file"
          
          count=$((count + 1))
        fi
      fi
    done
  fi
done

echo ""
echo "Added #[cfg_attr(tarpaulin, ignore)] to $count tests"


