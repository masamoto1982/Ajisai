# Ajisai

開発コンセプト

FORTHを参考にしたスタックベース、逆ポーランド記法
一つのスタックと一つのレジスタによる構成
唯一のデータ構造としてVectorを持つ（インデックスは0オリジン、負のインデックスで末尾からの検索が可能、NILを含むことができる）
組み込みワードの削除、意味の上書きは不可
カスタムワードAを含むカスタムワードBが存在するケースにおいて、カスタムワードAの削除や意味の上書きは不可
すべての数値を内部的に分数扱いすることにより丸め誤差を生じない
メモリーの使用状況や辞書の状態をGUIで表現

（Ajisaiという名称は、小さなワードの集まりが機能をなすFORTHの特徴を、紫陽花の花のイメージになぞらえたもの。）※紫陽花の花の部分は実際には花ではない。

