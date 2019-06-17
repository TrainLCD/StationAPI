defmodule StationApi.Common do
  def to_column_map(columns, rows) do
    Enum.map(rows, fn row -> Enum.into(List.zip([columns, row]), %{}) end)
  end
end
