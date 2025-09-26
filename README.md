![Rust](https://img.shields.io/badge/Rust-E05555?style=flat-square)
![TypeScript](https://img.shields.io/badge/TypeScript-3178C6?style=flat&logo=typescript&logoColor=white)
![CSS3](https://img.shields.io/badge/CSS3-1572B6?style=flat-square)
![HTML5](https://img.shields.io/badge/HTML5-E34F26?style=flat-square)
[![Build and Deploy Ajisai](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml/badge.svg)](https://github.com/masamoto1982/Ajisai/actions/workflows/build.yml)

# Ajisai

### 開発コンセプト
- FORTHを参考にしたスタックベース、逆ポーランド記法
- システムは辞書に登録されたワード、真偽値、数値、文字列、Nilのみを認識する
- 唯一のデータ構造としてVectorを持つ
- Vectorは真偽値、文字列、Nilを含むことが可能で負のインデックスを指定することにより末尾検索可能
- Vector操作について、位置を指定する操作は0オリジン、量を指定する際は1オリジン
- 組み込みワードの削除、意味の上書きは不可
- すべての数値を内部的に分数扱いすることにより丸め誤差を生じない
- メモリーの使用状況や辞書の状態をGUIで表現
- 一行ごとに反復回数と処理時間の指定が可能
- カスタムワードを定義することで分岐が可能（分岐の仕組みはGOTO命令やケース式に似る）

（Ajisaiという名称は、小さなワードの集まりが機能をなすFORTHの特徴を、紫陽花の花のイメージになぞらえたもの。）※紫陽花の花の部分は実際には花ではない。

