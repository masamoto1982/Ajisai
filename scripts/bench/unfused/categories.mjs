// Function → factor for the committed bundle at 7edd4c0 (function indices are
// specific to that build). Evidence: "loc" = rustc panic Location in the body
// names the source file; "alloc-chain" = reached from the __wbindgen_* allocator
// exports; "shape" = body only frees/recurses (drop glue) or was read with
// disasm.mjs; "callee" = inferred from what it calls / who calls it.
export const CATEGORY = {
  alloc:    { label: "確保・解放 (dlmalloc)", fns: { 1147: "alloc-chain __rust_alloc", 97: "alloc-chain malloc", 604: "alloc-chain", 709: "alloc-chain", 658: "alloc-chain", 556: "alloc-chain", 987: "alloc-chain __rust_dealloc", 375: "alloc-chain free (dlmalloc.rs)", 321: "alloc-chain __rust_realloc" } },
  drop:     { label: "drop glue（値の破棄・参照カウント減算）", fns: { 611: "shape recursive drop", 882: "shape", 1039: "shape", 571: "shape", 1040: "shape", 790: "shape", 876: "shape", 822: "shape", 879: "shape", 937: "shape", 976: "shape", 702: "shape", 522: "shape", 910: "shape", 1062: "shape" } },
  valueio:  { label: "値の生成・Vector⇔値の変換", fns: { 809: "shape Value::from(Fraction)", 401: "loc tensor_storage.rs", 374: "loc tensor_storage/lanes.rs", 113: "loc btree+smallvec, recursive over the Vector", 165: "loc smallvec, caller of 113", 188: "loc btree, recursive", 626: "callee 762 btree map.rs", 477: "callee lanes 374", 1006: "loc sync.rs Arc", 823: "callee 809/710" } },
  allocwrap: { label: "確保を呼ぶ小関数（用途未特定: Vec/BigUint/値の構築のいずれか）", fns: { 710: "callee alloc only", 707: "callee alloc only", 693: "callee alloc only", 680: "callee alloc only", 445: "callee alloc only", 470: "callee alloc only" } },
  dispatch: { label: "命令ディスパッチ（ワード呼び出し機構）", fns: { 66: "loc segment.rs (compiled line runner)", 328: "loc execute_builtin (Core Word match)", 404: "loc quickened.rs", 174: "loc quickened.rs", 185: "loc declared_nil_contract.rs", 186: "callee sibling of 185", 146: "loc execution_loop.rs", 228: "loc execute_def.rs/execution_loop.rs", 964: "callee step/meter (36 call sites in 328)", 947: "callee of 964", 296: "callee (66)", 499: "callee (66/328)", 378: "callee type switch", 1034: "callee", 928: "callee", 72: "callee POW operand lift (call_indirect)", 90: "callee execution loop (calls 66/328/404/185)", 485: "shape FxHash SEED 0x517cc1b727220a95 (fast_hash.rs): name hashing for lookup", 433: "callee recursive (328)", 458: "callee recursive" } },
  stack:    { label: "スタック操作（関数として残っている部分のみ）", fns: { 664: "shape walks the stack's Vec<Value> from the fresh mark (40-byte stride); once per Word" } },
  arith:    { label: "数値演算（有理数・bigint・POW）", fns: { 233: "loc exact/power.rs", 200: "loc exact/power.rs", 224: "loc math_ops.rs (POW)", 362: "loc fraction.rs", 281: "loc fraction.rs", 159: "loc small_rational.rs", 297: "loc num-bigint shift", 171: "loc num-bigint division", 180: "loc num-bigint shift", 399: "loc num-bigint division", 398: "loc num-bigint addition", 340: "callee 399", 668: "callee 180/340", 660: "callee 1060→171", 1060: "callee 171", 338: "callee (fraction)", 217: "callee 362/338 (fraction)", 451: "callee 523 (num-bigint)", 523: "callee 297 (num-bigint)", 688: "callee 421/500 (bigint build)", 421: "callee", 500: "callee", 262: "callee 659/1145", 659: "callee 1146→398", 1146: "callee 398", 381: "loc tensor_storage (bigint column)" } },
  tokenize: { label: "字句解析・文字列受け渡し", fns: { 82: "loc tokenizer.rs", 230: "loc tokenizer.rs", 448: "callee of 82", 459: "callee of 448", 740: "callee of 82", 752: "callee of 82", "passStringToWasm0": "JS glue" } },
  setupfused: { label: "（setup 由来の融合ループ・密カーネル）", fns: { 57: "loc fused_block*.rs", 122: "loc dense_kernels.rs" } },
};
export function categoryOf(fnName) {
  const m = /wasm-function\[(\d+)\]/.exec(fnName); const key = m ? Number(m[1]) : fnName;
  for (const [k, c] of Object.entries(CATEGORY)) if (key in c.fns) return k;
  return "unknown";
}
