impl IntInference {
    pub fn get_int_type<'ast>(&self, expr: &Expression<'ast>) -> IntType {
        let location = expr.location();
        let (file_id, line, col) = if let Some(loc) = location {
            (
                self.file_name_to_id
                    .get(&loc.file.to_string_lossy().to_string())
                    .copied()
                    .unwrap_or(0),
                loc.line,
                loc.column,
            )
        } else {
            (0, 0, 0)
        };

        let cache_key = (file_id, line, col);
        if let Some(&expr_id) = self.expr_id_cache.get(&cache_key) {
            if let Some(&int_ty) = self.inferred_types.get(&expr_id) {
                return int_ty;
            }
        }

        for (expr_id, int_ty) in &self.inferred_types {
            if expr_id.file_id == file_id && expr_id.line == line && expr_id.col == col {
                return *int_ty;
            }
        }

        IntType::Unknown
    }
}
