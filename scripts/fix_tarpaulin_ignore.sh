#!/bin/bash
# Fix the tarpaulin ignore bug - add to ALL test functions in files with subprocesses
# The previous script only processed the first test per file due to line number shifts

set -e

cd "$(dirname "$0")/.."

count=0
files_modified=0

for file in tests/*.rs; do
  # Check if file spawns subprocesses
  if grep -q 'Command::new.*cargo\|compile_and_verify\|compile_should_succeed\|compile_and_run\|compile_fixture' "$file" 2>/dev/null; then
    file_modified=0
    
    # Get all test line numbers in reverse order (bottom to top)
    # This way, inserting lines doesn't affect line numbers of tests above
    test_lines=$(grep -n '^fn test_' "$file" | cut -d: -f1 | sort -rn)
    
    for line in $test_lines; do
      prev_line=$((line - 1))
      
      # Check if already has tarpaulin ignore
      if ! sed -n "${prev_line}p" "$file" | grep -q 'cfg_attr(tarpaulin'; then
        # Check if line before is #[test]
        if sed -n "${prev_line}p" "$file" | grep -q '^#\[test\]'; then
          test_name=$(sed -n "${line}p" "$file" | sed 's/fn \([^(]*\).*/\1/')
          echo "  $file: Adding ignore to $test_name (line $line)"
          
          # Insert #[cfg_attr(tarpaulin, ignore)] after #[test]
          sed -i '' "${prev_line}a\\
#[cfg_attr(tarpaulin, ignore)]
" "$file"
          
          count=$((count + 1))
          file_modified=1
        fi
      fi
    done
    
    if [ $file_modified -eq 1 ]; then
      files_modified=$((files_modified + 1))
    fi
  fi
done

echo ""
echo "✅ Added #[cfg_attr(tarpaulin, ignore)] to $count tests across $files_modified files"

