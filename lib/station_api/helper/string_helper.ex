defmodule StringHelper do
    @moduledoc """
    文字列を扱う汎用的なヘルパー関数群。
    https://qiita.com/kuroda@github/items/b62ff5c4a14b6539e74e
    """
  
    @doc """
    与えられた文字列に含まれるひらがなをカタカナに変換して返す。
    """
    def hiragana2katakana(source) do
      source
      |> String.codepoints()
      |> Enum.map(&do_convert_h2k(&1))
      |> Enum.join()
    end
  
    defp do_convert_h2k(cp) when cp < "\u3041" or cp > "\u3096", do: cp
  
    defp do_convert_h2k(cp) do
      <<n::utf8>> = cp
      m = n + 0x60
      <<m::utf8>>
    end
  
    @doc """
    与えられた文字列に含まれるカタカナをひらがなに変換して返す。
    """
    def katakana2hiragana(source) do
      source
      |> String.codepoints()
      |> Enum.map(&do_convert_k2h(&1))
      |> Enum.join()
    end
  
    defp do_convert_k2h(cp) when cp < "\u30A1" or cp > "\u30F6", do: cp
  
    defp do_convert_k2h(cp) do
      <<n::utf8>> = cp
      m = n - 0x60
      <<m::utf8>>
    end
  end
  