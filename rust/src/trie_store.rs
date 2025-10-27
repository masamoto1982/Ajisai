use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// トライ木のノード
// ★ 修正点: `Default` を derive から外す
#[derive(Debug)]
struct TrieNode<T> {
    children: HashMap<char, TrieNode<T>>,
    value: Option<T>,
}

// ★ 修正点: `TrieNode` の `Default` を手動で実装
// (T への Default 制約がなくなります)
impl<T> Default for TrieNode<T> {
    fn default() -> Self {
        TrieNode {
            children: HashMap::new(),
            value: None,
        }
    }
}

/// トライ木ベースのストア（辞書）
#[derive(Debug)]
pub struct TrieStore<T> {
    root: TrieNode<T>,
}

// ★ 修正点: `T: Default` の制約を外す
impl<T: Clone> TrieStore<T> {
    /// 新しいTrieStoreを作成します
    pub fn new() -> Self {
        TrieStore {
            root: TrieNode::default(), // これでOK
        }
    }

    /// 値をTrieに挿入します
    pub fn insert(&mut self, key: &str, value: T) {
        let mut node = &mut self.root;
        for c in key.chars() {
            // ★ `TrieNode<T>: Default` が制約なしになったため、これもOK
            node = node.children.entry(c).or_default();
        }
        node.value = Some(value);
    }

    /// キーで値を検索します
    pub fn find(&self, key: &str) -> Option<T> {
        let mut node = &self.root;
        for c in key.chars() {
            match node.children.get(&c) {
                Some(n) => node = n,
                None => return None,
            }
        }
        node.value.clone()
    }

    /// すべての値を（ソートせずに）取得します (GUI用)
    pub fn get_all_values(&self) -> Vec<T> {
        let mut values = Vec::new();
        self.traverse_collect(&self.root, &mut values);
        values
    }

    /// ノードを再帰的に辿って値を収集します
    fn traverse_collect(&self, node: &TrieNode<T>, values: &mut Vec<T>) {
        if let Some(value) = &node.value {
            values.push(value.clone());
        }
        for child in node.children.values() {
            self.traverse_collect(child, values);
        }
    }

    /// (参考) キーと値のペアをすべて取得
    pub fn get_all_pairs(&self) -> Vec<(String, T)> {
        let mut pairs = Vec::new();
        self.traverse_pairs(&self.root, String::new(), &mut pairs);
        pairs.sort_by(|a, b| a.0.cmp(&b.0)); // 名前順にソート
        pairs
    }

    fn traverse_pairs(&self, node: &TrieNode<T>, prefix: String, pairs: &mut Vec<(String, T)>) {
        if let Some(value) = &node.value {
            pairs.push((prefix.clone(), value.clone()));
        }
        for (c, child) in &node.children {
            self.traverse_pairs(child, format!("{}{}", prefix, c), pairs);
        }
    }
}

// Storeはスレッドセーフである必要があるため、RwLockでラップします
pub type SharedTrieStore<T> = Arc<RwLock<TrieStore<T>>>;

// Arc<RwLock<TrieStore<T>>> を簡単に作成するためのヘルパー
pub fn new_shared_store<T: Clone>() -> SharedTrieStore<T> {
    Arc::new(RwLock::new(TrieStore::new()))
}
