use crate::error::Result;
use crate::types::fraction::Fraction;
use crate::types::{DisplayHint, FlowToken, SemanticRegistry, Stack, Token, Value, WordDefinition};
use smallvec::SmallVec;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub const DEFAULT_MAX_EXECUTION_STEPS: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OperationTargetMode {
    StackTop,
    Stack,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConsumptionMode {
    Consume,
    Keep,
}

#[derive(Debug, Clone)]
pub(crate) struct UserDictionary {
    pub order: u64,
    pub words: HashMap<String, Arc<WordDefinition>>,
}

#[derive(Debug, Clone)]
pub(crate) struct ModuleDictionary {
    pub words: HashMap<String, Arc<WordDefinition>>,
    pub sample_words: HashMap<String, Arc<WordDefinition>>,
}

#[derive(Debug, Clone)]
pub(crate) struct ImportedModule {
    pub module_name: String,
    pub import_all_public: bool,
    pub imported_words: HashSet<String>,
    pub imported_samples: HashSet<String>,
    pub imported_at: u64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ImportTable {
    pub modules: HashMap<String, ImportedModule>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct DictionaryDependencyInfo {
    pub depends_on: HashSet<String>,
    pub depended_by: HashSet<String>,
}

#[derive(Debug, Clone)]
pub(crate) enum ChildState {
    Running,
    Completed,
    Failed,
    Killed,
    Timeout,
}

#[derive(Debug, Clone)]
pub(crate) enum ExitReason {
    Normal,
    Error(String),
    Killed,
    Timeout,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeDictionarySnapshot {
    pub user_words: HashMap<String, Arc<WordDefinition>>,
    pub user_dictionaries: HashMap<String, UserDictionary>,
    pub dependents: HashMap<String, HashSet<String>>,
    pub import_table: ImportTable,
    pub module_vocabulary: HashMap<String, ModuleDictionary>,
    pub dictionary_dependencies: HashMap<String, DictionaryDependencyInfo>,
    pub next_registration_order: u64,
    pub active_user_dictionary: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ChildRuntime {
    pub id: u64,
    pub code_block: Vec<Token>,
    pub dictionary_snapshot: RuntimeDictionarySnapshot,
    pub state: ChildState,
    pub exit_reason: Option<ExitReason>,
    pub result_snapshot: Option<Vec<Value>>,
    pub restart_count: usize,
    pub supervisor_id: Option<u64>,
    pub monitored: bool,
}

pub struct Interpreter {
    pub(crate) stack: Stack,
    pub(crate) core_vocabulary: HashMap<String, Arc<WordDefinition>>,
    pub(crate) user_words: HashMap<String, Arc<WordDefinition>>,
    pub(crate) user_dictionaries: HashMap<String, UserDictionary>,
    pub(crate) dependents: HashMap<String, HashSet<String>>,
    pub(crate) output_buffer: String,
    pub(crate) definition_to_load: Option<String>,
    pub(crate) operation_target_mode: OperationTargetMode,
    pub(crate) consumption_mode: ConsumptionMode,
    pub(crate) force_flag: bool,
    pub(crate) disable_no_change_check: bool,
    pub(crate) safe_mode: bool,
    pub(crate) pending_tokens: Option<Vec<Token>>,
    pub(crate) pending_token_index: usize,
    pub(crate) module_state: HashMap<String, Box<dyn std::any::Any>>,
    pub(crate) import_table: ImportTable,
    pub(crate) call_stack: SmallVec<[String; 5]>,
    pub(crate) execution_step_count: usize,
    pub(crate) max_execution_steps: usize,
    pub(crate) input_buffer: String,
    pub(crate) io_output_buffer: String,



    pub gui_mode: bool,

    pub(crate) flow_tracking: bool,
    pub(crate) active_flows: Vec<FlowToken>,
    pub(crate) flow_consumed_log: Vec<(u64, Fraction)>,

    pub(crate) module_vocabulary: HashMap<String, ModuleDictionary>,
    pub(crate) dictionary_dependencies: HashMap<String, DictionaryDependencyInfo>,
    pub(crate) next_registration_order: u64,
    pub(crate) active_user_dictionary: String,

    pub(crate) semantic_registry: SemanticRegistry,

    pub(crate) child_runtimes: HashMap<u64, ChildRuntime>,
    pub(crate) next_child_id: u64,
    pub(crate) monitor_notifications: Vec<Vec<Value>>,
    pub(crate) next_supervisor_id: u64,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Interpreter {
            stack: Vec::new(),
            core_vocabulary: HashMap::new(),
            user_words: HashMap::new(),
            user_dictionaries: HashMap::new(),
            dependents: HashMap::new(),
            output_buffer: String::new(),
            definition_to_load: None,
            operation_target_mode: OperationTargetMode::StackTop,
            consumption_mode: ConsumptionMode::Consume,
            force_flag: false,
            disable_no_change_check: true,
            safe_mode: false,
            pending_tokens: None,
            pending_token_index: 0,
            module_state: HashMap::new(),
            import_table: ImportTable::default(),
            call_stack: SmallVec::new(),
            execution_step_count: 0,
            max_execution_steps: DEFAULT_MAX_EXECUTION_STEPS,
            input_buffer: String::new(),
            io_output_buffer: String::new(),
            gui_mode: false,
            flow_tracking: false,
            active_flows: Vec::new(),
            flow_consumed_log: Vec::new(),
            module_vocabulary: HashMap::new(),
            dictionary_dependencies: HashMap::new(),
            next_registration_order: 1,
            active_user_dictionary: "DEMO".to_string(),
            semantic_registry: SemanticRegistry::new(),
            child_runtimes: HashMap::new(),
            next_child_id: 1,
            monitor_notifications: Vec::new(),
            next_supervisor_id: 1,
        };
        crate::builtins::register_builtins(&mut interpreter.core_vocabulary);
        interpreter
    }



    /// Internal diagnostic toggle for fractional-flow invariant tracking.
    ///
    /// This is not part of user-visible language semantics.
    pub fn update_flow_tracking(&mut self, enabled: bool) {
        self.flow_tracking = enabled;
        if enabled {
            self.active_flows.clear();
            self.flow_consumed_log.clear();
        }
    }

    pub fn begin_flow(&mut self, value: &Value) -> FlowToken {
        let token = FlowToken::from_value(value);
        if self.flow_tracking {
            self.active_flows.push(token.clone());
        }
        token
    }

    pub fn record_consumption(
        &mut self,
        flow: &FlowToken,
        consumed: &Fraction,
    ) -> Result<FlowToken> {
        let (_consumed_amount, new_flow) = flow.consume(consumed)?;
        if self.flow_tracking {
            self.flow_consumed_log.push((flow.id, consumed.abs()));
            if let Some(af) = self.active_flows.iter_mut().find(|f| f.id == flow.id) {
                *af = new_flow.clone();
            }
        }
        Ok(new_flow)
    }

    /// Internal invariant verification across tracked flows.
    pub fn verify_all_flows(&self) -> Result<()> {
        for flow in &self.active_flows {
            let consumed_for_flow: Vec<Fraction> = self
                .flow_consumed_log
                .iter()
                .filter(|(id, _)| *id == flow.id)
                .map(|(_, c)| c.clone())
                .collect();
            flow.verify_conservation(&consumed_for_flow)?;
        }
        Ok(())
    }

    /// Internal leak check for tracked flows at pipeline boundaries.
    pub fn assert_all_flows_complete(&self) -> Result<()> {
        for flow in &self.active_flows {
            flow.assert_complete("pipeline end")?;
        }
        Ok(())
    }

    pub fn record_bifurcation(&mut self, flow: &FlowToken, n: usize) -> Result<Vec<FlowToken>> {
        let (updated_parent, children) = flow.bifurcate(n)?;
        if self.flow_tracking {
            if let Some(af) = self.active_flows.iter_mut().find(|f| f.id == flow.id) {
                *af = updated_parent;
            }
            for child in &children {
                self.active_flows.push(child.clone());
            }
        }
        Ok(children)
    }



    pub(crate) fn update_operation_target_mode(&mut self, mode: OperationTargetMode) {
        self.operation_target_mode = mode;
    }

    pub(crate) fn update_consumption_mode(&mut self, mode: ConsumptionMode) {
        self.consumption_mode = mode;
    }

    pub(crate) fn reset_execution_modes(&mut self) {
        self.operation_target_mode = OperationTargetMode::StackTop;
        self.consumption_mode = ConsumptionMode::Consume;
        self.safe_mode = false;
    }

    pub(crate) fn normalize_symbol<'a>(symbol: &'a str) -> std::borrow::Cow<'a, str> {
        match symbol {
            "%" => std::borrow::Cow::Borrowed("MOD"),
            "&" => std::borrow::Cow::Borrowed("AND"),
            _ => {
                if symbol.as_bytes().iter().any(|b| b.is_ascii_lowercase()) {
                    std::borrow::Cow::Owned(symbol.to_uppercase())
                } else {
                    std::borrow::Cow::Borrowed(symbol)
                }
            }
        }
    }

    pub(crate) fn next_registration_order(&mut self) -> u64 {
        let order = self.next_registration_order;
        self.next_registration_order += 1;
        order
    }



    pub fn execute_reset(&mut self) -> Result<()> {
        self.stack.clear();
        self.core_vocabulary.clear();
        self.user_words.clear();
        self.user_dictionaries.clear();
        self.dependents.clear();
        self.output_buffer.clear();
        self.definition_to_load = None;
        self.reset_execution_modes();
        self.force_flag = false;
        self.pending_tokens = None;
        self.pending_token_index = 0;
        self.module_state.clear();
        self.call_stack.clear();
        self.import_table.modules.clear();
        self.module_vocabulary.clear();
        self.dictionary_dependencies.clear();
        self.next_registration_order = 1;
        self.active_user_dictionary = "DEMO".to_string();
        self.semantic_registry.clear();
        self.child_runtimes.clear();
        self.next_child_id = 1;
        self.monitor_notifications.clear();
        self.next_supervisor_id = 1;
        crate::builtins::register_builtins(&mut self.core_vocabulary);
        Ok(())
    }

    pub fn collect_output(&mut self) -> String {
        std::mem::take(&mut self.output_buffer)
    }

    pub fn get_stack(&self) -> &Stack {
        &self.stack
    }

    pub fn update_stack(&mut self, stack: Stack) {
        self.stack = stack;
        self.semantic_registry
            .normalize_to_stack_len(self.stack.len());
    }

    pub fn update_stack_with_hints(&mut self, stack: Stack, hints: Vec<DisplayHint>) {
        self.stack = stack;
        self.semantic_registry.stack_hints = hints;
        self.semantic_registry
            .normalize_to_stack_len(self.stack.len());
    }

    pub fn collect_stack_hints(&self) -> &[DisplayHint] {
        &self.semantic_registry.stack_hints
    }
}
