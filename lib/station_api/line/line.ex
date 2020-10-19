defmodule StationApi.Line do
  alias StationApi.Common

  def line_by_id(id) do
    {:ok, result} =
      Ecto.Adapters.SQL.query(StationApi.Repo, "SELECT * FROM `lines` WHERE line_cd = ? AND e_status = 0", [
        id
      ])
    {:ok, Common.to_column_map(result.columns, result.rows)}
  end

  def lines_by_group_id(group_id) do
    {:ok, result} =
      Ecto.Adapters.SQL.query(StationApi.Repo, "SELECT * FROM `lines` WHERE line_cd IN (SELECT line_cd FROM stations WHERE station_g_cd = ?) AND e_status = 0", [
        group_id
      ])

    {:ok, Common.to_column_map(result.columns, result.rows)}
  end
end
