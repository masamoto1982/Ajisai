// rust/src/types/fraction.rs

use num_bigint::BigInt;
use num_traits::{Zero, One, ToPrimitive, Signed};
use num_integer::Integer;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fraction {
    pub numerator: BigInt,
    pub denominator: BigInt,
}

impl Fraction {
    pub fn new(numerator: BigInt, denominator: BigInt) -> Self {
        if denominator.is_zero() { panic!("Division by zero"); }
        let common = numerator.gcd(&denominator);
        let mut num = &numerator / &common;
        let mut den = &denominator / &common;
        if den < BigInt::zero() {
            num = -num;
            den = -den;
        }
        Fraction { numerator: num, denominator: den }
    }
    
    pub fn from_str(s: &str) -> std::result::Result<Self, String> {
        if s.is_empty() { return Err("Empty string".to_string()); }

        if let Some(e_pos) = s.find(|c| c == 'e' || c == 'E') {
            let mantissa_str = &s[..e_pos];
            let exponent_str = &s[e_pos+1..];
            
            let mantissa = Self::from_str(mantissa_str)?;
            let exponent = exponent_str.parse::<i32>().map_err(|e| e.to_string())?;
            
            if exponent >= 0 {
                let power = BigInt::from(10).pow(exponent as u32);
                return Ok(Fraction::new(mantissa.numerator * power, mantissa.denominator));
            } else {
                let power = BigInt::from(10).pow((-exponent) as u32);
                return Ok(Fraction::new(mantissa.numerator, mantissa.denominator * power));
            }
        }
        if let Some(pos) = s.find('/') {
            let num = BigInt::from_str(&s[..pos]).map_err(|e| e.to_string())?;
            let den = BigInt::from_str(&s[pos+1..]).map_err(|e| e.to_string())?;
            Ok(Fraction::new(num, den))
        } else if let Some(dot_pos) = s.find('.') {
            let int_part_str = if s.starts_with('.') { "0" } else { &s[..dot_pos] };
            let frac_part_str = &s[dot_pos+1..];
            if frac_part_str.is_empty() { return Self::from_str(int_part_str); }
            let int_part = BigInt::from_str(int_part_str).map_err(|e| e.to_string())?;
            let frac_num = BigInt::from_str(frac_part_str).map_err(|e| e.to_string())?;
            let frac_den = BigInt::from(10).pow(frac_part_str.len() as u32);
            let total_num = int_part.abs() * &frac_den + frac_num;
            Ok(Fraction::new(if int_part < BigInt::zero() { -total_num } else { total_num }, frac_den))
        } else {
            let num = BigInt::from_str(s).map_err(|e| e.to_string())?;
            Ok(Fraction::new(num, BigInt::one()))
        }
    }

    /// 加算: (a/b) + (c/d)
    /// 共通分母の場合は通分をスキップして高速化
    pub fn add(&self, other: &Fraction) -> Fraction {
        // 整数同士の場合: 分母の乗算とGCDをスキップ
        if self.denominator.is_one() && other.denominator.is_one() {
            return Fraction {
                numerator: &self.numerator + &other.numerator,
                denominator: BigInt::one(),
            };
        }
        // 共通分母の場合: 通分不要
        if self.denominator == other.denominator {
            return Fraction::new(
                &self.numerator + &other.numerator,
                self.denominator.clone(),
            );
        }
        // 一般的な場合
        Fraction::new(
            &self.numerator * &other.denominator + &other.numerator * &self.denominator,
            &self.denominator * &other.denominator,
        )
    }

    /// 減算: (a/b) - (c/d)
    /// 共通分母の場合は通分をスキップして高速化
    pub fn sub(&self, other: &Fraction) -> Fraction {
        // 整数同士の場合: 分母の乗算とGCDをスキップ
        if self.denominator.is_one() && other.denominator.is_one() {
            return Fraction {
                numerator: &self.numerator - &other.numerator,
                denominator: BigInt::one(),
            };
        }
        // 共通分母の場合: 通分不要
        if self.denominator == other.denominator {
            return Fraction::new(
                &self.numerator - &other.numerator,
                self.denominator.clone(),
            );
        }
        // 一般的な場合
        Fraction::new(
            &self.numerator * &other.denominator - &other.numerator * &self.denominator,
            &self.denominator * &other.denominator,
        )
    }

    /// 乗算: (a/b) × (c/d)
    /// 交差簡約（Cross-Cancellation）により、乗算前に約分して高速化
    /// g1 = gcd(a, d), g2 = gcd(c, b) を先に計算し、
    /// (a/g1 × c/g2) / (b/g2 × d/g1) とすることで最終GCDを削減
    pub fn mul(&self, other: &Fraction) -> Fraction {
        // 整数同士の場合: 分母の乗算とGCDをスキップ
        if self.denominator.is_one() && other.denominator.is_one() {
            return Fraction {
                numerator: &self.numerator * &other.numerator,
                denominator: BigInt::one(),
            };
        }
        // どちらかが整数の場合の最適化
        if self.denominator.is_one() {
            // (a/1) × (c/d) = (a×c)/d - aとdで交差簡約
            let g = self.numerator.gcd(&other.denominator);
            let a_reduced = &self.numerator / &g;
            let d_reduced = &other.denominator / &g;
            return Fraction::new(
                a_reduced * &other.numerator,
                d_reduced,
            );
        }
        if other.denominator.is_one() {
            // (a/b) × (c/1) = (a×c)/b - cとbで交差簡約
            let g = other.numerator.gcd(&self.denominator);
            let c_reduced = &other.numerator / &g;
            let b_reduced = &self.denominator / &g;
            return Fraction::new(
                &self.numerator * c_reduced,
                b_reduced,
            );
        }
        // 交差簡約: (a/b) × (c/d)
        // g1 = gcd(a, d), g2 = gcd(c, b)
        let g1 = self.numerator.gcd(&other.denominator);
        let g2 = other.numerator.gcd(&self.denominator);

        let a_reduced = &self.numerator / &g1;
        let d_reduced = &other.denominator / &g1;
        let c_reduced = &other.numerator / &g2;
        let b_reduced = &self.denominator / &g2;

        // 交差簡約後は既に互いに素なので、GCDは1になるはず
        // ただし符号の正規化のためにnewを使用
        Fraction::new(
            a_reduced * c_reduced,
            b_reduced * d_reduced,
        )
    }

    /// 除算: (a/b) ÷ (c/d) = (a/b) × (d/c)
    /// 交差簡約により高速化
    pub fn div(&self, other: &Fraction) -> Fraction {
        if other.numerator.is_zero() {
            panic!("Division by zero");
        }
        // 整数同士の場合
        if self.denominator.is_one() && other.denominator.is_one() {
            return Fraction::new(
                self.numerator.clone(),
                other.numerator.clone(),
            );
        }
        // どちらかが整数の場合の最適化
        if self.denominator.is_one() {
            // (a/1) ÷ (c/d) = (a×d)/c - aとcで交差簡約
            let g = self.numerator.gcd(&other.numerator);
            let a_reduced = &self.numerator / &g;
            let c_reduced = &other.numerator / &g;
            return Fraction::new(
                a_reduced * &other.denominator,
                c_reduced,
            );
        }
        if other.denominator.is_one() {
            // (a/b) ÷ (c/1) = a/(b×c) - aとcで交差簡約
            let g = self.numerator.gcd(&other.numerator);
            let a_reduced = &self.numerator / &g;
            let c_reduced = &other.numerator / &g;
            return Fraction::new(
                a_reduced,
                &self.denominator * c_reduced,
            );
        }
        // 交差簡約: (a/b) ÷ (c/d) = (a×d)/(b×c)
        // g1 = gcd(a, c), g2 = gcd(d, b)
        let g1 = self.numerator.gcd(&other.numerator);
        let g2 = other.denominator.gcd(&self.denominator);

        let a_reduced = &self.numerator / &g1;
        let c_reduced = &other.numerator / &g1;
        let d_reduced = &other.denominator / &g2;
        let b_reduced = &self.denominator / &g2;

        Fraction::new(
            a_reduced * d_reduced,
            b_reduced * c_reduced,
        )
    }
    /// 小なり比較: 整数同士の場合は乗算をスキップ
    pub fn lt(&self, other: &Fraction) -> bool {
        if self.denominator.is_one() && other.denominator.is_one() {
            return self.numerator < other.numerator;
        }
        &self.numerator * &other.denominator < &other.numerator * &self.denominator
    }

    /// 小なりイコール比較: 整数同士の場合は乗算をスキップ
    pub fn le(&self, other: &Fraction) -> bool {
        if self.denominator.is_one() && other.denominator.is_one() {
            return self.numerator <= other.numerator;
        }
        &self.numerator * &other.denominator <= &other.numerator * &self.denominator
    }

    /// 大なり比較: 整数同士の場合は乗算をスキップ
    pub fn gt(&self, other: &Fraction) -> bool {
        if self.denominator.is_one() && other.denominator.is_one() {
            return self.numerator > other.numerator;
        }
        &self.numerator * &other.denominator > &other.numerator * &self.denominator
    }

    /// 大なりイコール比較: 整数同士の場合は乗算をスキップ
    pub fn ge(&self, other: &Fraction) -> bool {
        if self.denominator.is_one() && other.denominator.is_one() {
            return self.numerator >= other.numerator;
        }
        &self.numerator * &other.denominator >= &other.numerator * &self.denominator
    }

    /// 切り捨て（負の無限大方向への丸め）
    /// 整数の場合は自身をそのまま返す（高速化）
    pub fn floor(&self) -> Fraction {
        // 整数の場合はそのまま返す
        if self.denominator.is_one() {
            return self.clone();
        }

        let q = &self.numerator / &self.denominator;
        let r = &self.numerator % &self.denominator;

        // 負の数で余りがある場合は、さらに1を引く（負の無限大方向）
        let floored = if self.numerator < BigInt::zero() && !r.is_zero() {
            q - BigInt::one()
        } else {
            q
        };

        Fraction {
            numerator: floored,
            denominator: BigInt::one(),
        }
    }

    /// 切り上げ（正の無限大方向への丸め）
    /// 整数の場合は自身をそのまま返す（高速化）
    pub fn ceil(&self) -> Fraction {
        // 整数の場合はそのまま返す
        if self.denominator.is_one() {
            return self.clone();
        }

        let q = &self.numerator / &self.denominator;
        let r = &self.numerator % &self.denominator;

        // 正の数で余りがある場合は、1を加える（正の無限大方向）
        let ceiled = if self.numerator > BigInt::zero() && !r.is_zero() {
            q + BigInt::one()
        } else if self.numerator < BigInt::zero() && !r.is_zero() {
            // 負の数の場合、商は既にゼロ方向に切り捨てられているのでそのまま
            q
        } else {
            q
        };

        Fraction {
            numerator: ceiled,
            denominator: BigInt::one(),
        }
    }

    /// 四捨五入（0.5は0から遠い方向へ: round half away from zero）
    /// 整数の場合は自身をそのまま返す（高速化）
    ///
    /// 正の数: floor(x + 0.5)
    /// 負の数: -floor(|x| + 0.5)
    pub fn round(&self) -> Fraction {
        // 整数の場合はそのまま返す
        if self.denominator.is_one() {
            return self.clone();
        }

        if self.numerator.is_zero() {
            return Fraction {
                numerator: BigInt::zero(),
                denominator: BigInt::one(),
            };
        }

        // Round half away from zero: |x| + 0.5 を floor して符号を戻す
        // |x| + 1/2 = (2|num| + den) / (2 * den)
        let is_negative = self.numerator < BigInt::zero();
        let abs_num = if is_negative {
            -&self.numerator
        } else {
            self.numerator.clone()
        };

        // floor(|x| + 0.5) = (2*|num| + den) / (2*den) (integer division)
        let two = BigInt::from(2);
        let two_abs_num = &abs_num * &two;
        let result = (&two_abs_num + &self.denominator) / (&two * &self.denominator);

        Fraction {
            numerator: if is_negative { -result } else { result },
            denominator: BigInt::one(),
        }
    }

    /// 整数かどうかを判定
    pub fn is_exact_integer(&self) -> bool {
        self.denominator == BigInt::one()
    }

    /// 非負整数としてusizeに変換（分母が1の場合のみ）
    pub fn as_usize(&self) -> Option<usize> {
        if self.is_exact_integer() && self.numerator >= BigInt::zero() {
            self.numerator.to_usize()
        } else {
            None
        }
    }

    /// 剰余演算（数学的剰余: a mod b = a - b * floor(a/b)）
    /// 整数同士の場合はBigIntの剰余演算を直接使用（高速化）
    pub fn modulo(&self, other: &Fraction) -> Fraction {
        if other.numerator.is_zero() {
            panic!("Modulo by zero");
        }

        // 整数同士の場合: BigIntの剰余演算を直接使用
        if self.denominator.is_one() && other.denominator.is_one() {
            // 数学的剰余（常に非負）: ((a % b) + b) % b
            let rem = &self.numerator % &other.numerator;
            let result = if rem < BigInt::zero() {
                if other.numerator > BigInt::zero() {
                    rem + &other.numerator
                } else {
                    rem - &other.numerator
                }
            } else {
                rem
            };
            return Fraction {
                numerator: result,
                denominator: BigInt::one(),
            };
        }

        // 一般的な場合: a mod b = a - b * floor(a/b)
        let div_result = self.div(other);
        let floored = div_result.floor();
        self.sub(&other.mul(&floored))
    }
}

impl PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Fraction {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // 整数同士の場合は乗算をスキップ
        if self.denominator.is_one() && other.denominator.is_one() {
            return self.numerator.cmp(&other.numerator);
        }
        // Compare a/b with c/d using a*d vs b*c (integer comparison)
        let lhs = &self.numerator * &other.denominator;
        let rhs = &other.numerator * &self.denominator;
        lhs.cmp(&rhs)
    }
}

impl ToPrimitive for Fraction {
    fn to_i64(&self) -> Option<i64> {
        // Division result as i64
        (&self.numerator / &self.denominator).to_i64()
    }

    fn to_u64(&self) -> Option<u64> {
        if self.numerator < BigInt::zero() {
            None
        } else {
            (&self.numerator / &self.denominator).to_u64()
        }
    }

    fn to_f64(&self) -> Option<f64> {
        // Convert numerator and denominator to f64, then divide
        let num_f64 = self.numerator.to_f64()?;
        let den_f64 = self.denominator.to_f64()?;
        if den_f64 == 0.0 {
            None
        } else {
            Some(num_f64 / den_f64)
        }
    }
}
