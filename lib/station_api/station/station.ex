defmodule StationApi.Station do
  alias StationApi.Common

  def station_by_coords(latitude, longitude) do
    latitude_str = latitude |> to_string
    longitude_str = longitude |> to_string

    {:ok, result} =
      Ecto.Adapters.SQL.query(
        StationApi.Repo,
        "
    SELECT *,
    (
      6371 * acos(
      cos(radians(" <> latitude_str <> "))
      * cos(radians(lat))
      * cos(radians(lon) - radians(" <> longitude_str <> "))
      + sin(radians(" <> latitude_str <> "))
      * sin(radians(lat))
      )
    ) AS distance
    FROM
    stations
    WHERE
    e_status = 0
    ORDER BY
    distance
    LIMIT 1",
        []
      )

    {:ok, Common.to_column_map(result.columns, result.rows)}
  end

  def station_by_group_id(group_id) do
    {:ok, result} =
      Ecto.Adapters.SQL.query(StationApi.Repo, "SELECT * FROM stations WHERE station_g_cd = ? AND e_status = 0", [
        group_id
      ])

    {:ok, Common.to_column_map(result.columns, result.rows)}
  end

  def stations_by_line_id(line_id) do
    {:ok, result} =
      Ecto.Adapters.SQL.query(
        StationApi.Repo,
        "SELECT * FROM stations WHERE line_cd = ? AND e_status = 0 ORDER BY e_sort, station_cd",
        [
          line_id
        ]
      )

    {:ok, Common.to_column_map(result.columns, result.rows)}
  end

  def stations_by_name(name) do
    {:ok, result} =
      Ecto.Adapters.SQL.query(
        StationApi.Repo,
        "SELECT * FROM stations WHERE (station_name LIKE '" <> name <>"%' OR station_name_r LIKE '"<> name <> "%') AND e_status = 0 ORDER BY e_sort, station_cd"
      )

    {:ok, Common.to_column_map(result.columns, result.rows)}
  end
end
