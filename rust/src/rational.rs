use num_rational::BigRational;
use num_traits::Zero; // <--- `Num` を削除
use std::fmt;
use std::str::FromStr;

/// Ajisai内部で使用する分数型（`BigRational`のラッパー）
/// 将来的により詳細なエラーハンドリングやカスタムフォーマットを実装可能
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Rational {
    val: BigRational,
}

impl Rational {
    /// 新しいRationalを生成します (a/b)
    pub fn new(a: i64, b: i64) -> Self {
        Rational {
            val: BigRational::new(a.into(), b.into()),
        }
    }

    /// ゼロ除算をチェックしつつ除算を行います
    pub fn div(&self, other: &Self) -> Option<Self> {
        if other.val.is_zero() {
            None // ゼロ除算
        } else {
            Some(Rational {
                val: self.val.clone() / other.val.clone(),
            })
        }
    }

    // --- ラッパーメソッド ---
    pub fn add(&self, other: &Self) -> Self {
        Rational {
            val: self.val.clone() + other.val.clone(),
        }
    }
    pub fn sub(&self, other: &Self) -> Self {
        Rational {
            val: self.val.clone() - other.val.clone(),
        }
    }
    pub fn mul(&self, other: &Self) -> Self {
        Rational {
            val: self.val.clone() * other.val.clone(),
        }
    }
    pub fn is_zero(&self) -> bool {
        self.val.is_zero()
    }
}

/// 文字列からのパース ("1/3", "10", "0.5")
impl FromStr for Rational {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // "0.5" のような小数を分数に変換
        if s.contains('.') {
            BigRational::from_str(s)
                .map(|val| Rational { val })
                .map_err(|e| e.to_string())
        } else {
            // "1/3" または "10"
            BigRational::from_str(s)
                .map(|val| Rational { val })
                .map_err(|e| e.to_string())
        }
    }
}

/// 文字列へのフォーマット
impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.val.is_integer() {
            write!(f, "{}", self.val.numer())
        } else {
            write!(f, "{}", self.val)
        }
    }
}

impl Zero for Rational {
    fn zero() -> Self {
        Rational {
            val: BigRational::zero(),
        }
    }
    fn is_zero(&self) -> bool {
        self.val.is_zero()
    }
}
