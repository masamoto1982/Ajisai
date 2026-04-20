![Rust](https://img.shields.io/badge/Rust-000000?style=flat&logo=rust&logoColor=white)
![WebAssembly](https://img.shields.io/badge/WebAssembly-654FF0?style=flat&logo=webassembly&logoColor=white)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat&logo=typescript&logoColor=white)
[![Build and Deploy Ajisai](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml/badge.svg)](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

![Ajisai Logo](public/images/ajisai-logo.png "Ajisai Programming Language Logo")
![Ajisai QR Code](public/images/ajisai-qr.png "Ajisai QR Code")

# Ajisai

Ajisaiはプログラミング言語における型システムの新しい可能性を追求するために生まれた。  

本言語は、型安全性の担保と丸め誤差の回避を目的に、すべての情報を分数として扱う。  
分数はVectorという器に対し水のように注がれ、その水面たる表示部には様々な波紋が浮かぶ。

現実の水がそうであるように、その水面をどれだけ掻き回したとしても、水は水のまま不変である。  
このコンセプトにより、Ajisaiは型安全でありながらも親しみやすい言語を目指す。


Ajisaiという名前は、「水の器」という意味の学名を持つ紫陽花に因んでいる。

Playground: https://masamoto1982.github.io/Ajisai/

Desktop (Tauri) build channel is available in the same repository (`src-tauri/`).

---

## 水のメタファー

### 水としての分数

Ajisaiの数はすべて分数として扱われ、近似や丸めは一切発生しない。  
水がどの器を通っても体積を失わないように、値は計算を通過しても厳密な形を保つ。

→ 技術的な詳細: [SPECIFICATION.md §4.2](SPECIFICATION.md#42-scalar-exact-rational-arithmetic)

### 器としての Vector

Vectorは、値を順序をもって収めるための器である。  
器は入れ子にすることができ、器の中にさらに器を置くことも可能である。  
しかしその本質は変わらず、Vectorは一貫して値を受け取り、保持し、渡すための構造として機能する。

→ 技術的な詳細: [SPECIFICATION.md §4.3](SPECIFICATION.md#43-vector)

### 器に対する水の注ぎ方としてのコードブロック

器があれば、それに注ぐ手段が必要となる。  
コードブロックは「どのように水を注ぐか」を記述する単位であり、順序・変換・操作の連鎖を表現する。  
注ぎ方そのものもまた器に収めることができ、別の注ぎ方へ渡すこともできる。

→ 技術的な詳細: [SPECIFICATION.md §4.6](SPECIFICATION.md#46-codeblock), [§8](SPECIFICATION.md#8-user-words)

### 水の流れを制御するモード

すべての操作は二つの軸によって制御される。

**操作対象モード** —— 水路のどこに作用するかを定める。水面の一点か、あるいは水路全体か。  
**消費モード** —— 流れが飲み込まれるか、それとも分流するかを定める。  
分流（`,,`）は流れを失うことがなく、源を残しつつ新たな流れを生み出す。

→ 技術的な詳細: [SPECIFICATION.md §6](SPECIFICATION.md#6-modifiers)

### 泡としての NIL

泡は水ではないが、水のある場所に現れる。  
NILは値の不在を表す——本来値があるべき場所に、何も存在しないときのかたちである。  
`~` を付与した操作は乱流を泡に変え、氾濫を未然に防ぐ。これにより上流は守られる。

→ 技術的な詳細: [SPECIFICATION.md §4.5](SPECIFICATION.md#45-nil), [§6.3](SPECIFICATION.md#63-safe-mode-modifier)

### 波紋としてのセマンティックプレーン

波紋は水ではないが、確かに水面に浮かび、値の姿を観る者に伝える。  
セマンティックプレーンは値に添えられる表示のヒントであり、同じ分数を数として見せるか、文字列として見せるか、日時として見せるかを決定する。  
波紋は水の体積を変えず、流れを乱さず、計算にも影響を与えない。それでも値を読み取る瞬間、そのかたちが見栄えを決める。

→ 技術的な詳細: [SPECIFICATION.md §5.2](SPECIFICATION.md#52-two-plane-architecture), [§12](SPECIFICATION.md#12-semantic-plane)

---

## Runtime

Rust interpreter core → WASM boundary → TypeScript GUI/runtime shell

- Web Playground channel: Vite build (`npm run build:web`) for GitHub Pages
- Desktop channel: Tauri wrapper (`npm run tauri:build`, frontend via `npm run build:tauri-frontend`)
- Runtime-specific behavior (Persistence / File I/O / Runtime hooks) is abstracted via `js/platform/` adapters

仕様の完全な定義: `SPECIFICATION.md`

---

## Development Checks

```sh
cd rust && cargo test --lib
cd rust && cargo test --tests
npm run check
```

GUI 動作テストはアプリ上の `Test` ボタンから `js/gui/gui-interpreter-test-cases.ts` のケースを実行して確認します。

---

## License

MIT (`LICENSE`)
