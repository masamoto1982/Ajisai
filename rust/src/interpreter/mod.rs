use std::collections::{HashMap, HashSet};

pub struct Interpreter {
    pub(crate) stack: Stack,
    pub(crate) dictionary: HashMap<String, WordDefinition>,
    pub(crate) dependencies: HashMap<String, HashSet<String>>, // 追加：依存関係
    pub(crate) output_buffer: String,
    pub(crate) call_stack: Vec<String>,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Interpreter {
            stack: Vec::new(),
            dictionary: HashMap::new(),
            dependencies: HashMap::new(), // 追加
            output_buffer: String::new(),
            call_stack: Vec::new(),
        };
        
        crate::builtins::register_builtins(&mut interpreter.dictionary);
        interpreter
    }

    // ワード定義時に依存関係を記録
    fn handle_def(&mut self) -> Result<()> {
        if self.stack.len() < 2 {
            return Err(error::AjisaiError::from("DEF requires quotation and name"));
        }

        let name_val = self.stack.pop().unwrap();
        let quotation_val = self.stack.pop().unwrap();

        let name = match name_val.val_type {
            ValueType::String(s) => s.to_uppercase(),
            _ => return Err(error::AjisaiError::from("DEF requires string name")),
        };

        let tokens = match quotation_val.val_type {
            ValueType::Quotation(t) => t,
            _ => return Err(error::AjisaiError::from("DEF requires quotation")),
        };

        if let Some(existing) = self.dictionary.get(&name) {
            if existing.is_builtin {
                return Err(error::AjisaiError::from(format!("Cannot redefine builtin word: {}", name)));
            }
        }

        // 依存関係を検出・記録
        let mut dependencies = HashSet::new();
        for token in &tokens {
            if let Token::Symbol(sym) = token {
                if self.dictionary.contains_key(sym) && !self.is_builtin_word(sym) {
                    dependencies.insert(sym.clone());
                    // 逆方向の依存関係も記録
                    self.dependencies.entry(sym.clone())
                        .or_insert_with(HashSet::new)
                        .insert(name.clone());
                }
            }
        }

        self.dictionary.insert(name.clone(), WordDefinition {
            tokens,
            is_builtin: false,
            description: None,
        });

        self.append_output(&format!("Defined: {}\n", name));
        Ok(())
    }

    fn is_builtin_word(&self, name: &str) -> bool {
        self.dictionary.get(name)
            .map(|def| def.is_builtin)
            .unwrap_or(false)
    }

    // 保護されているかチェック
    fn is_protected(&self, name: &str) -> bool {
        self.dependencies.get(name)
            .map(|deps| !deps.is_empty())
            .unwrap_or(false)
    }

    // カスタムワード情報（保護状態付き）
    pub fn get_custom_words_info(&self) -> Vec<(String, Option<String>, bool)> {
        self.dictionary.iter()
            .filter(|(_, def)| !def.is_builtin)
            .map(|(name, def)| {
                let protected = self.is_protected(name);
                (name.clone(), def.description.clone(), protected)
            })
            .collect()
    }
}
