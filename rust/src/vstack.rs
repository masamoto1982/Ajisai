use crate::rational::Rational;
use std::fmt;
use tauri::AppHandle;

/// VStack は Operand の実体であり、Rationalの動的配列（Vector）です。
#[derive(Debug, Clone)]
pub struct VStack {
    data: Vec<Rational>,
    // GUI通知用のTauri AppHandle
    app_handle: Option<AppHandle>,
    name: String, // 自分がどのOperandかを知っておく
}

impl VStack {
    /// 新しいVStackを作成します。
    pub fn new(name: String, app_handle: Option<AppHandle>) -> Self {
        VStack {
            data: Vec::new(),
            app_handle,
            name,
        }
    }

    /// GUIに状態変更を通知します
    fn notify(&self) {
        if let Some(handle) = &self.app_handle {
            // "operand-updated" イベントを発行し、更新されたVStackの情報を送る
            handle.emit_all("operand-updated", 
                (self.name.clone(), self.to_string())
            ).unwrap_or_else(|e| {
                eprintln!("Failed to emit GUI update event: {}", e);
            });
        }
    }

    /// VStackの内容を新しいスライスで上書きします。
    pub fn set(&mut self, new_data: Vec<Rational>) {
        self.data = new_data;
        self.notify();
    }

    /// 末尾に要素を追加します（スタック・モード用）。
    pub fn push(&mut self, r: Rational) {
        self.data.push(r);
        self.notify();
    }

    /// 末尾から要素を削除し、返します（スタック・モード用）。
    pub fn pop(&mut self) -> Option<Rational> {
        let result = self.data.pop();
        if result.is_some() {
            self.notify();
        }
        result
    }

    /// VStackの長さを返します。
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// VStackが空かどうかを返します。
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// VStackの現在のデータの（シャロー）コピーを返します（ベクター・モード用）。
    /// Rational自体がCloneなため、実質ディープコピーのように振る舞えます。
    pub fn get_copy(&self) -> Vec<Rational> {
        self.data.clone()
    }

    /// VStackのデータをソートします（ベクター・モード用）。
    pub fn sort(&mut self) {
        self.data.sort(); // Rational が Ord を実装しているため
        self.notify();
    }
}

/// 文字列へのフォーマット "[1/2, 1, 1/3]"
impl fmt::Display for VStack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[")?;
        for (i, r) in self.data.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{}", r)?;
        }
        write!(f, "]")
    }
}
