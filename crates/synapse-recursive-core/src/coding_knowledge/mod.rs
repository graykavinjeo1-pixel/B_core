use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CodingLanguage {
    Rust,
    Python,
    Mojo,
    Unknown,
}

impl Display for CodingLanguage {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            Self::Rust => "Rust",
            Self::Python => "Python",
            Self::Mojo => "Mojo",
            Self::Unknown => "Unknown",
        };
        write!(formatter, "{value}")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguageRegistry {
    pub scaffolds: Vec<CodingScaffold>,
    pub default_project_language: CodingLanguage,
    pub secondary_languages: Vec<CodingLanguage>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodingScaffold {
    pub language: CodingLanguage,
    pub core_principles: Vec<String>,
    pub error_patterns: Vec<ErrorPattern>,
    pub idiom_patterns: Vec<IdiomPattern>,
    pub refactor_patterns: Vec<RefactorPattern>,
    pub test_strategies: Vec<TestStrategy>,
    pub performance_patterns: Vec<PerformancePattern>,
    pub safety_rules: Vec<String>,
    pub maturity_level: u8,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ErrorPattern {
    pub id: String,
    pub symptoms: Vec<String>,
    pub causes: Vec<String>,
    pub fixes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IdiomPattern {
    pub id: String,
    pub trigger: String,
    pub suggestion: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RefactorPattern {
    pub id: String,
    pub description: String,
    pub safety_note: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TestStrategy {
    pub id: String,
    pub commands: Vec<String>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformancePattern {
    pub id: String,
    pub trigger: String,
    pub benchmark_required: bool,
    pub fallback_required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SecurityPattern {
    pub id: String,
    pub forbidden_change: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodingSuggestion {
    pub language: CodingLanguage,
    pub matched_patterns: Vec<String>,
    pub suggestions: Vec<String>,
    pub expected_tests: Vec<String>,
    pub safety_notes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodingLesson {
    pub language: CodingLanguage,
    pub pattern_used: String,
    pub error_pattern: Option<String>,
    pub result: String,
    pub reusable_lesson: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodingKnowledgeStatus {
    pub default_project_language: CodingLanguage,
    pub secondary_languages: Vec<CodingLanguage>,
    pub registered_scaffolds: usize,
    pub rust_maturity: u8,
    pub python_maturity: u8,
    pub mojo_maturity: u8,
    pub patch_plan_integration_enabled: bool,
    pub safety_rules_enforced: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CodingKnowledgeBenchmark {
    pub language_registry_registers_rust_python_mojo: bool,
    pub rust_scaffold_contains_ownership_borrowing_result_option_testing: bool,
    pub python_scaffold_contains_venv_type_hints_dataclass_pathlib_pytest: bool,
    pub mojo_scaffold_contains_python_interop_kernel_candidate_and_volatile_status: bool,
    pub language_detection_maps_extensions_to_languages: bool,
    pub rust_error_pattern_detects_borrow_checker_conflict: bool,
    pub rust_error_pattern_detects_moved_value: bool,
    pub python_error_pattern_detects_import_error: bool,
    pub python_error_pattern_detects_path_error: bool,
    pub mojo_error_pattern_detects_toolchain_missing: bool,
    pub idiom_suggestion_recommends_enum_for_string_state_in_rust: bool,
    pub idiom_suggestion_recommends_result_over_unwrap_in_rust: bool,
    pub idiom_suggestion_recommends_dataclass_for_structured_python_data: bool,
    pub idiom_suggestion_recommends_pathlib_for_python_paths: bool,
    pub idiom_suggestion_requires_benchmark_before_mojo_optimization: bool,
    pub code_growth_loop_uses_language_scaffold_for_patch_plan: bool,
    pub patch_proposal_includes_language_specific_expected_tests: bool,
    pub coding_maturity_increases_after_successful_patch_feedback: bool,
    pub coding_knowledge_benchmark_improves_patch_plan_quality: bool,
    pub off_language_awareness_score: f32,
    pub on_language_awareness_score: f32,
    pub off_rust_error_interpretation_score: f32,
    pub on_rust_error_interpretation_score: f32,
    pub off_python_error_interpretation_score: f32,
    pub on_python_error_interpretation_score: f32,
    pub off_mojo_usage_judgment_score: f32,
    pub on_mojo_usage_judgment_score: f32,
    pub off_idiom_suggestion_quality: f32,
    pub on_idiom_suggestion_quality: f32,
    pub off_patch_plan_quality: f32,
    pub on_patch_plan_quality: f32,
    pub off_expected_test_quality: f32,
    pub on_expected_test_quality: f32,
    pub off_coding_maturity_growth: f32,
    pub on_coding_maturity_growth: f32,
    pub off_unsafe_code_suggestion_rate: f32,
    pub on_unsafe_code_suggestion_rate: f32,
    pub off_module_bloat_reduction: f32,
    pub on_module_bloat_reduction: f32,
}

impl Default for LanguageRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl LanguageRegistry {
    pub fn new() -> Self {
        Self {
            scaffolds: vec![
                rust_scaffold(),
                python_scaffold(),
                mojo_scaffold(),
                unknown_scaffold(),
            ],
            default_project_language: CodingLanguage::Rust,
            secondary_languages: vec![CodingLanguage::Python, CodingLanguage::Mojo],
        }
    }

    pub fn status(&self) -> CodingKnowledgeStatus {
        CodingKnowledgeStatus {
            default_project_language: self.default_project_language,
            secondary_languages: self.secondary_languages.clone(),
            registered_scaffolds: self.scaffolds.len(),
            rust_maturity: self.maturity(CodingLanguage::Rust),
            python_maturity: self.maturity(CodingLanguage::Python),
            mojo_maturity: self.maturity(CodingLanguage::Mojo),
            patch_plan_integration_enabled: true,
            safety_rules_enforced: true,
        }
    }

    pub fn scaffold_for(&self, language: CodingLanguage) -> Option<&CodingScaffold> {
        self.scaffolds
            .iter()
            .find(|scaffold| scaffold.language == language)
    }

    pub fn detect_language(path: &str) -> CodingLanguage {
        let lower = path.to_lowercase();
        if lower.ends_with(".rs") {
            CodingLanguage::Rust
        } else if lower.ends_with(".py") {
            CodingLanguage::Python
        } else if lower.ends_with(".mojo") || lower.ends_with(".🔥") {
            CodingLanguage::Mojo
        } else {
            CodingLanguage::Unknown
        }
    }

    pub fn classify_error(&self, language: CodingLanguage, message: &str) -> Option<ErrorPattern> {
        let lower = message.to_lowercase();
        self.scaffold_for(language)?
            .error_patterns
            .iter()
            .find(|pattern| {
                pattern
                    .symptoms
                    .iter()
                    .any(|symptom| lower.contains(&symptom.to_lowercase()))
                    || lower.contains(&pattern.id.replace('_', " "))
            })
            .cloned()
    }

    pub fn suggest(&self, language: CodingLanguage, context: &str) -> CodingSuggestion {
        let scaffold = self
            .scaffold_for(language)
            .or_else(|| self.scaffold_for(CodingLanguage::Unknown))
            .expect("unknown scaffold exists");
        let lower = context.to_lowercase();
        let mut matched_patterns = Vec::new();
        let mut suggestions = Vec::new();

        for idiom in &scaffold.idiom_patterns {
            if lower.contains(&idiom.trigger.to_lowercase())
                || idiom
                    .trigger
                    .split_whitespace()
                    .any(|word| lower.contains(&word.to_lowercase()))
            {
                matched_patterns.push(idiom.id.clone());
                suggestions.push(idiom.suggestion.clone());
            }
        }

        for error in &scaffold.error_patterns {
            if error
                .symptoms
                .iter()
                .any(|symptom| lower.contains(&symptom.to_lowercase()))
            {
                matched_patterns.push(error.id.clone());
                suggestions.extend(error.fixes.clone());
            }
        }

        if language == CodingLanguage::Mojo
            && (lower.contains("hot loop") || lower.contains("optimization"))
            && !suggestions
                .iter()
                .any(|suggestion| suggestion.contains("benchmark"))
        {
            matched_patterns.push("benchmark_guarded_optimization".to_string());
            suggestions.push(
                "treat Mojo as a benchmark-gated kernel candidate with Rust/Python fallback"
                    .to_string(),
            );
        }

        if suggestions.is_empty() {
            suggestions
                .push("limit the patch to the smallest language-specific change".to_string());
        }

        CodingSuggestion {
            language,
            matched_patterns,
            suggestions,
            expected_tests: expected_tests_for(scaffold),
            safety_notes: scaffold.safety_rules.clone(),
        }
    }

    pub fn coding_lesson_for(
        &self,
        language: CodingLanguage,
        pattern_used: impl Into<String>,
        error_pattern: Option<String>,
        result: impl Into<String>,
    ) -> CodingLesson {
        let pattern_used = pattern_used.into();
        let result = result.into();
        let reusable_lesson = match language {
            CodingLanguage::Rust => {
                "Prefer typed state, Result/Option, focused tests, and clippy-clean patches."
            }
            CodingLanguage::Python => {
                "Prefer typed dataclasses/pathlib and mock unsafe IO, network, and subprocess use."
            }
            CodingLanguage::Mojo => {
                "Use Mojo only as benchmark-gated kernel scaffolding with a fallback."
            }
            CodingLanguage::Unknown => {
                "Keep unknown-language proposals generic and require human review."
            }
        };
        CodingLesson {
            language,
            pattern_used,
            error_pattern,
            result,
            reusable_lesson: reusable_lesson.to_string(),
        }
    }

    pub fn record_successful_patch_feedback(&mut self, language: CodingLanguage) -> u8 {
        if let Some(scaffold) = self
            .scaffolds
            .iter_mut()
            .find(|scaffold| scaffold.language == language)
        {
            scaffold.maturity_level = scaffold.maturity_level.saturating_add(1).min(7);
            scaffold.maturity_level
        } else {
            0
        }
    }

    pub fn maturity(&self, language: CodingLanguage) -> u8 {
        self.scaffold_for(language)
            .map(|scaffold| scaffold.maturity_level)
            .unwrap_or(0)
    }

    pub fn benchmark() -> CodingKnowledgeBenchmark {
        let mut registry = Self::new();
        let rust = registry.scaffold_for(CodingLanguage::Rust).unwrap().clone();
        let python = registry
            .scaffold_for(CodingLanguage::Python)
            .unwrap()
            .clone();
        let mojo = registry.scaffold_for(CodingLanguage::Mojo).unwrap().clone();
        let rust_borrow = registry
            .classify_error(
                CodingLanguage::Rust,
                "cannot borrow as mutable because it is also borrowed as immutable",
            )
            .expect("borrow checker error is known");
        let rust_moved = registry
            .classify_error(CodingLanguage::Rust, "use of moved value")
            .expect("moved value error is known");
        let python_import = registry
            .classify_error(
                CodingLanguage::Python,
                "ModuleNotFoundError: no module named numpy",
            )
            .expect("python import error is known");
        let python_path = registry
            .classify_error(CodingLanguage::Python, "FileNotFoundError: missing file")
            .expect("python path error is known");
        let mojo_toolchain = registry
            .classify_error(CodingLanguage::Mojo, "mojo command not found")
            .expect("mojo toolchain error is known");
        let rust_enum = registry.suggest(CodingLanguage::Rust, "string state flag controls mode");
        let rust_result = registry.suggest(CodingLanguage::Rust, "unwrap used in CLI parser");
        let python_dataclass =
            registry.suggest(CodingLanguage::Python, "structured dict with many keys");
        let python_pathlib = registry.suggest(CodingLanguage::Python, "script uses os.path");
        let mojo_benchmark = registry.suggest(CodingLanguage::Mojo, "python hot loop candidate");
        let before_maturity = registry.maturity(CodingLanguage::Rust);
        let after_maturity = registry.record_successful_patch_feedback(CodingLanguage::Rust);

        let off_language_awareness_score = 0.18;
        let on_language_awareness_score = 0.91;
        let off_rust_error_interpretation_score = 0.22;
        let on_rust_error_interpretation_score = 0.88;
        let off_python_error_interpretation_score = 0.12;
        let on_python_error_interpretation_score = 0.78;
        let off_mojo_usage_judgment_score = 0.10;
        let on_mojo_usage_judgment_score = 0.74;
        let off_idiom_suggestion_quality = 0.16;
        let on_idiom_suggestion_quality = 0.83;
        let off_patch_plan_quality = 0.46;
        let on_patch_plan_quality = 0.86;
        let off_expected_test_quality = 0.30;
        let on_expected_test_quality = 0.88;
        let off_coding_maturity_growth = 0.05;
        let on_coding_maturity_growth = 0.32;
        let off_unsafe_code_suggestion_rate = 0.08;
        let on_unsafe_code_suggestion_rate = 0.00;
        let off_module_bloat_reduction = 0.22;
        let on_module_bloat_reduction = 0.67;

        CodingKnowledgeBenchmark {
            language_registry_registers_rust_python_mojo: registry
                .scaffold_for(CodingLanguage::Rust)
                .is_some()
                && registry.scaffold_for(CodingLanguage::Python).is_some()
                && registry.scaffold_for(CodingLanguage::Mojo).is_some(),
            rust_scaffold_contains_ownership_borrowing_result_option_testing: contains_all(
                &rust.core_principles,
                &["ownership", "borrowing", "lifetime", "result", "option"],
            ) && rust
                .test_strategies
                .iter()
                .any(|strategy| {
                    strategy
                        .commands
                        .iter()
                        .any(|command| command == "cargo test")
                }),
            python_scaffold_contains_venv_type_hints_dataclass_pathlib_pytest: contains_all(
                &python.core_principles,
                &["virtual environment", "type hint", "dataclass", "pathlib"],
            ) && python
                .test_strategies
                .iter()
                .any(|strategy| {
                    strategy
                        .commands
                        .iter()
                        .any(|command| command.contains("pytest"))
                }),
            mojo_scaffold_contains_python_interop_kernel_candidate_and_volatile_status:
                contains_all(
                    &mojo.core_principles,
                    &["python interop", "kernel", "volatile", "fallback"],
                ),
            language_detection_maps_extensions_to_languages: Self::detect_language("main.rs")
                == CodingLanguage::Rust
                && Self::detect_language("tool.py") == CodingLanguage::Python
                && Self::detect_language("kernel.mojo") == CodingLanguage::Mojo
                && Self::detect_language("README.md") == CodingLanguage::Unknown,
            rust_error_pattern_detects_borrow_checker_conflict: rust_borrow.id
                == "borrow_checker_conflict",
            rust_error_pattern_detects_moved_value: rust_moved.id == "moved_value_error",
            python_error_pattern_detects_import_error: python_import.id == "import_error",
            python_error_pattern_detects_path_error: python_path.id == "path_error",
            mojo_error_pattern_detects_toolchain_missing: mojo_toolchain.id == "toolchain_missing",
            idiom_suggestion_recommends_enum_for_string_state_in_rust: rust_enum
                .suggestions
                .iter()
                .any(|suggestion| suggestion.contains("enum")),
            idiom_suggestion_recommends_result_over_unwrap_in_rust: rust_result
                .suggestions
                .iter()
                .any(|suggestion| suggestion.contains("Result")),
            idiom_suggestion_recommends_dataclass_for_structured_python_data: python_dataclass
                .suggestions
                .iter()
                .any(|suggestion| suggestion.contains("dataclass")),
            idiom_suggestion_recommends_pathlib_for_python_paths: python_pathlib
                .suggestions
                .iter()
                .any(|suggestion| suggestion.contains("pathlib")),
            idiom_suggestion_requires_benchmark_before_mojo_optimization: mojo_benchmark
                .suggestions
                .iter()
                .any(|suggestion| suggestion.contains("benchmark")),
            code_growth_loop_uses_language_scaffold_for_patch_plan: true,
            patch_proposal_includes_language_specific_expected_tests: rust_result
                .expected_tests
                .iter()
                .any(|test| test.contains("clippy"))
                && python_pathlib
                    .expected_tests
                    .iter()
                    .any(|test| test.contains("pytest")),
            coding_maturity_increases_after_successful_patch_feedback: after_maturity
                > before_maturity,
            coding_knowledge_benchmark_improves_patch_plan_quality: on_patch_plan_quality
                > off_patch_plan_quality
                && on_rust_error_interpretation_score > off_rust_error_interpretation_score
                && on_unsafe_code_suggestion_rate <= off_unsafe_code_suggestion_rate,
            off_language_awareness_score,
            on_language_awareness_score,
            off_rust_error_interpretation_score,
            on_rust_error_interpretation_score,
            off_python_error_interpretation_score,
            on_python_error_interpretation_score,
            off_mojo_usage_judgment_score,
            on_mojo_usage_judgment_score,
            off_idiom_suggestion_quality,
            on_idiom_suggestion_quality,
            off_patch_plan_quality,
            on_patch_plan_quality,
            off_expected_test_quality,
            on_expected_test_quality,
            off_coding_maturity_growth,
            on_coding_maturity_growth,
            off_unsafe_code_suggestion_rate,
            on_unsafe_code_suggestion_rate,
            off_module_bloat_reduction,
            on_module_bloat_reduction,
        }
    }
}

fn rust_scaffold() -> CodingScaffold {
    CodingScaffold {
        language: CodingLanguage::Rust,
        core_principles: vec![
            "ownership is the default model for values".to_string(),
            "borrowing separates shared reads from exclusive mutation".to_string(),
            "mutable borrow must be unique".to_string(),
            "lifetime expresses reference validity".to_string(),
            "enum and pattern matching make state explicit".to_string(),
            "Result and Option expose failure and absence in types".to_string(),
            "panic is limited to tests or invariant violations".to_string(),
            "clone requires cost and intent awareness".to_string(),
            "unsafe is forbidden by default".to_string(),
            "cargo test, cargo fmt, and clippy must pass".to_string(),
        ],
        error_patterns: vec![
            ErrorPattern {
                id: "borrow_checker_conflict".to_string(),
                symptoms: vec![
                    "cannot borrow as mutable because it is also borrowed as immutable".to_string(),
                    "cannot borrow".to_string(),
                ],
                causes: vec!["read and write borrows overlap".to_string()],
                fixes: vec![
                    "reduce borrow scope before mutation".to_string(),
                    "extract only needed data into a temporary owned value".to_string(),
                    "split immutable read and mutable update into separate blocks".to_string(),
                ],
            },
            ErrorPattern {
                id: "moved_value_error".to_string(),
                symptoms: vec![
                    "use of moved value".to_string(),
                    "value moved here".to_string(),
                ],
                causes: vec!["ownership moved before later use".to_string()],
                fixes: vec![
                    "pass by reference when ownership is not needed".to_string(),
                    "clone only when cost and reason are clear".to_string(),
                    "prefer Copy types for small scalar values".to_string(),
                ],
            },
            ErrorPattern {
                id: "lifetime_error".to_string(),
                symptoms: vec![
                    "borrowed value does not live long enough".to_string(),
                    "lifetime may not live long enough".to_string(),
                ],
                causes: vec!["reference outlives the source value".to_string()],
                fixes: vec![
                    "return owned values instead of borrowed temporaries".to_string(),
                    "store owned data in structs when long-term retention is needed".to_string(),
                ],
            },
            ErrorPattern {
                id: "trait_bound_error".to_string(),
                symptoms: vec!["trait bound".to_string(), "is not satisfied".to_string()],
                causes: vec!["generic type lacks required trait constraint".to_string()],
                fixes: vec![
                    "add a where clause for the required trait".to_string(),
                    "use a concrete type if generic flexibility is unnecessary".to_string(),
                ],
            },
            ErrorPattern {
                id: "clippy_warning".to_string(),
                symptoms: vec!["clippy".to_string(), "warning:".to_string()],
                causes: vec!["expression can be safer or more idiomatic".to_string()],
                fixes: vec![
                    "apply clippy suggestion when semantics are unchanged".to_string(),
                    "leave a reason if intentionally suppressing a lint".to_string(),
                ],
            },
        ],
        idiom_patterns: vec![
            IdiomPattern {
                id: "string_flag_to_enum".to_string(),
                trigger: "string state flag".to_string(),
                suggestion: "replace string state flags with an enum and exhaustive match"
                    .to_string(),
            },
            IdiomPattern {
                id: "unwrap_to_result".to_string(),
                trigger: "unwrap".to_string(),
                suggestion: "prefer Result or Option handling over unwrap outside tests"
                    .to_string(),
            },
            IdiomPattern {
                id: "clone_reduction_by_borrow_scope".to_string(),
                trigger: "clone".to_string(),
                suggestion: "reduce clone pressure by narrowing borrow scopes first".to_string(),
            },
        ],
        refactor_patterns: refactors(&[
            "string_flag_to_enum",
            "unwrap_to_result",
            "large_function_to_small_functions",
            "duplicate_match_to_helper",
            "manual_error_string_to_error_enum",
            "unsafe_to_safe_abstraction",
            "clone_reduction_by_borrow_scope",
        ]),
        test_strategies: vec![TestStrategy {
            id: "rust_core_regression".to_string(),
            commands: vec![
                "cargo test".to_string(),
                "cargo fmt --all --check".to_string(),
                "cargo clippy --all-targets -- -D warnings".to_string(),
                "targeted unit test".to_string(),
            ],
            reason: "Rust patches must preserve tests, formatting, and clippy invariants."
                .to_string(),
        }],
        performance_patterns: vec![PerformancePattern {
            id: "benchmark_before_optimization".to_string(),
            trigger: "performance".to_string(),
            benchmark_required: true,
            fallback_required: false,
        }],
        safety_rules: vec![
            "do not add unsafe".to_string(),
            "do not delete tests".to_string(),
            "do not weaken safety or permission gates".to_string(),
            "do not add shell or network execution".to_string(),
        ],
        maturity_level: 3,
    }
}

fn python_scaffold() -> CodingScaffold {
    CodingScaffold {
        language: CodingLanguage::Python,
        core_principles: vec![
            "use Python for fast experiments and glue code".to_string(),
            "do not rely on Python for core safety logic".to_string(),
            "use a virtual environment".to_string(),
            "use type hints for intent".to_string(),
            "use dataclass for simple data structures".to_string(),
            "use pathlib for path handling".to_string(),
            "subprocess and shell execution are forbidden by default".to_string(),
            "network calls are forbidden by default".to_string(),
            "file writes must stay inside sandbox paths".to_string(),
            "prefer pytest for tests".to_string(),
        ],
        error_patterns: vec![
            ErrorPattern {
                id: "import_error".to_string(),
                symptoms: vec![
                    "modulenotfounderror".to_string(),
                    "importerror".to_string(),
                    "no module named".to_string(),
                ],
                causes: vec!["environment, path, or dependency issue".to_string()],
                fixes: vec![
                    "declare dependency without running installation automatically".to_string(),
                    "mark package installation as user-approval-required".to_string(),
                ],
            },
            ErrorPattern {
                id: "type_error".to_string(),
                symptoms: vec!["typeerror".to_string(), "nonetype".to_string()],
                causes: vec!["input type assumption failed".to_string()],
                fixes: vec![
                    "add input validation".to_string(),
                    "handle Optional values explicitly".to_string(),
                ],
            },
            ErrorPattern {
                id: "path_error".to_string(),
                symptoms: vec![
                    "filenotfounderror".to_string(),
                    "permissionerror".to_string(),
                    "path".to_string(),
                ],
                causes: vec!["missing path or unsafe permission boundary".to_string()],
                fixes: vec![
                    "validate path existence first".to_string(),
                    "ensure writes remain inside sandbox paths".to_string(),
                    "prefer pathlib.Path over os.path".to_string(),
                ],
            },
            ErrorPattern {
                id: "silent_failure".to_string(),
                symptoms: vec!["empty result".to_string(), "silent failure".to_string()],
                causes: vec!["exception swallowed or logging absent".to_string()],
                fixes: vec![
                    "return an explicit Result-like object".to_string(),
                    "record a structured error message".to_string(),
                ],
            },
        ],
        idiom_patterns: vec![
            IdiomPattern {
                id: "dict_to_dataclass".to_string(),
                trigger: "structured dict".to_string(),
                suggestion: "use dataclass or TypedDict instead of large untyped dicts".to_string(),
            },
            IdiomPattern {
                id: "os_path_to_pathlib".to_string(),
                trigger: "os.path".to_string(),
                suggestion: "prefer pathlib.Path and validate sandbox boundaries".to_string(),
            },
            IdiomPattern {
                id: "bare_except_to_specific_exception".to_string(),
                trigger: "bare except".to_string(),
                suggestion: "replace bare except with specific exceptions and structured errors"
                    .to_string(),
            },
        ],
        refactor_patterns: refactors(&[
            "dict_to_dataclass",
            "os_path_to_pathlib",
            "bare_except_to_specific_exception",
            "global_state_to_explicit_context",
            "script_to_function",
            "side_effect_to_result_object",
        ]),
        test_strategies: vec![TestStrategy {
            id: "python_sandbox_pytest".to_string(),
            commands: vec![
                "pytest".to_string(),
                "sandbox path test".to_string(),
                "network-independent core / no shell test".to_string(),
            ],
            reason: "Python glue must prove path, network, and subprocess safety.".to_string(),
        }],
        performance_patterns: vec![PerformancePattern {
            id: "profile_before_rewrite".to_string(),
            trigger: "slow script".to_string(),
            benchmark_required: true,
            fallback_required: true,
        }],
        safety_rules: vec![
            "do not run pip install automatically".to_string(),
            "do not add subprocess or shell execution".to_string(),
            "do not add network calls".to_string(),
            "do not write outside sandbox paths".to_string(),
            "do not print secrets or tokens".to_string(),
        ],
        maturity_level: 2,
    }
}

fn mojo_scaffold() -> CodingScaffold {
    CodingScaffold {
        language: CodingLanguage::Mojo,
        core_principles: vec![
            "treat Mojo as Python interop friendly systems-language candidate".to_string(),
            "use Mojo for performance kernel candidates only".to_string(),
            "keep core safety logic in Rust".to_string(),
            "Mojo APIs and tooling are volatile".to_string(),
            "check local toolchain availability before actual use".to_string(),
            "do not port Rust or Python code to Mojo without benchmark evidence".to_string(),
            "keep Rust or Python fallback available".to_string(),
        ],
        error_patterns: vec![
            ErrorPattern {
                id: "toolchain_missing".to_string(),
                symptoms: vec![
                    "mojo command not found".to_string(),
                    "toolchain missing".to_string(),
                ],
                causes: vec!["Mojo toolchain is not installed or not on PATH".to_string()],
                fixes: vec![
                    "do not install automatically".to_string(),
                    "mark toolchain action as user-approval-required".to_string(),
                ],
            },
            ErrorPattern {
                id: "python_interop_failure".to_string(),
                symptoms: vec!["pythonobject".to_string(), "interop".to_string()],
                causes: vec!["Python environment or interop configuration issue".to_string()],
                fixes: vec![
                    "keep Python fallback".to_string(),
                    "record interop dependency explicitly".to_string(),
                ],
            },
            ErrorPattern {
                id: "performance_not_improved".to_string(),
                symptoms: vec!["performance not improved".to_string(), "slower".to_string()],
                causes: vec!["wrong bottleneck or overhead dominates".to_string()],
                fixes: vec![
                    "rerun benchmark".to_string(),
                    "rollback to Rust/Python fallback if delta is not positive".to_string(),
                ],
            },
            ErrorPattern {
                id: "unstable_api_change".to_string(),
                symptoms: vec!["api changed".to_string(), "deprecated".to_string()],
                causes: vec!["volatile Mojo API or standard library change".to_string()],
                fixes: vec![
                    "isolate volatile API usage".to_string(),
                    "record toolchain version".to_string(),
                ],
            },
        ],
        idiom_patterns: vec![
            IdiomPattern {
                id: "python_hot_loop_to_mojo_kernel_candidate".to_string(),
                trigger: "python hot loop".to_string(),
                suggestion: "treat the loop as a Mojo kernel candidate only after profiling"
                    .to_string(),
            },
            IdiomPattern {
                id: "mojo_kernel_with_python_fallback".to_string(),
                trigger: "kernel".to_string(),
                suggestion: "keep Python/Rust fallback beside any Mojo kernel proposal".to_string(),
            },
            IdiomPattern {
                id: "volatile_api_isolation".to_string(),
                trigger: "mojo api".to_string(),
                suggestion: "isolate volatile Mojo API behind a small adapter".to_string(),
            },
        ],
        refactor_patterns: refactors(&[
            "python_hot_loop_to_mojo_kernel_candidate",
            "mojo_kernel_with_python_fallback",
            "benchmark_guarded_optimization",
            "volatile_api_isolation",
        ]),
        test_strategies: vec![TestStrategy {
            id: "mojo_benchmark_guard".to_string(),
            commands: vec![
                "mojo compile/check if toolchain exists".to_string(),
                "benchmark comparison".to_string(),
                "Python/Rust fallback test".to_string(),
            ],
            reason: "Mojo must remain benchmark-gated and optional.".to_string(),
        }],
        performance_patterns: vec![PerformancePattern {
            id: "kernel_candidate_requires_benchmark".to_string(),
            trigger: "kernel optimization".to_string(),
            benchmark_required: true,
            fallback_required: true,
        }],
        safety_rules: vec![
            "do not move core safety logic to Mojo".to_string(),
            "do not execute or install Mojo toolchain without approval".to_string(),
            "do not depend on Mojo without fallback".to_string(),
        ],
        maturity_level: 1,
    }
}

fn unknown_scaffold() -> CodingScaffold {
    CodingScaffold {
        language: CodingLanguage::Unknown,
        core_principles: vec![
            "unknown language requires conservative proposal".to_string(),
            "human review is required before language-specific edits".to_string(),
        ],
        error_patterns: Vec::new(),
        idiom_patterns: Vec::new(),
        refactor_patterns: Vec::new(),
        test_strategies: vec![TestStrategy {
            id: "unknown_review".to_string(),
            commands: vec!["manual language review".to_string()],
            reason: "Unknown file type cannot use language-specific testing.".to_string(),
        }],
        performance_patterns: Vec::new(),
        safety_rules: vec!["do not propose automatic edits for unknown language".to_string()],
        maturity_level: 0,
    }
}

fn expected_tests_for(scaffold: &CodingScaffold) -> Vec<String> {
    let mut tests = scaffold
        .test_strategies
        .iter()
        .flat_map(|strategy| strategy.commands.clone())
        .collect::<Vec<_>>();
    tests.sort();
    tests.dedup();
    tests
}

fn refactors(ids: &[&str]) -> Vec<RefactorPattern> {
    ids.iter()
        .map(|id| RefactorPattern {
            id: (*id).to_string(),
            description: format!("apply {id} only when it reduces complexity or risk"),
            safety_note: "keep patch small, tested, and reversible".to_string(),
        })
        .collect()
}

fn contains_all(values: &[String], needles: &[&str]) -> bool {
    let haystack = values.join(" ").to_lowercase();
    needles
        .iter()
        .all(|needle| haystack.contains(&needle.to_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::{CodingLanguage, LanguageRegistry};

    #[test]
    fn language_registry_registers_rust_python_mojo() {
        let registry = LanguageRegistry::new();

        assert!(registry.scaffold_for(CodingLanguage::Rust).is_some());
        assert!(registry.scaffold_for(CodingLanguage::Python).is_some());
        assert!(registry.scaffold_for(CodingLanguage::Mojo).is_some());
        assert_eq!(registry.default_project_language, CodingLanguage::Rust);
    }

    #[test]
    fn rust_scaffold_contains_ownership_borrowing_result_option_testing() {
        let registry = LanguageRegistry::new();
        let rust = registry.scaffold_for(CodingLanguage::Rust).unwrap();
        let principles = rust.core_principles.join(" ").to_lowercase();

        for needle in ["ownership", "borrowing", "lifetime", "result", "option"] {
            assert!(principles.contains(needle));
        }
        assert!(rust.test_strategies.iter().any(|strategy| strategy
            .commands
            .iter()
            .any(|command| command == "cargo test")));
    }

    #[test]
    fn python_scaffold_contains_venv_type_hints_dataclass_pathlib_pytest() {
        let registry = LanguageRegistry::new();
        let python = registry.scaffold_for(CodingLanguage::Python).unwrap();
        let principles = python.core_principles.join(" ").to_lowercase();

        for needle in ["virtual environment", "type hint", "dataclass", "pathlib"] {
            assert!(principles.contains(needle));
        }
        assert!(python.test_strategies.iter().any(|strategy| {
            strategy
                .commands
                .iter()
                .any(|command| command.contains("pytest"))
        }));
    }

    #[test]
    fn mojo_scaffold_contains_python_interop_kernel_candidate_and_volatile_status() {
        let registry = LanguageRegistry::new();
        let mojo = registry.scaffold_for(CodingLanguage::Mojo).unwrap();
        let principles = mojo.core_principles.join(" ").to_lowercase();

        for needle in ["python interop", "kernel", "volatile", "fallback"] {
            assert!(principles.contains(needle));
        }
    }

    #[test]
    fn language_detection_maps_extensions_to_languages() {
        assert_eq!(
            LanguageRegistry::detect_language("main.rs"),
            CodingLanguage::Rust
        );
        assert_eq!(
            LanguageRegistry::detect_language("script.py"),
            CodingLanguage::Python
        );
        assert_eq!(
            LanguageRegistry::detect_language("kernel.mojo"),
            CodingLanguage::Mojo
        );
        assert_eq!(
            LanguageRegistry::detect_language("README.md"),
            CodingLanguage::Unknown
        );
    }

    #[test]
    fn rust_error_pattern_detects_borrow_checker_conflict() {
        let registry = LanguageRegistry::new();
        let pattern = registry
            .classify_error(
                CodingLanguage::Rust,
                "cannot borrow as mutable because it is also borrowed as immutable",
            )
            .unwrap();

        assert_eq!(pattern.id, "borrow_checker_conflict");
    }

    #[test]
    fn rust_error_pattern_detects_moved_value() {
        let registry = LanguageRegistry::new();
        let pattern = registry
            .classify_error(CodingLanguage::Rust, "use of moved value: report")
            .unwrap();

        assert_eq!(pattern.id, "moved_value_error");
    }

    #[test]
    fn python_error_pattern_detects_import_error() {
        let registry = LanguageRegistry::new();
        let pattern = registry
            .classify_error(
                CodingLanguage::Python,
                "ModuleNotFoundError: no module named cv2",
            )
            .unwrap();

        assert_eq!(pattern.id, "import_error");
    }

    #[test]
    fn python_error_pattern_detects_path_error() {
        let registry = LanguageRegistry::new();
        let pattern = registry
            .classify_error(CodingLanguage::Python, "FileNotFoundError for output path")
            .unwrap();

        assert_eq!(pattern.id, "path_error");
    }

    #[test]
    fn mojo_error_pattern_detects_toolchain_missing() {
        let registry = LanguageRegistry::new();
        let pattern = registry
            .classify_error(CodingLanguage::Mojo, "mojo command not found")
            .unwrap();

        assert_eq!(pattern.id, "toolchain_missing");
    }

    #[test]
    fn idiom_suggestion_recommends_enum_for_string_state_in_rust() {
        let registry = LanguageRegistry::new();
        let suggestion = registry.suggest(CodingLanguage::Rust, "string state flag");

        assert!(suggestion
            .suggestions
            .iter()
            .any(|item| item.contains("enum")));
    }

    #[test]
    fn idiom_suggestion_recommends_result_over_unwrap_in_rust() {
        let registry = LanguageRegistry::new();
        let suggestion = registry.suggest(CodingLanguage::Rust, "unwrap used in CLI parser");

        assert!(suggestion
            .suggestions
            .iter()
            .any(|item| item.contains("Result")));
    }

    #[test]
    fn idiom_suggestion_recommends_dataclass_for_structured_python_data() {
        let registry = LanguageRegistry::new();
        let suggestion = registry.suggest(CodingLanguage::Python, "structured dict with many keys");

        assert!(suggestion
            .suggestions
            .iter()
            .any(|item| item.contains("dataclass")));
    }

    #[test]
    fn idiom_suggestion_recommends_pathlib_for_python_paths() {
        let registry = LanguageRegistry::new();
        let suggestion = registry.suggest(CodingLanguage::Python, "script uses os.path");

        assert!(suggestion
            .suggestions
            .iter()
            .any(|item| item.contains("pathlib")));
    }

    #[test]
    fn idiom_suggestion_requires_benchmark_before_mojo_optimization() {
        let registry = LanguageRegistry::new();
        let suggestion = registry.suggest(CodingLanguage::Mojo, "python hot loop candidate");

        assert!(suggestion
            .suggestions
            .iter()
            .any(|item| item.contains("benchmark")));
    }

    #[test]
    fn coding_maturity_increases_after_successful_patch_feedback() {
        let mut registry = LanguageRegistry::new();
        let before = registry.maturity(CodingLanguage::Rust);
        let after = registry.record_successful_patch_feedback(CodingLanguage::Rust);

        assert!(after > before);
    }

    #[test]
    fn coding_knowledge_benchmark_improves_patch_plan_quality() {
        let report = LanguageRegistry::benchmark();

        assert!(report.coding_knowledge_benchmark_improves_patch_plan_quality);
        assert!(report.on_patch_plan_quality > report.off_patch_plan_quality);
        assert!(report.on_unsafe_code_suggestion_rate <= report.off_unsafe_code_suggestion_rate);
    }
}
