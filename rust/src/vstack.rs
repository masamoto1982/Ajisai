use crate::rational::Rational;
use std::fmt;

/// VStack は Operand の実体であり、Rationalの動的配列（Vector）です。
#[derive(Debug, Clone, Default)]
pub struct VStack {
    data: Vec<Rational>,
    // GUI通知用のTauri AppHandle は削除
    // name: String, (VStack自身が名前を持つ必要がなくなったため削除)
}

impl VStack {
    /// 新しいVStackを作成します。
    pub fn new() -> Self {
        VStack { data: Vec::new() }
    }

    // notify() 関数は削除

    /// VStackの内容を新しいスライスで上書きします。
    pub fn set(&mut self, new_data: Vec<Rational>) {
        self.data = new_data;
        // notify() 削除
    }

    /// 末尾に要素を追加します（スタック・モード用）。
    pub fn push(&mut self, r: Rational) {
        self.data.push(r);
        // notify() 削除
    }

    /// 末尾から要素を削除し、返します（スタック・モード用）。
    pub fn pop(&mut self) -> Option<Rational> {
        let result = self.data.pop();
        // notify() 削除
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
    pub fn get_copy(&self) -> Vec<Rational> {
        self.data.clone()
    }

    /// VStackのデータをソートします（ベクター・モード用）。
    pub fn sort(&mut self) {
        self.data.sort(); // Rational が Ord を実装しているため
        // notify() 削除
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
