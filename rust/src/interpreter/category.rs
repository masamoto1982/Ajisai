// rust/src/interpreter/category.rs
//!
//! 圏論的操作ワードの実装
//!
//! このモジュールはAjisaiを圏論的プログラミング言語として
//! 特徴付ける中核機能を提供する。
//!
//! ## 実装する圏構造
//!
//! ### Vect圏（有限次元ベクトル空間の圏）
//! - 対象: 自然数 n（Rⁿを表す）
//! - 射: n×m行列（Rᵐ → Rⁿの線形写像）
//! - 合成: 行列積
//! - 恒等射: 単位行列
//!
//! ### モノイダル構造
//! - テンソル積: クロネッカー積
//! - 単位対象: 1（スカラー）
//!
//! ## 主要ワード
//!
//! | ワード | スタック効果 | 説明 |
//! |--------|-------------|------|
//! | COMPOSE | (f g -- g∘f) | 射の合成（行列積） |
//! | THEN | (f g -- f;g) | 射の順次合成 |
//! | ID | (n -- I_n) | 恒等射（単位行列） |
//! | KRON | (A B -- A⊗B) | テンソル積（クロネッカー積） |
//! | OUTER | (v w -- v⊗w) | 外積（ベクトルのテンソル積） |
//! | CONTRACT | (T indices -- T') | テンソル縮約 |
//! | DOM | (f -- m) | 射の定義域の次元 |
//! | COD | (f -- n) | 射の値域の次元 |
//!

use crate::interpreter::Interpreter;
use crate::types::tensor::Tensor;
use crate::types::fraction::Fraction;
use crate::error::{AjisaiError, Result};
use num_traits::{ToPrimitive, One, Zero};
use num_bigint::BigInt;

impl Interpreter {
    /// COMPOSE: 射の合成（数学的順序 g ∘ f）
    /// スタック: (f g -- g∘f)
    /// 行列として: g @ f （Pythonのnumpy記法）
    ///
    /// 圏論的には g ∘ f を計算する。
    /// 行列として: result = g @ f
    ///
    /// # 型チェック
    /// f: m → k (k×m行列)
    /// g: k → n (n×k行列)
    /// g ∘ f: m → n (n×m行列)
    ///
    /// f の値域次元 = g の定義域次元 でなければエラー
    pub fn builtin_compose(&mut self) -> Result<()> {
        let g = self.pop_tensor()?;
        let f = self.pop_tensor()?;

        // 形状チェック
        let f_shape = f.shape();
        let g_shape = g.shape();

        if f_shape.len() != 2 || g_shape.len() != 2 {
            return Err(AjisaiError::from(
                "COMPOSE requires two 2D tensors (matrices)"
            ));
        }

        // f: k×m行列, g: n×k行列
        // f の行数(k) と g の列数(k) が一致する必要
        let f_rows = f_shape[0]; // k
        let f_cols = f_shape[1]; // m
        let g_rows = g_shape[0]; // n
        let g_cols = g_shape[1]; // k'

        if f_rows != g_cols {
            return Err(AjisaiError::from(format!(
                "COMPOSE: codomain of f ({}) != domain of g ({})",
                f_rows, g_cols
            )));
        }

        // 行列積の計算: result[i][j] = Σ_k g[i][k] * f[k][j]
        let result = self.matmul_tensors(&g, &f)?;

        self.push_tensor(result);
        Ok(())
    }

    /// THEN: 射の順次合成（図式的順序 f ; g）
    /// スタック: (f g -- f;g)
    /// 行列として: g @ f と同じ結果だが、思考の流れが異なる
    pub fn builtin_then(&mut self) -> Result<()> {
        // COMPOSE と同じ実装だが、引数の順序の意図が異なる
        // ドキュメント上の区別
        self.builtin_compose()
    }

    /// ID: 恒等射（単位行列）
    /// スタック: (n -- I_n)
    pub fn builtin_id(&mut self) -> Result<()> {
        let n = self.pop_scalar_usize()?;

        let mut data = vec![Fraction::new(BigInt::zero(), BigInt::one()); n * n];
        for i in 0..n {
            data[i * n + i] = Fraction::new(BigInt::one(), BigInt::one());
        }

        let tensor = Tensor::new(vec![n, n], data)?;
        self.push_tensor(tensor);
        Ok(())
    }

    /// KRON: クロネッカー積（テンソル積）
    /// スタック: (A B -- A⊗B)
    ///
    /// A が m×n, B が p×q なら結果は (m*p)×(n*q)
    ///
    /// (A ⊗ B)[i*p + k][j*q + l] = A[i][j] * B[k][l]
    pub fn builtin_kron(&mut self) -> Result<()> {
        let b = self.pop_tensor()?;
        let a = self.pop_tensor()?;

        let a_shape = a.shape();
        let b_shape = b.shape();

        if a_shape.len() != 2 || b_shape.len() != 2 {
            return Err(AjisaiError::from(
                "KRON requires two 2D tensors (matrices)"
            ));
        }

        let m = a_shape[0];
        let n = a_shape[1];
        let p = b_shape[0];
        let q = b_shape[1];

        let result_rows = m * p;
        let result_cols = n * q;

        let a_data = a.data();
        let b_data = b.data();

        let mut result_data = vec![Fraction::new(BigInt::zero(), BigInt::one()); result_rows * result_cols];

        for i in 0..m {
            for j in 0..n {
                let a_val = &a_data[i * n + j];
                for k in 0..p {
                    for l in 0..q {
                        let b_val = &b_data[k * q + l];
                        let row = i * p + k;
                        let col = j * q + l;
                        result_data[row * result_cols + col] = a_val.mul(b_val);
                    }
                }
            }
        }

        let tensor = Tensor::new(vec![result_rows, result_cols], result_data)?;
        self.push_tensor(tensor);
        Ok(())
    }

    /// OUTER: ベクトルの外積（テンソル積）
    /// スタック: (v w -- v⊗w)
    ///
    /// v が長さm, w が長さn なら結果は m×n 行列
    /// result[i][j] = v[i] * w[j]
    pub fn builtin_outer(&mut self) -> Result<()> {
        let w = self.pop_tensor()?;
        let v = self.pop_tensor()?;

        let v_shape = v.shape();
        let w_shape = w.shape();

        // 1次元テンソル（ベクトル）を期待
        if v_shape.len() != 1 || w_shape.len() != 1 {
            return Err(AjisaiError::from(
                "OUTER requires two 1D tensors (vectors)"
            ));
        }

        let m = v_shape[0];
        let n = w_shape[0];

        let v_data = v.data();
        let w_data = w.data();

        let mut result_data = Vec::with_capacity(m * n);

        for i in 0..m {
            for j in 0..n {
                result_data.push(v_data[i].mul(&w_data[j]));
            }
        }

        let tensor = Tensor::new(vec![m, n], result_data)?;
        self.push_tensor(tensor);
        Ok(())
    }

    /// CONTRACT: テンソル縮約
    /// スタック: (T indices -- T')
    ///
    /// 指定された軸ペアで縮約を行う
    /// indices は [[axis1, axis2], ...] の形式
    ///
    /// 例: 行列のトレース
    /// [ [ 1 2 ] [ 3 4 ] ] [ [ 0 1 ] ] CONTRACT
    /// → [ 5 ] (1 + 4)
    pub fn builtin_contract(&mut self) -> Result<()> {
        let _indices = self.pop_tensor()?;
        let tensor = self.pop_tensor()?;

        // indices の解析
        // 簡易実装: 2次元テンソルのトレース（対角和）のみサポート
        let t_shape = tensor.shape();

        if t_shape.len() == 2 && t_shape[0] == t_shape[1] {
            // 正方行列のトレース
            let n = t_shape[0];
            let data = tensor.data();
            let mut trace = Fraction::new(BigInt::zero(), BigInt::one());
            for i in 0..n {
                trace = trace.add(&data[i * n + i]);
            }

            let result = Tensor::vector(vec![trace]);
            self.push_tensor(result);
            Ok(())
        } else {
            Err(AjisaiError::from(
                "CONTRACT: currently only supports trace of square matrices"
            ))
        }
    }

    /// DOM: 射の定義域の次元
    /// スタック: (f -- m)
    pub fn builtin_dom(&mut self) -> Result<()> {
        let f = self.pop_tensor()?;
        let shape = f.shape();

        if shape.len() != 2 {
            return Err(AjisaiError::from(
                "DOM requires a 2D tensor (matrix)"
            ));
        }

        // n×m 行列の定義域は m
        let m = shape[1];
        let result = Tensor::vector(vec![Fraction::new(BigInt::from(m as i64), BigInt::one())]);
        self.push_tensor(result);
        Ok(())
    }

    /// COD: 射の値域の次元
    /// スタック: (f -- n)
    pub fn builtin_cod(&mut self) -> Result<()> {
        let f = self.pop_tensor()?;
        let shape = f.shape();

        if shape.len() != 2 {
            return Err(AjisaiError::from(
                "COD requires a 2D tensor (matrix)"
            ));
        }

        // n×m 行列の値域は n
        let n = shape[0];
        let result = Tensor::vector(vec![Fraction::new(BigInt::from(n as i64), BigInt::one())]);
        self.push_tensor(result);
        Ok(())
    }

    // ========================================================================
    // ヘルパーメソッド
    // ========================================================================

    /// 行列積の実装
    fn matmul_tensors(&self, a: &Tensor, b: &Tensor) -> Result<Tensor> {
        let a_shape = a.shape();
        let b_shape = b.shape();

        let m = a_shape[0];  // 結果の行数
        let k = a_shape[1];  // 内部次元
        let n = b_shape[1];  // 結果の列数

        let a_data = a.data();
        let b_data = b.data();

        let mut result_data = Vec::with_capacity(m * n);

        for i in 0..m {
            for j in 0..n {
                let mut sum = Fraction::new(BigInt::zero(), BigInt::one());
                for l in 0..k {
                    let a_val = &a_data[i * k + l];
                    let b_val = &b_data[l * n + j];
                    sum = sum.add(&a_val.mul(b_val));
                }
                result_data.push(sum);
            }
        }

        Tensor::new(vec![m, n], result_data)
    }

    /// スタックトップからTensorをポップ
    fn pop_tensor(&mut self) -> Result<Tensor> {
        if self.stack.is_empty() {
            return Err(AjisaiError::from("Stack underflow"));
        }

        let val = self.stack.pop().unwrap();
        match val.val_type {
            crate::types::ValueType::Tensor(t) => Ok(t),
            _ => Err(AjisaiError::type_error("Tensor", &format!("{:?}", val.val_type)))
        }
    }

    /// TensorをスタックにプッシュKORONEのようになっています
    fn push_tensor(&mut self, tensor: Tensor) {
        use crate::types::{Value, ValueType};
        self.stack.push(Value {
            val_type: ValueType::Tensor(tensor)
        });
    }

    /// スタックトップからスカラーusizeをポップ
    fn pop_scalar_usize(&mut self) -> Result<usize> {
        let tensor = self.pop_tensor()?;

        // スカラーまたは長さ1のベクトルを期待
        if tensor.is_scalar() {
            let scalar = tensor.as_scalar()
                .ok_or_else(|| AjisaiError::from("Expected scalar"))?;

            // 分母が1かどうかチェック（整数であること）
            if scalar.denominator != BigInt::one() {
                return Err(AjisaiError::from("Expected integer"));
            }

            scalar.numerator.to_usize()
                .ok_or_else(|| AjisaiError::from("Integer too large"))
        } else if tensor.shape().len() == 1 && tensor.shape()[0] == 1 {
            // 長さ1のベクトル
            let data = tensor.data();
            let scalar = &data[0];

            // 分母が1かどうかチェック（整数であること）
            if scalar.denominator != BigInt::one() {
                return Err(AjisaiError::from("Expected integer"));
            }

            scalar.numerator.to_usize()
                .ok_or_else(|| AjisaiError::from("Integer too large"))
        } else {
            Err(AjisaiError::from("Expected scalar or single-element vector"))
        }
    }
}
