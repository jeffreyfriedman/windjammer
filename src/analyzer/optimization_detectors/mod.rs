//! Optimization detection methods for the analyzer.
//! PHASE 2-9: Clone, struct mapping, string, assignment, defer drop,
//! const/static, SmallVec, and Cow optimization detection.

mod assignment_detection;
mod const_smallvec_detection;
mod cow_detection;
mod defer_drop_detection;
mod loop_optimization_detection;
mod string_optimization_detection;
mod struct_mapping_detection;
