// test_tokenizer.rs - 空白区切りトークナイザーのテスト

#[cfg(test)]
mod test_tokenizer {
    use crate::tokenizer::tokenize;
    use crate::types::Token;

    // === コメント処理のテスト ===

    #[test]
    fn test_comment_basic() {
        let result = tokenize("1 2 # this is a comment").unwrap();
        assert_eq!(result, vec![
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
        ]);
    }

    #[test]
    fn test_comment_inline() {
        let result = tokenize("1 2 # comment\n3 4").unwrap();
        assert_eq!(result, vec![
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
            Token::LineBreak,
            Token::Number("3".to_string()),
            Token::Number("4".to_string()),
        ]);
    }

    #[test]
    fn test_comment_no_newline() {
        let result = tokenize("1 # comment").unwrap();
        assert_eq!(result, vec![
            Token::Number("1".to_string()),
        ]);
    }

    #[test]
    fn test_comment_adjacent_to_number() {
        // スペースなしで数値の直後にコメントが来るケース
        // #はis_special_char()でトークン境界となるため、正しく分離される
        let result = tokenize("123#comment").unwrap();
        assert_eq!(result, vec![
            Token::Number("123".to_string()),
        ]);
    }

    #[test]
    fn test_comment_adjacent_to_fraction() {
        // 分数リテラルの直後にコメントが来るケース
        // 統一分数アーキテクチャとコメントシステムの調和を確認
        let result = tokenize("1/3#これはコメント").unwrap();
        assert_eq!(result, vec![
            Token::Number("1/3".to_string()),
        ]);
    }

    #[test]
    fn test_comment_with_sharp_in_string() {
        let result = tokenize("'#not a comment' 1").unwrap();
        assert_eq!(result, vec![
            Token::String("#not a comment".to_string()),
            Token::Number("1".to_string()),
        ]);
    }

    #[test]
    fn test_multiple_comments() {
        let result = tokenize("# line 1\n# line 2\n1 2").unwrap();
        assert_eq!(result, vec![
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
        ]);
    }

    // === 引用文字列のテスト ===

    #[test]
    fn test_flexible_quotes_single_with_single_inside() {
        let result = tokenize("'He'llo'").unwrap();
        assert_eq!(result, vec![
            Token::String("He'llo".to_string()),
        ]);
    }

    #[test]
    fn test_flexible_quotes_single_with_double_inside() {
        let result = tokenize("'He\"llo'").unwrap();
        assert_eq!(result, vec![
            Token::String("He\"llo".to_string()),
        ]);
    }

    #[test]
    fn test_flexible_quotes_with_space_delimiter() {
        let result = tokenize("'Hello' 'World'").unwrap();
        assert_eq!(result, vec![
            Token::String("Hello".to_string()),
            Token::String("World".to_string()),
        ]);
    }

    #[test]
    fn test_flexible_quotes_with_bracket_delimiter() {
        let result = tokenize("['test']").unwrap();
        assert_eq!(result, vec![
            Token::VectorStart,
            Token::String("test".to_string()),
            Token::VectorEnd,
        ]);
    }

    // === 日本語ワードサポートのテスト（空白区切り） ===

    #[test]
    fn test_japanese_word_with_whitespace() {


        // 空白で区切られた日本語ワード
        let result = tokenize("2 3 足す").unwrap();
        assert_eq!(result, vec![
            Token::Number("2".to_string()),
            Token::Number("3".to_string()),
            Token::Symbol("足す".to_string()),
        ]);
    }

    #[test]
    fn test_japanese_word_boundary() {


        // 日本語ワードだけの入力
        let result = tokenize("足す").unwrap();
        assert_eq!(result, vec![
            Token::Symbol("足す".to_string()),
        ]);

        // 複数の日本語ワード（空白区切り）
        let result2 = tokenize("2 足す 3 掛ける 4").unwrap();
        assert_eq!(result2, vec![
            Token::Number("2".to_string()),
            Token::Symbol("足す".to_string()),
            Token::Number("3".to_string()),
            Token::Symbol("掛ける".to_string()),
            Token::Number("4".to_string()),
        ]);
    }

    #[test]
    fn test_mixed_japanese_english() {


        let result = tokenize("'Hello' 出力する").unwrap();
        assert_eq!(result, vec![
            Token::String("Hello".to_string()),
            Token::Symbol("出力する".to_string()),
        ]);
    }

    #[test]
    fn test_hiragana_katakana_kanji() {


        // ひらがな
        let result = tokenize("あいうえお").unwrap();
        assert_eq!(result, vec![
            Token::Symbol("あいうえお".to_string()),
        ]);

        // カタカナ
        let result2 = tokenize("アイウエオ").unwrap();
        assert_eq!(result2, vec![
            Token::Symbol("アイウエオ".to_string()),
        ]);

        // 漢字
        let result3 = tokenize("合計").unwrap();
        assert_eq!(result3, vec![
            Token::Symbol("合計".to_string()),
        ]);

        // 混在
        let result4 = tokenize("ひらがなカタカナ漢字").unwrap();
        assert_eq!(result4, vec![
            Token::Symbol("ひらがなカタカナ漢字".to_string()),
        ]);
    }

    #[test]
    fn test_japanese_with_operators() {


        // 演算子と日本語（空白で区切る）
        let result = tokenize("1 + 2 結果").unwrap();
        assert_eq!(result, vec![
            Token::Number("1".to_string()),
            Token::Symbol("+".to_string()),
            Token::Number("2".to_string()),
            Token::Symbol("結果".to_string()),
        ]);
    }

    // === 数値のテスト ===

    #[test]
    fn test_number_parsing() {


        // 整数
        let result = tokenize("123").unwrap();
        assert_eq!(result, vec![Token::Number("123".to_string())]);

        // 小数
        let result2 = tokenize("123.456").unwrap();
        assert_eq!(result2, vec![Token::Number("123.456".to_string())]);

        // 負の数
        let result3 = tokenize("-123").unwrap();
        assert_eq!(result3, vec![Token::Number("-123".to_string())]);

        // 科学的記数法
        let result4 = tokenize("1.5e10").unwrap();
        assert_eq!(result4, vec![Token::Number("1.5e10".to_string())]);
    }

    #[test]
    fn test_operator_symbols() {


        // + と - は単独で演算子
        let result = tokenize("+ -").unwrap();
        assert_eq!(result, vec![
            Token::Symbol("+".to_string()),
            Token::Symbol("-".to_string()),
        ]);

        // 数値と区別
        let result2 = tokenize("1 + 2 - 3").unwrap();
        assert_eq!(result2, vec![
            Token::Number("1".to_string()),
            Token::Symbol("+".to_string()),
            Token::Number("2".to_string()),
            Token::Symbol("-".to_string()),
            Token::Number("3".to_string()),
        ]);
    }

    // === キーワードのテスト ===
    // TRUE/FALSE/NIL は組み込みワードとして実装されるため、
    // トークナイザーでは Symbol として認識される

    #[test]
    fn test_keywords() {


        // TRUE, FALSE, NIL は Symbol として認識される
        let result = tokenize("TRUE FALSE NIL").unwrap();
        assert_eq!(result, vec![
            Token::Symbol("TRUE".to_string()),
            Token::Symbol("FALSE".to_string()),
            Token::Symbol("NIL".to_string()),
        ]);

        // 大文字小文字は保持される（インタープリタで大文字変換される）
        let result2 = tokenize("true false nil").unwrap();
        assert_eq!(result2, vec![
            Token::Symbol("true".to_string()),
            Token::Symbol("false".to_string()),
            Token::Symbol("nil".to_string()),
        ]);
    }

    // === ブラケットのテスト ===

    #[test]
    fn test_brackets() {


        // [] のテスト
        let result = tokenize("[ 1 2 3 ]").unwrap();
        assert_eq!(result, vec![
            Token::VectorStart,
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
            Token::Number("3".to_string()),
            Token::VectorEnd,
        ]);

        // {} も同等に扱われる
        let result2 = tokenize("{ a b c }").unwrap();
        assert_eq!(result2, vec![
            Token::VectorStart,
            Token::Symbol("a".to_string()),
            Token::Symbol("b".to_string()),
            Token::Symbol("c".to_string()),
            Token::VectorEnd,
        ]);

        // () も同等に扱われる
        let result3 = tokenize("( x y z )").unwrap();
        assert_eq!(result3, vec![
            Token::VectorStart,
            Token::Symbol("x".to_string()),
            Token::Symbol("y".to_string()),
            Token::Symbol("z".to_string()),
            Token::VectorEnd,
        ]);
    }

    #[test]
    fn test_mixed_bracket_styles() {


        // 異なる括弧スタイルの混在（FRAMEワードが生成する形式）
        let result = tokenize("{ ( [ 1 ] ) }").unwrap();
        assert_eq!(result, vec![
            Token::VectorStart,   // {
            Token::VectorStart,   // (
            Token::VectorStart,   // [
            Token::Number("1".to_string()),
            Token::VectorEnd,     // ]
            Token::VectorEnd,     // )
            Token::VectorEnd,     // }
        ]);

        // 2次元構造
        let result2 = tokenize("{ ( ) ( ) }").unwrap();
        assert_eq!(result2, vec![
            Token::VectorStart,   // {
            Token::VectorStart,   // (
            Token::VectorEnd,     // )
            Token::VectorStart,   // (
            Token::VectorEnd,     // )
            Token::VectorEnd,     // }
        ]);
    }

    #[test]
    fn test_frame_output_format() {


        // FRAMEワードが生成する3次元構造の形式
        // [ 1 2 3 ] FRAME → { ( [ ] [ ] [ ] ) ( [ ] [ ] [ ] ) }
        let frame_output = "{ ( [ ] [ ] [ ] ) ( [ ] [ ] [ ] ) }";
        let result = tokenize(frame_output).unwrap();

        // 正しくトークン化されることを確認
        assert!(result.iter().all(|t| matches!(t, Token::VectorStart | Token::VectorEnd)));

        // VectorStart と VectorEnd の数が一致
        let starts = result.iter().filter(|t| matches!(t, Token::VectorStart)).count();
        let ends = result.iter().filter(|t| matches!(t, Token::VectorEnd)).count();
        assert_eq!(starts, ends);
    }

    // === 複雑なパターンのテスト ===

    #[test]
    fn test_complex_expression() {
        let result = tokenize(
            "[ 1 2 3 ] LENGTH '結果' PRINT"
        ).unwrap();
        assert_eq!(result, vec![
            Token::VectorStart,
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
            Token::Number("3".to_string()),
            Token::VectorEnd,
            Token::Symbol("LENGTH".to_string()),
            Token::String("結果".to_string()),
            Token::Symbol("PRINT".to_string()),
        ]);
    }

    #[test]
    fn test_multiline_expression() {
        let result = tokenize(
            "1 2 +\n3 4 *\n"
        ).unwrap();
        assert_eq!(result, vec![
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
            Token::Symbol("+".to_string()),
            Token::LineBreak,
            Token::Number("3".to_string()),
            Token::Number("4".to_string()),
            Token::Symbol("*".to_string()),
        ]);
    }

    // === 空白のテスト ===

    #[test]
    fn test_whitespace_handling() {


        // タブやスペースは同じように扱われる
        let result = tokenize("1\t2  3   4").unwrap();
        assert_eq!(result, vec![
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
            Token::Number("3".to_string()),
            Token::Number("4".to_string()),
        ]);
    }

    // === エッジケースのテスト ===

    #[test]
    fn test_empty_input() {
        let result = tokenize("").unwrap();
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_only_whitespace() {
        let result = tokenize("   \n  \t  ").unwrap();
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_symbol_with_special_chars() {


        // FORTHスタイルのワード（?や!を含む）
        let result = tokenize("PRINT? SET!").unwrap();
        assert_eq!(result, vec![
            Token::Symbol("PRINT?".to_string()),
            Token::Symbol("SET!".to_string()),
        ]);
    }

    // === 分数リテラルのテスト ===

    #[test]
    fn test_fraction_literal() {


        // 基本的な分数
        let result = tokenize("1/3").unwrap();
        assert_eq!(result, vec![Token::Number("1/3".to_string())]);

        // 負の分数
        let result2 = tokenize("-1/3").unwrap();
        assert_eq!(result2, vec![Token::Number("-1/3".to_string())]);

        // 分数と他のトークン
        let result3 = tokenize("1/3 + 2/5").unwrap();
        assert_eq!(result3, vec![
            Token::Number("1/3".to_string()),
            Token::Symbol("+".to_string()),
            Token::Number("2/5".to_string()),
        ]);
    }

    #[test]
    fn test_fraction_in_vector() {


        let result = tokenize("[ 1/2 3/4 ]").unwrap();
        assert_eq!(result, vec![
            Token::VectorStart,
            Token::Number("1/2".to_string()),
            Token::Number("3/4".to_string()),
            Token::VectorEnd,
        ]);
    }

    #[test]
    fn test_invalid_fraction() {


        // "1/" は不完全なのでシンボルとして扱われる
        let result = tokenize("1/").unwrap();
        assert_eq!(result, vec![Token::Symbol("1/".to_string())]);

        // "1/a" もシンボル
        let result2 = tokenize("1/a").unwrap();
        assert_eq!(result2, vec![Token::Symbol("1/a".to_string())]);
    }

    // === 閉じられていない文字列のテスト ===

    #[test]
    fn test_unclosed_string_error() {


        let result = tokenize("'unclosed string");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Unclosed literal"));
    }

    // === Dot operator テスト ===

    #[test]
    fn test_dot_operator() {


        let result = tokenize(". + 3").unwrap();
        assert_eq!(result, vec![
            Token::Symbol(".".to_string()),
            Token::Symbol("+".to_string()),
            Token::Number("3".to_string()),
        ]);

        let result2 = tokenize(".. + 3").unwrap();
        assert_eq!(result2, vec![
            Token::Symbol("..".to_string()),
            Token::Symbol("+".to_string()),
            Token::Number("3".to_string()),
        ]);
    }

    #[test]
    fn test_dot_operator_with_vector() {


        let result = tokenize("[ 1 2 3 ] . LENGTH").unwrap();
        assert_eq!(result, vec![
            Token::VectorStart,
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
            Token::Number("3".to_string()),
            Token::VectorEnd,
            Token::Symbol(".".to_string()),
            Token::Symbol("LENGTH".to_string()),
        ]);

        let result2 = tokenize("a b c [ 1 ] .. GET").unwrap();
        assert_eq!(result2, vec![
            Token::Symbol("a".to_string()),
            Token::Symbol("b".to_string()),
            Token::Symbol("c".to_string()),
            Token::VectorStart,
            Token::Number("1".to_string()),
            Token::VectorEnd,
            Token::Symbol("..".to_string()),
            Token::Symbol("GET".to_string()),
        ]);
    }

    // === 空白なしブラケットのテスト ===

    #[test]
    fn test_bracket_without_space() {


        // 空白なしの基本ケース
        let result = tokenize("[1]").unwrap();
        assert_eq!(result, vec![
            Token::VectorStart,
            Token::Number("1".to_string()),
            Token::VectorEnd,
        ]);

        // 複数要素
        let result2 = tokenize("[1 2 3]").unwrap();
        assert_eq!(result2, vec![
            Token::VectorStart,
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
            Token::Number("3".to_string()),
            Token::VectorEnd,
        ]);

        // ネストされた構造
        let result3 = tokenize("[[1][2]]").unwrap();
        assert_eq!(result3, vec![
            Token::VectorStart,
            Token::VectorStart,
            Token::Number("1".to_string()),
            Token::VectorEnd,
            Token::VectorStart,
            Token::Number("2".to_string()),
            Token::VectorEnd,
            Token::VectorEnd,
        ]);

        // ワードとの組み合わせ
        let result4 = tokenize("[1 2]+[3 4]").unwrap();
        assert_eq!(result4, vec![
            Token::VectorStart,
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
            Token::VectorEnd,
            Token::Symbol("+".to_string()),
            Token::VectorStart,
            Token::Number("3".to_string()),
            Token::Number("4".to_string()),
            Token::VectorEnd,
        ]);
    }

    // === 文字列リテラルのテスト ===

    #[test]
    fn test_string_with_double_quote() {


        // 文字列内にダブルクォートを含む
        let result = tokenize("'He said \"Hello\"'").unwrap();
        assert_eq!(result, vec![
            Token::String("He said \"Hello\"".to_string()),
        ]);
    }

    #[test]
    fn test_string_with_single_quote() {


        // 文字列内にシングルクォートを含む
        let result = tokenize("'It's fine'").unwrap();
        assert_eq!(result, vec![
            Token::String("It's fine".to_string()),
        ]);
    }

    // === Vector Duality - Vectorをコードとして使用するテスト ===

    #[test]
    fn test_vector_as_code_syntax() {


        // Vectorをコードとして記述（新構文）
        let result = tokenize("[ [ 1 ] + ]").unwrap();
        // VectorStart, VectorStart, Number, VectorEnd, Symbol, VectorEnd
        assert_eq!(result.len(), 6);
        assert!(matches!(&result[0], Token::VectorStart));
        assert!(matches!(&result[1], Token::VectorStart));
        assert!(matches!(&result[2], Token::Number(n) if n == "1"));
        assert!(matches!(&result[3], Token::VectorEnd));
        assert!(matches!(&result[4], Token::Symbol(s) if s == "+"));
        assert!(matches!(&result[5], Token::VectorEnd));
    }

    #[test]
    fn test_def_with_vector_code() {


        // DEFでVectorをコードとして使用
        let result = tokenize("[ [ 2 ] * ] 'DOUBLE' DEF").unwrap();
        // VectorStart, VectorStart, Number, VectorEnd, Symbol, VectorEnd, String, Symbol
        assert_eq!(result.len(), 8);
        assert!(matches!(&result[6], Token::String(s) if s == "DOUBLE"));
        assert!(matches!(&result[7], Token::Symbol(s) if s == "DEF"));
    }

    // === シェブロン分岐トークンのテスト ===

    #[test]
    fn test_chevron_branch_token() {


        // >> トークン
        let result = tokenize(">> [ 5 ] [ 3 ] <").unwrap();
        assert_eq!(result[0], Token::ChevronBranch);
    }

    #[test]
    fn test_chevron_default_token() {


        // >>> トークン
        let result = tokenize(">>> [ 0 ]").unwrap();
        assert_eq!(result[0], Token::ChevronDefault);
    }

    #[test]
    fn test_chevron_structure() {


        // 複数行のシェブロン構造
        // ">> [ 5 ] [ 3 ] <\n>> [ 100 ]\n>>> [ 0 ]"
        // Tokens: ChevronBranch, VectorStart, Number(5), VectorEnd, VectorStart, Number(3), VectorEnd, Symbol(<), LineBreak,
        //         ChevronBranch, VectorStart, Number(100), VectorEnd, LineBreak,
        //         ChevronDefault, VectorStart, Number(0), VectorEnd
        let result = tokenize(">> [ 5 ] [ 3 ] <\n>> [ 100 ]\n>>> [ 0 ]").unwrap();
        assert!(matches!(&result[0], Token::ChevronBranch));       // index 0: >>
        assert!(matches!(&result[8], Token::LineBreak));           // index 8: \n (after <)
        assert!(matches!(&result[9], Token::ChevronBranch));       // index 9: >>
        assert!(matches!(&result[13], Token::LineBreak));          // index 13: \n (after ])
        assert!(matches!(&result[14], Token::ChevronDefault));     // index 14: >>>
    }

    // === コードブロックトークンのテスト ===

    #[test]
    fn test_code_block_tokens() {


        // : と ; トークン
        let result = tokenize(": [ 2 ] * ;").unwrap();
        assert_eq!(result[0], Token::CodeBlockStart);
        assert_eq!(result[result.len()-1], Token::CodeBlockEnd);
    }

    #[test]
    fn test_code_block_def_syntax() {


        // 新しいDEF構文
        let result = tokenize(": [ 2 ] * ; 'DOUBLE' DEF").unwrap();
        // CodeBlockStart, VectorStart, Number, VectorEnd, Symbol, CodeBlockEnd, String, Symbol
        assert_eq!(result[0], Token::CodeBlockStart);
        assert_eq!(result[5], Token::CodeBlockEnd);
        assert!(matches!(&result[6], Token::String(s) if s == "DOUBLE"));
        assert!(matches!(&result[7], Token::Symbol(s) if s == "DEF"));
    }

    // === 廃止された演算子のエラーテスト ===

    #[test]
    fn test_greater_than_error() {


        // 単独の > はエラー
        let result = tokenize("[ 5 ] [ 3 ] >");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("removed"));
    }

    #[test]
    fn test_greater_than_equal_error() {


        // >= はエラー
        let result = tokenize("[ 5 ] [ 3 ] >=");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("removed"));
    }

    #[test]
    fn test_multiline_code_block_with_chevrons() {


        // マルチラインのコードブロック
        let input = r#":
>> [ 1 ] =
>> [ 10 ]
>>> [ 20 ]
; 'CHECK_ONE' DEF"#;

        let result = tokenize(input).unwrap();

        // Print tokens for debugging
        for (i, t) in result.iter().enumerate() {
            println!("[{}] {:?}", i, t);
        }

        // Verify key tokens
        assert_eq!(result[0], Token::CodeBlockStart); // :

        // Find CodeBlockEnd
        let code_block_end_index = result.iter().position(|t| matches!(t, Token::CodeBlockEnd));
        assert!(code_block_end_index.is_some(), "CodeBlockEnd should exist in tokens");
        println!("CodeBlockEnd at index: {:?}", code_block_end_index);
    }
}
