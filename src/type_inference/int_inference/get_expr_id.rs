impl IntInference {
    fn get_expr_id<'ast>(&mut self, expr: &Expression<'ast>) -> ExprId {
        let (line, col) = expr
            .location()
            .map(|loc| (loc.line, loc.column))
            .unwrap_or((0, 0));

        let cache_key = (self.current_file_id, line, col);
        if line > 0 {
            if let Some(&cached_id) = self.expr_id_cache.get(&cache_key) {
                return cached_id;
            }
        }

        let seq_id = self.next_seq_id;
        self.next_seq_id += 1;

        let expr_id = ExprId {
            seq_id,
            file_id: self.current_file_id,
            line,
            col,
        };

        if line > 0 {
            self.expr_id_cache.insert(cache_key, expr_id);
        }

        expr_id
    }
}
