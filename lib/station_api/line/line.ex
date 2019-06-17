defmodule StationApi.Line do
  alias StationApi.Common

  def line_by_id(id) do
    {:ok, result} =
      Ecto.Adapters.SQL.query(StationApi.Repo, "SELECT * FROM `lines` WHERE line_cd = ?", [
        id
      ])
    {:ok, Common.to_column_map(result.columns, result.rows)}
  end
end
