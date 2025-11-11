#!/bin/bash
# Add ToVNode implementations to all component files

for file in text.rs panel.rs container.rs flex.rs input.rs code_editor.rs alert.rs card.rs grid.rs toolbar.rs tabs.rs file_tree.rs; do
    if [ -f "$file" ]; then
        # Check if ToVNode is already imported
        if ! grep -q "use crate::to_vnode::ToVNode" "$file"; then
            # Add import after the first use statement
            sed -i '' '1,/^use /s|^\(use crate::simple_vnode.*\)$|\1\nuse crate::to_vnode::ToVNode;|' "$file"
        fi
        
        # Check if ToVNode impl already exists
        if ! grep -q "impl ToVNode for" "$file"; then
            # Get the struct name from the file
            struct_name=$(grep -m 1 "^pub struct" "$file" | sed 's/pub struct \([A-Za-z]*\).*/\1/')
            if [ -n "$struct_name" ]; then
                # Add ToVNode implementation at the end
                echo "" >> "$file"
                echo "// Implement ToVNode for $struct_name" >> "$file"
                echo "impl ToVNode for $struct_name {" >> "$file"
                echo "    fn to_vnode(self) -> VNode {" >> "$file"
                echo "        self.render()" >> "$file"
                echo "    }" >> "$file"
                echo "}" >> "$file"
                echo "Added ToVNode to $file ($struct_name)"
            fi
        fi
    fi
done
