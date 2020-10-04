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

  def station_by_criteria(_parent, args, _resolution) do
    {:ok, stations} = StationApi.Station.station_by_criteria(args)
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
  def stations_by_name(_parent, %{name: name}, _resolution) do
    {:ok, stations} = StationApi.Station.stations_by_name(name)

    case stations do
      [] -> {:error, "Not matched."}
      stations ->
        stations_lines =
          Enum.map(stations, fn s ->
            s_map = to_atomic_map(s)
            {:ok, lines} = StationApi.Line.lines_by_group_id(s_map[:station_g_cd])
            Enum.map(lines, fn line ->
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

  def stations_by_line_id(_parent, %{line_id: line_id}, _resolution) do
    {:ok, stations} = StationApi.Station.stations_by_line_id(line_id)

    case stations do
      [] -> {:error, "Not matched."}
      stations ->
        stations_lines =
          Enum.map(stations, fn s ->
            s_map = to_atomic_map(s)
            {:ok, lines} = StationApi.Line.lines_by_group_id(s_map[:station_g_cd])
            Enum.map(lines, fn line ->
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
      name_k: map[:station_name_k],
      name_r: map[:station_name_r],
      postal_code: map[:post],
      pref_id: map[:pref_cd],
      open_ymd: map[:open_ymd],
      latitude: Decimal.to_float(map[:lat]),
      longitude: Decimal.to_float(map[:lon]),
      distance: map[:distance],
      address: map[:address],
      lines: lines
    }
  end
end
