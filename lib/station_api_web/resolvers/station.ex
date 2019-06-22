defmodule StationApiWeb.Resolvers.Station do
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
        lines =
          Enum.map(stations, fn s ->
            s_map = to_atomic_map(s)
            {:ok, lines} = StationApi.Line.line_by_id(s_map[:line_cd])
            line = List.first(lines)
            line_map = to_atomic_map(line)
            transform_line_result(line_map)
          end)

        station_maps = Enum.map(stations, fn s ->
          to_atomic_map(s)
        end)
        api_result = Enum.map(station_maps, fn s ->
          transform_station_result(s, lines)
        end)

        {:ok, api_result}
    end
  end

  defp transform_station_result(map, lines) do
    %{
      id: map[:station_cd],
      group_id: map[:station_g_cd],
      name: map[:station_name],
      name_r: map[:station_name_r],
      name_k: map[:station_name_k],
      postal_code: map[:post],
      open_ymd: map[:open_ymd],
      latitude: map[:lat],
      longitude: map[:lon],
      distance: map[:distance],
      address: map[:addr],
      lines: lines
    }
  end

  defp transform_line_result(map) do
    %{
      id: map[:line_cd],
      company_id: map[:company_cd],
      latitude: map[:lat],
      line_color_c: map[:line_color_c],
      line_color_t: map[:line_color_t],
      name: map[:line_name],
      name_h: map[:line_name_h],
      name_k: map[:line_name_k],
      line_type: map[:line_type],
      longitude: map[:longitude],
      zoom: map[:zoom]
    }
  end

  defp to_atomic_map(str_map) do
    str_map |> Map.new(fn {k, v} -> {String.to_atom(k), v} end)
  end
end
