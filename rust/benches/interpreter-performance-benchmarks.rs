use criterion::{criterion_group, criterion_main, Criterion, black_box};
use std::collections::HashMap;
use num_bigint::BigInt;
use num_traits::One;

use ajisai_core::types::fraction::Fraction;
use ajisai_core::elastic::ElasticMode;
use ajisai_core::interpreter::Interpreter;


fn build_ajisai_dictionary() -> HashMap<String, String> {
    let words = vec![
        "GET", "INSERT", "REPLACE", "REMOVE", "LENGTH", "TAKE", "SPLIT",
        "CONCAT", "REVERSE", "RANGE", "REORDER", "COLLECT", "SORT",
        "SHAPE", "RANK", "RESHAPE", "TRANSPOSE", "FILL",
        "FLOOR", "CEIL", "ROUND", "MOD",
        "+", "-", "*", "/", "=", "<", "<=",
        "AND", "OR", "NOT",
        "PRINT", "DEF", "DEL", "?",
        "MAP", "FILTER", "FOLD", "TIMES", "EXEC", "EVAL",
        "TRUE", "FALSE", "NIL",
        "STR", "NUM", "BOOL", "CHR", "CHARS", "JOIN",
        "NOW", "DATETIME", "TIMESTAMP", "CSPRNG", "HASH",
        "SEQ", "SIM", "SLOT", "GAIN", "GAIN-RESET",
        "PAN", "PAN-RESET", "FX-RESET", "PLAY", "CHORD", "ADSR",
        "SINE", "SQUARE", "SAW", "TRI",
        "PARSE", "STRINGIFY", "INPUT", "OUTPUT",
        "JSON-GET", "JSON-KEYS", "JSON-SET", "JSON-EXPORT",
        "!",
    ];
    let mut map = HashMap::new();
    for w in &words {
        map.insert(w.to_string(), w.to_string());
    }
    map
}

fn bench_hashmap_lookup_hit(c: &mut Criterion) {
    let dict = build_ajisai_dictionary();
    let keys = vec!["GET", "+", "MAP", "JSON-GET", "FOLD", "TRANSPOSE", "CSPRNG"];

    c.bench_function("hashmap_lookup_hit", |b| {
        b.iter(|| {
            for key in &keys {
                black_box(dict.get(*key));
            }
        })
    });
}

fn bench_hashmap_lookup_miss(c: &mut Criterion) {
    let dict = build_ajisai_dictionary();
    let keys = vec!["UNKNOWN", "FOOBAR", "MY-WORD", "CUSTOM123"];

    c.bench_function("hashmap_lookup_miss", |b| {
        b.iter(|| {
            for key in &keys {
                black_box(dict.get(*key));
            }
        })
    });
}


fn bench_fraction_new_small_integers(c: &mut Criterion) {
    c.bench_function("fraction_new_small_integers", |b| {
        b.iter(|| {
            black_box(Fraction::new(BigInt::from(42), BigInt::one()));
        })
    });
}

fn bench_fraction_new_needs_gcd(c: &mut Criterion) {
    c.bench_function("fraction_new_needs_gcd", |b| {
        b.iter(|| {
            black_box(Fraction::new(BigInt::from(355), BigInt::from(113)));
        })
    });
}

fn bench_fraction_new_large_gcd(c: &mut Criterion) {
    let num = BigInt::from(1_000_000_007i64) * BigInt::from(6i64);
    let den = BigInt::from(1_000_000_009i64) * BigInt::from(6i64);
    c.bench_function("fraction_new_large_gcd", |b| {
        b.iter(|| {
            black_box(Fraction::new(num.clone(), den.clone()));
        })
    });
}


fn bench_fraction_add_i64_path(c: &mut Criterion) {
    let a = Fraction::new(BigInt::from(3), BigInt::from(7));
    let b = Fraction::new(BigInt::from(5), BigInt::from(11));

    c.bench_function("fraction_add_i64_path", |b_iter| {
        b_iter.iter(|| {
            black_box(a.add(&b));
        })
    });
}

fn bench_fraction_add_bigint_path(c: &mut Criterion) {
    let big = BigInt::from(i64::MAX) * BigInt::from(2i64);
    let a = Fraction::new(big.clone(), BigInt::from(7));
    let b = Fraction::new(big.clone(), BigInt::from(11));

    c.bench_function("fraction_add_bigint_path", |b_iter| {
        b_iter.iter(|| {
            black_box(a.add(&b));
        })
    });
}

fn bench_fraction_mul_i64_path(c: &mut Criterion) {
    let a = Fraction::new(BigInt::from(3), BigInt::from(7));
    let b = Fraction::new(BigInt::from(5), BigInt::from(11));

    c.bench_function("fraction_mul_i64_path", |b_iter| {
        b_iter.iter(|| {
            black_box(a.mul(&b));
        })
    });
}

fn bench_fraction_mul_bigint_path(c: &mut Criterion) {
    let big = BigInt::from(i64::MAX) * BigInt::from(2i64);
    let a = Fraction::new(big.clone(), BigInt::from(7));
    let b = Fraction::new(big.clone(), BigInt::from(11));

    c.bench_function("fraction_mul_bigint_path", |b_iter| {
        b_iter.iter(|| {
            black_box(a.mul(&b));
        })
    });
}

fn bench_fraction_add_integers(c: &mut Criterion) {
    let a = Fraction::from(42i64);
    let b = Fraction::from(58i64);

    c.bench_function("fraction_add_integers", |b_iter| {
        b_iter.iter(|| {
            black_box(a.add(&b));
        })
    });
}

fn bench_fraction_modulo(c: &mut Criterion) {
    let a = Fraction::new(BigInt::from(7), BigInt::from(3));
    let b = Fraction::new(BigInt::from(5), BigInt::from(4));

    c.bench_function("fraction_modulo", |b_iter| {
        b_iter.iter(|| {
            black_box(a.modulo(&b));
        })
    });
}

fn bench_fraction_comparison(c: &mut Criterion) {
    let a = Fraction::new(BigInt::from(355), BigInt::from(113));
    let b = Fraction::new(BigInt::from(22), BigInt::from(7));

    c.bench_function("fraction_comparison_lt", |b_iter| {
        b_iter.iter(|| {
            black_box(a.lt(&b));
        })
    });
}

fn bench_fraction_eq_i64(c: &mut Criterion) {
    let a = Fraction::from(42i64);
    let b = Fraction::from(42i64);

    c.bench_function("fraction_eq_i64", |b_iter| {
        b_iter.iter(|| {
            black_box(a == b);
        })
    });
}

fn bench_fraction_lt_i64(c: &mut Criterion) {
    let a = Fraction::from(42i64);
    let b = Fraction::from(58i64);

    c.bench_function("fraction_lt_i64", |b_iter| {
        b_iter.iter(|| {
            black_box(a.lt(&b));
        })
    });
}

fn bench_fraction_eq_fraction(c: &mut Criterion) {
    let a = Fraction::new(BigInt::from(355), BigInt::from(113));
    let b = Fraction::new(BigInt::from(355), BigInt::from(113));

    c.bench_function("fraction_eq_fraction", |b_iter| {
        b_iter.iter(|| {
            black_box(a == b);
        })
    });
}


fn bench_interpreter_simple_arithmetic(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("interp_simple_arithmetic", |b| {
        b.iter(|| {
            let mut interp = Interpreter::new();
            rt.block_on(interp.execute("[ 1 2 3 ] [ 4 5 6 ] +")).unwrap();
            black_box(&interp);
        })
    });
}

fn bench_interpreter_map(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let code = "[ 1 2 3 4 5 6 7 8 9 10 ] : [ 2 ] * ; MAP";

    c.bench_function("interp_map", |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();
            let mut last_interp = None;
            for _ in 0..iters {
                let mut interp = Interpreter::new();
                rt.block_on(interp.execute(code)).unwrap();
                black_box(interp.get_stack().clone());
                last_interp = Some(interp);
            }
            if let Some(interp) = last_interp {
                let m = interp.runtime_metrics();
                eprintln!(
                    "[bench metrics] plan_hit={} miss={} quant_use={}",
                    m.compiled_plan_cache_hit_count,
                    m.compiled_plan_cache_miss_count,
                    m.quantized_block_use_count
                );
            }
            start.elapsed()
        })
    });
}

fn bench_interpreter_fold(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let code = "[ 1 2 3 4 5 6 7 8 9 10 ] [ 0 ] : + ; FOLD";

    c.bench_function("interp_fold", |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();
            let mut last_interp = None;
            for _ in 0..iters {
                let mut interp = Interpreter::new();
                rt.block_on(interp.execute(code)).unwrap();
                black_box(interp.get_stack().clone());
                last_interp = Some(interp);
            }
            if let Some(interp) = last_interp {
                let m = interp.runtime_metrics();
                eprintln!(
                    "[bench metrics] plan_hit={} miss={} quant_use={}",
                    m.compiled_plan_cache_hit_count,
                    m.compiled_plan_cache_miss_count,
                    m.quantized_block_use_count
                );
            }
            start.elapsed()
        })
    });
}

fn bench_interpreter_sort(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("interp_sort", |b| {
        b.iter(|| {
            let mut interp = Interpreter::new();
            rt.block_on(interp.execute("[ 5 3 8 1 9 2 7 4 10 6 ] SORT")).unwrap();
            black_box(&interp);
        })
    });
}

fn bench_interpreter_word_lookup_overhead(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("interp_many_word_lookups", |b| {
        b.iter(|| {
            let mut interp = Interpreter::new();
            rt.block_on(interp.execute(
                "[ 3 1 2 ] SORT == ,, LENGTH"
            )).unwrap();
            black_box(&interp);
        })
    });
}

fn bench_interpreter_custom_word(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let code = ": [ 2 ] * ; 'DOUBLE' DEF [ 1 2 3 4 5 ] : DOUBLE ; MAP";

    c.bench_function("interp_custom_word", |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();
            let mut last_interp = None;
            for _ in 0..iters {
                let mut interp = Interpreter::new();
                rt.block_on(interp.execute(code)).unwrap();
                black_box(interp.get_stack().clone());
                last_interp = Some(interp);
            }
            if let Some(interp) = last_interp {
                let m = interp.runtime_metrics();
                eprintln!(
                    "[bench metrics] plan_hit={} miss={} quant_use={}",
                    m.compiled_plan_cache_hit_count,
                    m.compiled_plan_cache_miss_count,
                    m.quantized_block_use_count
                );
            }
            start.elapsed()
        })
    });
}

fn bench_interpreter_vector_construction(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("interp_vector_construction", |b| {
        b.iter(|| {
            let mut interp = Interpreter::new();
            rt.block_on(interp.execute("[ 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17 18 19 20 ]")).unwrap();
            black_box(&interp);
        })
    });
}

fn bench_interpreter_fraction_heavy(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    c.bench_function("interp_fraction_heavy", |b| {
        b.iter(|| {
            let mut interp = Interpreter::new();
            rt.block_on(interp.execute(
                "[ 1/3 2/7 5/11 3/13 7/17 ] [ 1/2 3/5 4/9 2/3 1/7 ] *"
            )).unwrap();
            black_box(&interp);
        })
    });
}

fn bench_interpreter_init_only(c: &mut Criterion) {
    c.bench_function("interp_init_only", |b| {
        b.iter(|| {
            black_box(Interpreter::new());
        })
    });
}

fn bench_interpreter_reuse(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut interp = Interpreter::new();

    c.bench_function("interp_reuse_add", |b| {
        b.iter(|| {
            interp.update_stack(Vec::new());
            rt.block_on(interp.execute("[ 1 2 3 ] [ 4 5 6 ] +")).unwrap();
            black_box(&interp);
        })
    });
}

fn bench_interpreter_hof_mode_matrix(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let scenarios: [(&str, &str, ElasticMode); 13] = [
        ("map_arith_greedy", "[ 1 2 3 4 5 6 7 8 9 10 ] { [ 2 ] * } MAP", ElasticMode::Greedy),
        ("map_arith_hedged", "[ 1 2 3 4 5 6 7 8 9 10 ] { [ 2 ] * } MAP", ElasticMode::HedgedSafe),
        ("map_arith_fast_guarded", "[ 1 2 3 4 5 6 7 8 9 10 ] { [ 2 ] * } MAP", ElasticMode::FastGuarded),
        ("map_predicate_greedy", "[ 1 2 3 4 5 6 7 8 9 10 ] { [ 5 ] < } MAP", ElasticMode::Greedy),
        ("map_predicate_fast_guarded", "[ 1 2 3 4 5 6 7 8 9 10 ] { [ 5 ] < } MAP", ElasticMode::FastGuarded),
        ("filter_greedy", "[ -5 -4 -3 -2 -1 0 1 2 3 4 5 ] { [ 0 ] <= NOT } FILTER", ElasticMode::Greedy),
        ("filter_fast_guarded", "[ -5 -4 -3 -2 -1 0 1 2 3 4 5 ] { [ 0 ] <= NOT } FILTER", ElasticMode::FastGuarded),
        ("fold_greedy", "[ 1 2 3 4 5 6 7 8 9 10 ] [ 0 ] { + } FOLD", ElasticMode::Greedy),
        ("fold_fast_guarded", "[ 1 2 3 4 5 6 7 8 9 10 ] [ 0 ] { + } FOLD", ElasticMode::FastGuarded),
        ("scan_greedy", "[ 1 2 3 4 5 6 7 8 9 10 ] [ 0 ] { + } SCAN", ElasticMode::Greedy),
        ("scan_fast_guarded", "[ 1 2 3 4 5 6 7 8 9 10 ] [ 0 ] { + } SCAN", ElasticMode::FastGuarded),
        ("epoch_change_hedged", "{ [2] * } 'DBL' DEF [1 2 3 4] 'DBL' MAP { [3] * } 'DBL' DEF [1 2 3 4] 'DBL' MAP", ElasticMode::HedgedSafe),
        ("epoch_change_fast_guarded", "{ [2] * } 'DBL' DEF [1 2 3 4] 'DBL' MAP { [3] * } 'DBL' DEF [1 2 3 4] 'DBL' MAP", ElasticMode::FastGuarded),
    ];

    c.bench_function("interp_hof_mode_matrix", |b| {
        b.iter_custom(|iters| {
            let start = std::time::Instant::now();
            let mut last_metrics = None;
            for _ in 0..iters {
                for (_name, code, mode) in &scenarios {
                    let mut interp = Interpreter::new();
                    interp.set_elastic_mode(*mode);
                    rt.block_on(interp.execute(code)).unwrap();
                    black_box(interp.get_stack().clone());
                    last_metrics = Some(interp.runtime_metrics());
                }
            }
            if let Some(m) = last_metrics {
                eprintln!(
                    "[bench metrics] quant_use={} hedged_started={} winner_q={} winner_plain={} fallback={} reject={}",
                    m.quantized_block_use_count,
                    m.hedged_race_started_count,
                    m.hedged_race_winner_quantized_count,
                    m.hedged_race_winner_plain_count,
                    m.hedged_race_fallback_count,
                    m.hedged_race_validation_reject_count
                );
            }
            start.elapsed()
        })
    });
}


const TRIE_ALPHABET_SIZE: usize = 40;

struct TrieNode {
    children: Box<[Option<Box<TrieNode>>; TRIE_ALPHABET_SIZE]>,
    value: Option<usize>,
}

impl TrieNode {
    fn new() -> Self {
        TrieNode {
            children: Box::new(std::array::from_fn(|_| None)),
            value: None,
        }
    }

    fn char_to_index(c: char) -> Option<usize> {
        match c {
            'A'..='Z' => Some((c as usize) - ('A' as usize)),
            '-' => Some(26),
            '_' => Some(27),
            '!' => Some(28),
            '~' => Some(29),
            '?' => Some(30),
            '=' => Some(31),
            '<' => Some(32),
            '>' => Some(33),
            '.' => Some(34),
            '+' => Some(35),
            '*' => Some(36),
            '/' => Some(37),
            ',' => Some(38),
            ';' => Some(39),
            _ => None,
        }
    }

    fn insert(&mut self, key: &str, val: usize) {
        let mut node = self;
        for c in key.chars() {
            let idx = Self::char_to_index(c).unwrap();
            node = node.children[idx].get_or_insert_with(|| Box::new(TrieNode::new()));
        }
        node.value = Some(val);
    }

    fn get(&self, key: &str) -> Option<usize> {
        let mut node = self;
        for c in key.chars() {
            let idx = Self::char_to_index(c)?;
            match &node.children[idx] {
                Some(child) => node = child,
                None => return None,
            }
        }
        node.value
    }
}

impl Default for TrieNode {
    fn default() -> Self {
        Self::new()
    }
}

fn build_trie_dictionary() -> TrieNode {
    let words = vec![
        "GET", "INSERT", "REPLACE", "REMOVE", "LENGTH", "TAKE", "SPLIT",
        "CONCAT", "REVERSE", "RANGE", "REORDER", "COLLECT", "SORT",
        "SHAPE", "RANK", "RESHAPE", "TRANSPOSE", "FILL",
        "FLOOR", "CEIL", "ROUND", "MOD",
        "+", "-", "*", "/", "=", "<", "<=",
        "AND", "OR", "NOT",
        "PRINT", "DEF", "DEL", "?",
        "MAP", "FILTER", "FOLD", "TIMES", "EXEC", "EVAL",
        "TRUE", "FALSE", "NIL",
        "STR", "NUM", "BOOL", "CHR", "CHARS", "JOIN",
        "CSPRNG", "HASH",
        "SEQ", "SIM", "SLOT", "GAIN", "GAIN-RESET",
        "PAN", "PAN-RESET", "FX-RESET", "PLAY", "CHORD", "ADSR",
        "SINE", "SQUARE", "SAW", "TRI",
        "PARSE", "STRINGIFY", "INPUT", "OUTPUT",
        "JSON-GET", "JSON-KEYS", "JSON-SET", "JSON-EXPORT",
        "!",
    ];
    let mut trie = TrieNode::new();
    for (i, w) in words.iter().enumerate() {
        trie.insert(w, i);
    }
    trie
}

fn bench_trie_lookup_hit(c: &mut Criterion) {
    let trie = build_trie_dictionary();
    let keys = vec!["GET", "+", "MAP", "JSON-GET", "FOLD", "TRANSPOSE", "CSPRNG"];

    c.bench_function("trie_lookup_hit", |b| {
        b.iter(|| {
            for key in &keys {
                black_box(trie.get(key));
            }
        })
    });
}

fn bench_trie_lookup_miss(c: &mut Criterion) {
    let trie = build_trie_dictionary();
    let keys = vec!["UNKNOWN", "FOOBAR", "MY-WORD", "CUSTOM"];

    c.bench_function("trie_lookup_miss", |b| {
        b.iter(|| {
            for key in &keys {
                black_box(trie.get(key));
            }
        })
    });
}


criterion_group!(
    dictionary_benches,
    bench_hashmap_lookup_hit,
    bench_hashmap_lookup_miss,
    bench_trie_lookup_hit,
    bench_trie_lookup_miss,
);

criterion_group!(
    fraction_benches,
    bench_fraction_new_small_integers,
    bench_fraction_new_needs_gcd,
    bench_fraction_new_large_gcd,
    bench_fraction_add_i64_path,
    bench_fraction_add_bigint_path,
    bench_fraction_add_integers,
    bench_fraction_mul_i64_path,
    bench_fraction_mul_bigint_path,
    bench_fraction_modulo,
    bench_fraction_comparison,
    bench_fraction_eq_i64,
    bench_fraction_lt_i64,
    bench_fraction_eq_fraction,
);

criterion_group!(
    interpreter_benches,
    bench_interpreter_init_only,
    bench_interpreter_simple_arithmetic,
    bench_interpreter_reuse,
    bench_interpreter_map,
    bench_interpreter_fold,
    bench_interpreter_sort,
    bench_interpreter_word_lookup_overhead,
    bench_interpreter_custom_word,
    bench_interpreter_vector_construction,
    bench_interpreter_fraction_heavy,
    bench_interpreter_hof_mode_matrix,
);

criterion_main!(dictionary_benches, fraction_benches, interpreter_benches);
