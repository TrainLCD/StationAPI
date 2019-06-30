defmodule StationApiWeb.Resolvers.Station do
  import StationApiWeb.Resolvers.Common

  def station_by_coords(_parent, %{latitude: latitude, longitude: longitude}, _resolution) do
    {:ok, stations} = StationApi.Station.station_by_coords(latitude, longitude)

    station = List.first(stations)
    station_map = to_atomic_map(station)

    {:ok, station_group} = StationApi.Station.station_by_group_id(station_map[:station_g_cd])

    lines =
      Enum.map(station_group, fn s ->
        s_map = to_atomic_map(s)
        {:ok, lines} = StationApi.Line.line_by_id(s_map[:line_cd])
        line = List.first(lines)
        line_map = to_atomic_map(line)
        transform_line_result(line_map)
      end)

    api_result = transform_station_result(station_map, lines)

    {:ok, api_result}
  end

  def station_by_id(_parent, %{id: id}, _resolution) do
    {:ok, stations} = StationApi.Station.station_by_group_id(id)
    station = List.first(stations)

    case station do
      nil -> {:error, "Station not found."}
      station ->
        station_map = to_atomic_map(station)

        lines =
          Enum.map(stations, fn s ->
            s_map = to_atomic_map(s)
            {:ok, lines} = StationApi.Line.line_by_id(s_map[:line_cd])
            line = List.first(lines)
            line_map = to_atomic_map(line)
            transform_line_result(line_map)
          end)

        api_result = transform_station_result(station_map, lines)

        {:ok, api_result}
    end
  end

  def stations_by_line_id(_parent, %{line_id: line_id}, _resolution) do
    {:ok, stations} = StationApi.Station.stations_by_line_id(line_id)

    case stations do
      [] -> {:error, "Not matched."}
      stations ->
        stations_lines =
          Enum.map(stations, fn s ->
            s_map = to_atomic_map(s)
            {:ok, group_stations} = StationApi.Station.station_by_group_id(s_map[:station_g_cd])
            Enum.map(group_stations, fn gs ->
              gs_map = to_atomic_map(gs)
              {:ok, lines} = StationApi.Line.line_by_id(gs_map[:line_cd])
              line = List.first(lines)
              line_map = to_atomic_map(line)
              transform_line_result(line_map)
            end)
          end)

        stations_with_index = stations |> Enum.with_index
        api_result = Enum.map(stations_with_index, fn {s, i} ->
          s_map = to_atomic_map(s)
          transform_station_result(s_map, Enum.at(stations_lines, i))
        end)

        {:ok, api_result}
    end
  end

  defp transform_station_result(map, lines) do
    %{
      id: map[:station_cd],
      group_id: map[:station_g_cd],
      name: map[:station_name],
      postal_code: map[:post],
      open_ymd: map[:open_ymd],
      latitude: map[:lat],
      longitude: map[:lon],
      distance: map[:distance],
      address: map[:add],
      lines: lines
    }
  end
end
